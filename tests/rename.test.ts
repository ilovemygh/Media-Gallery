import { access, mkdtemp, mkdir, rm, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { afterEach, describe, expect, it } from 'vitest';
import { NodeSqliteAdapter } from '../src/core/database/nodeSqliteAdapter.ts';
import { MediaRepository } from '../src/core/database/repository.ts';
import { RenameService } from '../src/features/alias/renameService.ts';
import { ScannerService } from '../src/features/scanner/scannerService.ts';

async function exists(p: string): Promise<boolean> {
  try {
    await access(p);
    return true;
  } catch {
    return false;
  }
}

describe('RenameService, Undo Log, Bulk Rename & Trash (PRD C3, C4, J1)', () => {
  const tempDirs: string[] = [];

  afterEach(async () => {
    for (const dir of tempDirs.splice(0)) {
      await rm(dir, { recursive: true, force: true });
    }
  });

  async function setupFiles(names: string[]) {
    const root = await mkdtemp(path.join(os.tmpdir(), 'mg-ren-'));
    tempDirs.push(root);
    const lib = path.join(root, 'photos');
    await mkdir(lib, { recursive: true });
    for (const name of names) {
      await writeFile(path.join(lib, name), Buffer.from(`file-${name}`));
    }
    const repo = new MediaRepository(new NodeSqliteAdapter(':memory:'));
    repo.upsertDevice({
      id: 'pc-1',
      name: 'PC',
      type: 'pc',
      publicKey: 'pk',
      lastSeen: Date.now(),
      transports: ['wifi_lan'],
      isSelf: true,
      isOnline: true,
    });
    const scanner = new ScannerService(repo);
    await scanner.scanFolders({ deviceId: 'pc-1', deviceName: 'PC', roots: [lib] });
    return { root, lib, repo, renameService: new RenameService(repo) };
  }

  it('keeps DB aliases non-destructive and makes real disk renames undoable via rename_log', async () => {
    const { lib, repo, renameService } = await setupFiles(['IMG_2031.jpg']);
    const item = repo.queryMedia('', {})[0];

    // Non-destructive alias update
    renameService.setAlias(item.id, 'Bali Sunset');
    expect(await exists(path.join(lib, 'IMG_2031.jpg'))).toBe(true);
    expect(repo.getMediaById(item.id)?.alias).toBe('Bali Sunset');

    // Apply as real rename
    const { media: renamed, log } = await renameService.applyRealRename(item.id, 'Bali Sunset');
    expect(renamed.fileName).toBe('Bali Sunset.jpg');
    expect(await exists(path.join(lib, 'Bali Sunset.jpg'))).toBe(true);
    expect(await exists(path.join(lib, 'IMG_2031.jpg'))).toBe(false);
    expect(repo.listRenameLogs(item.id)).toHaveLength(1);
    expect(log.oldName).toBe('IMG_2031.jpg');

    // Undo rename
    const undone = await renameService.undoLastRename(item.id);
    expect(undone).not.toBeNull();
    expect(await exists(path.join(lib, 'IMG_2031.jpg'))).toBe(true);
    expect(await exists(path.join(lib, 'Bali Sunset.jpg'))).toBe(false);
    expect(repo.getMediaById(item.id)?.fileName).toBe('IMG_2031.jpg');

    repo.close();
  });

  it('previews and applies bulk pattern rename with batch undo (C4) and safe trash delete', async () => {
    const { root, lib, repo, renameService } = await setupFiles(['IMG_01.jpg', 'IMG_02.jpg']);
    const items = repo.queryMedia('', { sortBy: 'name', sortDir: 'asc' });

    const preview = renameService.previewBulkRename(
      items.map((i) => i.id),
      '{date} Vacation {index}',
    );
    expect(preview).toHaveLength(2);
    expect(preview[0].newName).toContain('Vacation 001.jpg');
    expect(preview[1].newName).toContain('Vacation 002.jpg');

    const { batchId, logs } = await renameService.applyBulkRename(
      items.map((i) => i.id),
      '{date} Vacation {index}',
    );
    expect(logs).toHaveLength(2);
    expect(await exists(path.join(lib, logs[0].newName))).toBe(true);

    const revertedCount = await renameService.undoBatchRename(batchId);
    expect(revertedCount).toBe(2);
    expect(await exists(path.join(lib, 'IMG_01.jpg'))).toBe(true);
    expect(await exists(path.join(lib, 'IMG_02.jpg'))).toBe(true);

    // Move to folder (drag and drop)
    const destFolder = path.join(root, 'Archive');
    const targetItem = items.find((i) => i.fileName === 'IMG_01.jpg')!;
    const moved = await renameService.moveMediaToFolder(targetItem.id, destFolder);
    expect(await exists(path.join(destFolder, 'IMG_01.jpg'))).toBe(true);
    expect(moved.folderPath).toBe(destFolder.replace(/\\/g, '/'));

    // Safe delete moves to trash only
    const trashedPath = await renameService.moveToTrash(targetItem.id);
    expect(await exists(trashedPath)).toBe(true);
    expect(await exists(path.join(destFolder, 'IMG_01.jpg'))).toBe(false);
    expect(repo.countMedia('', {})).toBe(1);

    repo.close();
  });
});
