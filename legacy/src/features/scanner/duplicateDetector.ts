import type { MediaRepository } from '../../core/database/repository.ts';
import type { MediaWithCover } from '../../core/types/domain.ts';

export interface DuplicateGroup {
  groupId: string;
  size: number;
  partialHash: string;
  items: MediaWithCover[];
}

/**
 * Flags duplicate media using file size + partial hash without ever deleting automatically (PRD A5).
 */
export class DuplicateDetector {
  constructor(private readonly repo: MediaRepository) {}

  detectAndFlagDuplicates(deviceId?: string): DuplicateGroup[] {
    const allMedia = this.repo.queryMedia('', {
      deviceId: deviceId ?? 'all',
      limit: 10000,
    });

    const buckets = new Map<string, MediaWithCover[]>();
    for (const item of allMedia) {
      if (!item.partialHash) continue;
      const key = `${item.size}:${item.partialHash}`;
      const list = buckets.get(key) ?? [];
      list.push(item);
      buckets.set(key, list);
    }

    const groups: DuplicateGroup[] = [];
    const flagUpdates: Array<{ id: string; isDuplicate: boolean; duplicateGroupId: string | null }> = [];

    for (const [key, items] of buckets.entries()) {
      if (items.length > 1) {
        const groupId = `dup-${key.slice(0, 20)}`;
        groups.push({
          groupId,
          size: items[0].size,
          partialHash: items[0].partialHash ?? '',
          items,
        });
        for (const item of items) {
          flagUpdates.push({ id: item.id, isDuplicate: true, duplicateGroupId: groupId });
        }
      } else if (items[0].isDuplicate) {
        flagUpdates.push({ id: items[0].id, isDuplicate: false, duplicateGroupId: null });
      }
    }

    if (flagUpdates.length > 0) {
      this.repo.updateDuplicateFlags(flagUpdates);
    }

    return groups;
  }
}
