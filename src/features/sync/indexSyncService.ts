import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import type { MediaRepository } from '../../core/database/repository.ts';
import type {
  AutoSyncRuleRecord,
  DeviceRecord,
  IndexDeltaPayload,
  MediaWithCover,
} from '../../core/types/domain.ts';

/**
 * Serverless Device-to-Device Index Sync ("serverless drive", PRD E1–E6, 10.4–10.6).
 * Only metadata records and small cached thumbnails ever move in index deltas.
 */
export class IndexSyncService {
  constructor(private readonly repo: MediaRepository) {}

  async exportLocalDelta(
    selfDeviceId: string,
    sinceTimestamp = 0,
    includeThumbnailBlobs = true,
  ): Promise<IndexDeltaPayload> {
    const delta = this.repo.exportDeltaSince(selfDeviceId, sinceTimestamp);

    if (includeThumbnailBlobs) {
      for (const cover of delta.upsertedCovers) {
        if (!cover.filePath) continue;
        try {
          const bytes = await readFile(cover.filePath);
          const ext = path.extname(cover.filePath).replace('.', '').toLowerCase() || 'svg';
          const mime = ext === 'svg' ? 'image/svg+xml' : `image/${ext === 'jpg' ? 'jpeg' : ext}`;
          delta.thumbnailBlobs[cover.mediaId] = `data:${mime};base64,${bytes.toString('base64')}`;
        } catch {
          // Ignore missing local cache file cleanly.
        }
      }
    }

    return delta;
  }

  async importRemoteDelta(
    delta: IndexDeltaPayload,
    selfDeviceId: string,
    remoteThumbCacheDir: string,
  ): Promise<{ importedCount: number }> {
    await mkdir(remoteThumbCacheDir, { recursive: true });

    const rewrittenCovers = await Promise.all(
      delta.upsertedCovers.map(async (cover) => {
        const dataUri = delta.thumbnailBlobs[cover.mediaId];
        if (!dataUri) return cover;
        const commaIdx = dataUri.indexOf(',');
        const header = commaIdx >= 0 ? dataUri.slice(0, commaIdx) : '';
        const base64Body = commaIdx >= 0 ? dataUri.slice(commaIdx + 1) : dataUri;
        const ext = header.includes('svg') ? 'svg' : header.includes('png') ? 'png' : 'jpg';
        const localThumbPath = path.join(remoteThumbCacheDir, `${cover.mediaId}.${ext}`);
        await writeFile(localThumbPath, Buffer.from(base64Body, 'base64'));
        return { ...cover, filePath: localThumbPath };
      }),
    );

    return this.repo.applyRemoteDelta(
      { ...delta, upsertedCovers: rewrittenCovers },
      selfDeviceId,
    );
  }

  /**
   * Validates whether a remote item can be downloaded on demand (PRD E2, E6).
   */
  assertDownloadAllowed(media: MediaWithCover): { owner: DeviceRecord } {
    const owner = this.repo.getDevice(media.deviceId);
    if (!owner) {
      throw new Error('Cannot download: owning device is no longer registered.');
    }
    if (!owner.isOnline) {
      throw new Error(
        `Device "${owner.name}" is currently offline. Cached thumbnail remains browsable, but Download is disabled until the device reconnects.`,
      );
    }
    return { owner };
  }

  /**
   * Builds a direct LAN preview/stream URL without saving the remote file locally (PRD E4).
   */
  getLanStreamPreviewUrl(media: MediaWithCover): string | null {
    const owner = this.repo.getDevice(media.deviceId);
    if (!owner || !owner.isOnline || !owner.host || !owner.port) {
      return null;
    }
    return `https://${owner.host}:${owner.port}/stream/${encodeURIComponent(media.id)}`;
  }

  /**
   * Evaluates enabled auto-sync rules to determine remote items queued for sync (PRD E3).
   */
  findPendingAutoSyncItems(rules: AutoSyncRuleRecord[], selfDeviceId: string): MediaWithCover[] {
    const enabled = rules.filter((r) => r.enabled && r.targetDeviceId === selfDeviceId);
    if (enabled.length === 0) return [];

    const remoteItems = this.repo.queryMedia('', {
      syncState: 'Remote',
      limit: 5000,
    });

    return remoteItems.filter((item) =>
      enabled.some(
        (rule) =>
          rule.sourceDeviceId === item.deviceId &&
          rule.mediaTypes.includes(item.type) &&
          item.folderPath.toLowerCase().includes(rule.folderPattern.toLowerCase()),
      ),
    );
  }
}
