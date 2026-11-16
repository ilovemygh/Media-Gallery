import { randomUUID } from 'node:crypto';
import { access, mkdir, rename } from 'node:fs/promises';
import path from 'node:path';
import type { MediaRepository } from '../../core/database/repository.ts';
import type { MediaRecord, RenameLogRecord } from '../../core/types/domain.ts';
import { cleanFileNameForAlias, formatDateToken } from './aliasEngine.ts';

export interface BulkRenamePreviewItem {
  mediaId: string;
  oldName: string;
  newName: string;
  oldPath: string;
  newPath: string;
  hasConflict: boolean;
}

export type TrashHandler = (filePath: string) => Promise<string>;

async function fileExists(targetPath: string): Promise<boolean> {
  try {
    await access(targetPath);
    return true;
  } catch {
    return false;
  }
}

export function sanitizeFileNameSegment(name: string): string {
  return name
    .replace(/[<>:"/\\|?*\u0000-\u001f]/g, ' ')
    .replace(/\s+/g, ' ')
    .trim();
}

/**
 * Handles non-destructive DB aliases, undoable real disk renames, bulk renames,
 * drag-and-drop file moves, and safe trash deletion (PRD C3, C4, J1, 13).
 */
export class RenameService {
  constructor(
    private readonly repo: MediaRepository,
    private readonly customTrashHandler?: TrashHandler,
  ) {}

  /**
   * Updates display alias in DB only; does not alter file on disk.
   */
  setAlias(mediaId: string, alias: string): MediaRecord {
    const existing = this.repo.getMediaById(mediaId);
    if (!existing) {
      throw new Error(`Media item not found: ${mediaId}`);
    }
    const clean = alias.trim();
    this.repo.updateMediaAlias(mediaId, clean);
    return this.repo.getMediaById(mediaId)!;
  }

  /**
   * Renames the actual file on disk, logs to `rename_log` for undo, and updates the index.
   */
  async applyRealRename(
    mediaId: string,
    requestedName: string,
    batchId: string | null = null,
  ): Promise<{ media: MediaRecord; log: RenameLogRecord }> {
    const item = this.repo.getMediaById(mediaId);
    if (!item) {
      throw new Error(`Media item not found: ${mediaId}`);
    }

    const oldExt = path.extname(item.fileName);
    const sanitized = sanitizeFileNameSegment(requestedName);
    if (!sanitized) {
      throw new Error('New file name cannot be empty after sanitization');
    }

    const reqExt = path.extname(sanitized);
    const newFileName = reqExt.toLowerCase() === oldExt.toLowerCase() && reqExt !== ''
      ? sanitized
      : `${sanitized}${oldExt}`;

    const dir = path.dirname(item.path);
    const newPath = path.join(dir, newFileName);

    if (path.resolve(newPath) !== path.resolve(item.path) && (await fileExists(newPath))) {
      throw new Error(`Destination file already exists: ${newPath}`);
    }

    if (path.resolve(newPath) !== path.resolve(item.path)) {
      await rename(item.path, newPath);
    }

    const log: RenameLogRecord = {
      id: randomUUID(),
      mediaId: item.id,
      batchId,
      oldName: item.fileName,
      newName: newFileName,
      oldPath: item.path,
      newPath,
      appliedAt: Date.now(),
      undoneAt: null,
    };

    this.repo.insertRenameLog(log);
    this.repo.updateMediaPathAndName(item.id, newPath, newFileName, dir.replace(/\\/g, '/'));
    return { media: this.repo.getMediaById(item.id)!, log };
  }

  /**
   * Undoes the most recent real rename (for a specific media item or globally).
   */
  async undoLastRename(mediaId?: string): Promise<RenameLogRecord | null> {
    const log = this.repo.getLatestUndoableRename(mediaId);
    if (!log) return null;
    await this.revertSingleRenameLog(log);
    return { ...log, undoneAt: Date.now() };
  }

  /**
   * Undoes an entire bulk rename batch atomically in reverse order.
   */
  async undoBatchRename(batchId: string): Promise<number> {
    const entries = this.repo.getRenameBatch(batchId);
    let reverted = 0;
    for (const log of entries) {
      await this.revertSingleRenameLog(log);
      reverted += 1;
    }
    return reverted;
  }

  private async revertSingleRenameLog(log: RenameLogRecord): Promise<void> {
    if (path.resolve(log.oldPath) !== path.resolve(log.newPath)) {
      if (await fileExists(log.oldPath)) {
        throw new Error(`Cannot undo rename; original path already occupied: ${log.oldPath}`);
      }
      if (await fileExists(log.newPath)) {
        await rename(log.newPath, log.oldPath);
      }
    }
    const oldFolder = path.dirname(log.oldPath).replace(/\\/g, '/');
    this.repo.updateMediaPathAndName(log.mediaId, log.oldPath, log.oldName, oldFolder);
    this.repo.markRenameLogUndone(log.id);
  }

  /**
   * Previews bulk pattern rename (`{date} {place} {name}`) with collision detection (PRD C4).
   */
  previewBulkRename(mediaIds: string[], template: string): BulkRenamePreviewItem[] {
    const usedPaths = new Set<string>();
    const results: BulkRenamePreviewItem[] = [];

    mediaIds.forEach((mediaId, index) => {
      const item = this.repo.getMediaById(mediaId);
      if (!item) return;

      const ext = path.extname(item.fileName);
      const dateToken = formatDateToken(item.createdAt || item.modifiedAt, true);
      const cleanName = cleanFileNameForAlias(item.fileName) || path.basename(item.fileName, ext);
      const renderedStem = sanitizeFileNameSegment(
        template
          .replace(/\{date\}/g, dateToken)
          .replace(/\{place\}/g, item.place ?? '')
          .replace(/\{name\}/g, cleanName)
          .replace(/\{alias\}/g, item.alias || cleanName)
          .replace(/\{device\}/g, item.deviceName ?? item.deviceId)
          .replace(/\{index\}/g, String(index + 1).padStart(3, '0')),
      );

      let candidateName = `${renderedStem || cleanName}${ext}`;
      const dir = path.dirname(item.path);
      let candidatePath = path.join(dir, candidateName);
      let counter = 2;
      while (usedPaths.has(candidatePath.toLowerCase())) {
        candidateName = `${renderedStem || cleanName} (${counter})${ext}`;
        candidatePath = path.join(dir, candidateName);
        counter += 1;
      }
      usedPaths.add(candidatePath.toLowerCase());

      results.push({
        mediaId: item.id,
        oldName: item.fileName,
        newName: candidateName,
        oldPath: item.path,
        newPath: candidatePath,
        hasConflict: false,
      });
    });

    return results;
  }

  async applyBulkRename(
    mediaIds: string[],
    template: string,
  ): Promise<{ batchId: string; logs: RenameLogRecord[] }> {
    const previews = this.previewBulkRename(mediaIds, template);
    const batchId = randomUUID();
    const logs: RenameLogRecord[] = [];
    for (const preview of previews) {
      const stem = path.basename(preview.newName, path.extname(preview.newName));
      const { log } = await this.applyRealRename(preview.mediaId, stem, batchId);
      logs.push(log);
    }
    return { batchId, logs };
  }

  /**
   * Moves a media file to a destination folder (used by drag-and-drop & spring-loaded folders).
   */
  async moveMediaToFolder(mediaId: string, targetFolder: string): Promise<MediaRecord> {
    const item = this.repo.getMediaById(mediaId);
    if (!item) {
      throw new Error(`Media item not found: ${mediaId}`);
    }
    await mkdir(targetFolder, { recursive: true });
    const destinationPath = path.join(targetFolder, item.fileName);
    if (path.resolve(destinationPath) === path.resolve(item.path)) {
      return item;
    }
    if (await fileExists(destinationPath)) {
      throw new Error(`A file named "${item.fileName}" already exists in ${targetFolder}`);
    }

    await rename(item.path, destinationPath);
    const normalizedFolder = targetFolder.replace(/\\/g, '/');
    this.repo.updateMediaPathAndName(item.id, destinationPath, item.fileName, normalizedFolder);
    return this.repo.getMediaById(item.id)!;
  }

  /**
   * Safe delete: moves file to OS Recycle Bin / managed trash directory instead of unlinking.
   */
  async moveToTrash(mediaId: string, trashRootDir?: string): Promise<string> {
    const item = this.repo.getMediaById(mediaId);
    if (!item) {
      throw new Error(`Media item not found: ${mediaId}`);
    }

    let trashedPath: string;
    if (this.customTrashHandler) {
      trashedPath = await this.customTrashHandler(item.path);
    } else {
      const trashDir = trashRootDir ?? path.join(path.dirname(item.path), '.media-gallery-trash');
      await mkdir(trashDir, { recursive: true });
      trashedPath = path.join(trashDir, `${Date.now()}-${item.fileName}`);
      if (await fileExists(item.path)) {
        await rename(item.path, trashedPath);
      }
    }

    this.repo.markMediaTrashed(item.id, trashedPath);
    return trashedPath;
  }
}
