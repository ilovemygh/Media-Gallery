import { afterEach, describe, expect, it } from 'vitest';
import { NodeSqliteAdapter } from '../src/core/database/nodeSqliteAdapter.ts';
import { MediaRepository } from '../src/core/database/repository.ts';
import { generateAutoAlias } from '../src/features/alias/aliasEngine.ts';
import { SearchService } from '../src/features/search/searchService.ts';

describe('FTS5 Search, Filters, and Smart Folders (PRD C1, C5, C6)', () => {
  let repo: MediaRepository;

  afterEach(() => {
    repo?.close();
  });

  it('finds "2026 beach" by auto-alias (date + offline place) as well as real file name', () => {
    repo = new MediaRepository(new NodeSqliteAdapter(':memory:'));
    repo.upsertDevice({
      id: 'pc-1',
      name: 'Workstation',
      type: 'pc',
      publicKey: 'pk-1',
      lastSeen: Date.now(),
      transports: ['wifi_lan'],
      isSelf: true,
      isOnline: true,
    });

    const march2026 = Date.UTC(2026, 2, 14, 8, 30, 0);
    const { autoAlias, resolvedPlace } = generateAutoAlias({
      fileName: 'IMG_2031.jpg',
      type: 'image',
      createdAt: march2026,
      modifiedAt: march2026,
      gpsLat: -8.7185,
      gpsLon: 115.1686,
      deviceName: 'Workstation',
    });

    repo.upsertMedia({
      id: 'm-beach',
      deviceId: 'pc-1',
      path: 'C:/Photos/IMG_2031.jpg',
      folderPath: 'C:/Photos',
      fileName: 'IMG_2031.jpg',
      type: 'image',
      mime: 'image/jpeg',
      size: 4_500_000,
      hash: 'h1',
      partialHash: 'p1',
      createdAt: march2026,
      modifiedAt: march2026,
      durationMs: 0,
      width: 4000,
      height: 3000,
      gpsLat: -8.7185,
      gpsLon: 115.1686,
      place: resolvedPlace,
      tags: {},
      alias: autoAlias,
      autoAlias,
      coverId: null,
      syncState: 'Local',
    });

    repo.upsertMedia({
      id: 'm-city',
      deviceId: 'pc-1',
      path: 'C:/Photos/IMG_9999.jpg',
      folderPath: 'C:/Photos',
      fileName: 'IMG_9999.jpg',
      type: 'image',
      mime: 'image/jpeg',
      size: 1_200_000,
      hash: 'h2',
      partialHash: 'p2',
      createdAt: Date.UTC(2025, 0, 5, 22, 0, 0),
      modifiedAt: Date.UTC(2025, 0, 5, 22, 0, 0),
      durationMs: 0,
      width: 1920,
      height: 1080,
      gpsLat: 51.5072,
      gpsLon: -0.1276,
      place: 'London',
      tags: {},
      alias: '2025-01-05 London',
      autoAlias: '2025-01-05 London',
      coverId: null,
      syncState: 'Local',
    });

    const search = new SearchService(repo);
    const byAlias = search.search('2026 beach');
    expect(byAlias.totalCount).toBe(1);
    expect(byAlias.items[0].id).toBe('m-beach');

    const byRawName = search.search('IMG_2031');
    expect(byRawName.totalCount).toBe(1);
    expect(byRawName.items[0].id).toBe('m-beach');

    const bySizeFilter = search.search('', { minSize: 3_000_000 });
    expect(bySizeFilter.totalCount).toBe(1);
    expect(bySizeFilter.items[0].id).toBe('m-beach');

    const saved = search.saveSearch('Beach Trip', '2026 beach', { type: 'image' });
    expect(search.listSavedSearches().map((s) => s.id)).toContain(saved.id);
  });
});
