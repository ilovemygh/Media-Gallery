import { createHash } from 'node:crypto';
import { readdir, stat } from 'node:fs/promises';
import path from 'node:path';
import type { MediaRepository } from '../../core/database/repository.ts';
import type { MediaRecord, MediaTags, ScanProgress } from '../../core/types/domain.ts';
import { computePartialHash } from '../../core/utils/hash.ts';
import { generateAutoAlias } from '../alias/aliasEngine.ts';
import { ThumbnailService } from '../thumbnails/thumbnailService.ts';
import { classifyMediaExtension } from './mediaFormats.ts';

export interface MetadataProbeResult {
  durationMs?: number;
  width?: number;
  height?: number;
  createdAt?: number;
  gpsLat?: number | null;
  gpsLon?: number | null;
  tags?: MediaTags;
}

export type MetadataProber = (filePath: string) => Promise<MetadataProbeResult>;

export interface ScanOptions {
  deviceId: string;
  deviceName: string;
  roots: string[];
  excludedFolders?: string[];
  cacheDir?: string;
  generateThumbnails?: boolean;
  computeHashes?: boolean;
  batchSize?: number;
  aliasTemplate?: string;
  probeMetadata?: MetadataProber;
  onProgress?: (progress: ScanProgress) => void;
}

function stableMediaId(deviceId: string, fullPath: string): string {
  const normalized = path.resolve(fullPath).toLowerCase();
  return createHash('sha256').update(`${deviceId}::${normalized}`).digest('hex').slice(0, 24);
}

function normalizeForPrefix(p: string): string {
  return path.resolve(p).replace(/\\/g, '/').toLowerCase();
}

function isExcludedPath(targetPath: string, excludedPrefixes: string[]): boolean {
  const normalized = normalizeForPrefix(targetPath);
  const base = path.basename(targetPath);
  if (base === '.media-gallery-trash' || base.startsWith('.')) {
    return true;
  }
  return excludedPrefixes.some(
    (prefix) => normalized === prefix || normalized.startsWith(`${prefix}/`),
  );
}

function yieldToEventLoop(): Promise<void> {
  return new Promise((resolve) => setImmediate(resolve));
}

/**
 * Non-blocking recursive & incremental media scanner (PRD A1–A4, Section 13).
 * Uses chunked directory traversal, event-loop yielding, and batched SQLite transactions
 * so 100,000+ files scan smoothly without freezing the UI thread.
 */
export class ScannerService {
  private readonly thumbnailService: ThumbnailService;

  constructor(private readonly repo: MediaRepository) {
    this.thumbnailService = new ThumbnailService(repo);
  }

  async scanFolders(options: ScanOptions): Promise<ScanProgress> {
    const batchSize = Math.max(25, options.batchSize ?? 200);
    const excludedPrefixes = (options.excludedFolders ?? []).map(normalizeForPrefix);
    const progress: ScanProgress = {
      scannedFiles: 0,
      addedFiles: 0,
      updatedFiles: 0,
      skippedFiles: 0,
      currentFolder: '',
      inProgress: true,
    };

    const emitProgress = () => {
      options.onProgress?.({ ...progress });
    };

    const queue: string[] = [...options.roots];
    const pendingBatch: MediaRecord[] = [];
    const seenPathsInScan = new Set<string>();

    const flushBatch = async () => {
      if (pendingBatch.length === 0) return;
      const chunk = pendingBatch.splice(0, pendingBatch.length);
      this.repo.upsertMediaBatch(chunk);
      if (options.generateThumbnails && options.cacheDir) {
        for (const media of chunk) {
          if (media.type === 'video' || media.type === 'audio') {
            await this.thumbnailService.ensureThumbnail(media, {
              cacheDir: options.cacheDir,
              useFfmpeg: true,
            });
          }
        }
      }
      await yieldToEventLoop();
    };

    while (queue.length > 0) {
      const currentDir = queue.shift()!;
      if (isExcludedPath(currentDir, excludedPrefixes)) continue;
      progress.currentFolder = currentDir;

      let entries;
      try {
        entries = await readdir(currentDir, { withFileTypes: true });
      } catch {
        continue;
      }

      for (const entry of entries) {
        const fullPath = path.join(currentDir, entry.name);
        if (isExcludedPath(fullPath, excludedPrefixes)) continue;

        if (entry.isDirectory()) {
          queue.push(fullPath);
          continue;
        }
        if (!entry.isFile()) continue;

        const spec = classifyMediaExtension(path.extname(entry.name));
        if (!spec) continue;

        progress.scannedFiles += 1;
        seenPathsInScan.add(fullPath);

        let fileStat;
        try {
          fileStat = await stat(fullPath);
        } catch {
          continue;
        }

        const modifiedAt = Math.round(fileStat.mtimeMs);
        const existing = this.repo.getMediaByPath(options.deviceId, fullPath);
        if (existing && existing.size === fileStat.size && existing.modifiedAt === modifiedAt) {
          progress.skippedFiles += 1;
          if (progress.scannedFiles % batchSize === 0) {
            emitProgress();
            await yieldToEventLoop();
          }
          continue;
        }

        const probed = options.probeMetadata
          ? await options.probeMetadata(fullPath).catch(() => ({} as MetadataProbeResult))
          : ({} as MetadataProbeResult);

        const createdAt = probed.createdAt ?? Math.round(fileStat.birthtimeMs || modifiedAt);
        const partialHash =
          options.computeHashes !== false ? await computePartialHash(fullPath).catch(() => '') : '';

        const { autoAlias, resolvedPlace } = generateAutoAlias({
          fileName: entry.name,
          type: spec.type,
          createdAt,
          modifiedAt,
          gpsLat: probed.gpsLat ?? null,
          gpsLon: probed.gpsLon ?? null,
          tags: probed.tags,
          deviceName: options.deviceName,
          template: options.aliasTemplate,
        });

        const record: MediaRecord = {
          id: existing?.id ?? stableMediaId(options.deviceId, fullPath),
          deviceId: options.deviceId,
          path: fullPath,
          folderPath: path.dirname(fullPath).replace(/\\/g, '/'),
          fileName: entry.name,
          type: spec.type,
          mime: spec.mime,
          size: fileStat.size,
          hash: existing?.hash ?? partialHash,
          partialHash,
          createdAt,
          modifiedAt,
          durationMs: probed.durationMs ?? existing?.durationMs ?? 0,
          width: probed.width ?? existing?.width ?? 0,
          height: probed.height ?? existing?.height ?? 0,
          gpsLat: probed.gpsLat ?? null,
          gpsLon: probed.gpsLon ?? null,
          place: resolvedPlace,
          tags: probed.tags ?? existing?.tags ?? {},
          alias: existing?.alias || autoAlias,
          autoAlias,
          coverId: existing?.coverId ?? null,
          syncState: existing?.syncState ?? 'Local',
          isDuplicate: existing?.isDuplicate ?? false,
          duplicateGroupId: existing?.duplicateGroupId ?? null,
          isTrashed: false,
          trashedPath: null,
          updatedAt: Date.now(),
        };

        if (existing) {
          progress.updatedFiles += 1;
        } else {
          progress.addedFiles += 1;
        }

        pendingBatch.push(record);
        if (pendingBatch.length >= batchSize) {
          await flushBatch();
          emitProgress();
        }
      }
    }

    await flushBatch();
    progress.inProgress = false;
    emitProgress();
    return progress;
  }

  /**
   * Indexes or refreshes a single file path (used by file watcher & downloads).
   */
  async indexSingleFile(
    filePath: string,
    options: Omit<ScanOptions, 'roots'>,
  ): Promise<MediaRecord | null> {
    const ext = path.extname(filePath);
    const spec = classifyMediaExtension(ext);
    if (!spec) return null;

    let fileStat;
    try {
      fileStat = await stat(filePath);
    } catch {
      this.repo.deleteMediaByDevicePath(options.deviceId, filePath);
      return null;
    }
    if (!fileStat.isFile()) return null;

    const probed = options.probeMetadata
      ? await options.probeMetadata(filePath).catch(() => ({} as MetadataProbeResult))
      : ({} as MetadataProbeResult);
    const modifiedAt = Math.round(fileStat.mtimeMs);
    const createdAt = probed.createdAt ?? Math.round(fileStat.birthtimeMs || modifiedAt);
    const partialHash =
      options.computeHashes !== false ? await computePartialHash(filePath).catch(() => '') : '';
    const fileName = path.basename(filePath);
    const existing = this.repo.getMediaByPath(options.deviceId, filePath);

    const { autoAlias, resolvedPlace } = generateAutoAlias({
      fileName,
      type: spec.type,
      createdAt,
      modifiedAt,
      gpsLat: probed.gpsLat ?? null,
      gpsLon: probed.gpsLon ?? null,
      tags: probed.tags,
      deviceName: options.deviceName,
      template: options.aliasTemplate,
    });

    const record: MediaRecord = {
      id: existing?.id ?? stableMediaId(options.deviceId, filePath),
      deviceId: options.deviceId,
      path: filePath,
      folderPath: path.dirname(filePath).replace(/\\/g, '/'),
      fileName,
      type: spec.type,
      mime: spec.mime,
      size: fileStat.size,
      hash: existing?.hash ?? partialHash,
      partialHash,
      createdAt,
      modifiedAt,
      durationMs: probed.durationMs ?? existing?.durationMs ?? 0,
      width: probed.width ?? existing?.width ?? 0,
      height: probed.height ?? existing?.height ?? 0,
      gpsLat: probed.gpsLat ?? null,
      gpsLon: probed.gpsLon ?? null,
      place: resolvedPlace,
      tags: probed.tags ?? existing?.tags ?? {},
      alias: existing?.alias || autoAlias,
      autoAlias,
      coverId: existing?.coverId ?? null,
      syncState: existing?.syncState ?? 'Local',
      isDuplicate: existing?.isDuplicate ?? false,
      duplicateGroupId: existing?.duplicateGroupId ?? null,
      isTrashed: false,
      trashedPath: null,
      updatedAt: Date.now(),
    };

    this.repo.upsertMedia(record);
    if (options.generateThumbnails && options.cacheDir && (record.type === 'video' || record.type === 'audio')) {
      await this.thumbnailService.ensureThumbnail(record, {
        cacheDir: options.cacheDir,
        useFfmpeg: true,
      });
    }
    return record;
  }
}
