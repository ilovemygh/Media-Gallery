import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { afterEach, describe, expect, it } from 'vitest';
import { NodeSqliteAdapter } from '../src/core/database/nodeSqliteAdapter.ts';
import { MediaRepository } from '../src/core/database/repository.ts';
import type { MediaRecord } from '../src/core/types/domain.ts';
import {
  computeCropForAspect,
  CoverEditorService,
} from '../src/features/thumbnails/coverEditorService.ts';
import {
  selectRandomFirstThirdFrameMs,
  ThumbnailService,
} from '../src/features/thumbnails/thumbnailService.ts';

describe('Thumbnail Frame Selection & Cover Editor (PRD I1–I4)', () => {
  const tempDirs: string[] = [];

  afterEach(async () => {
    for (const dir of tempDirs.splice(0)) {
      await rm(dir, { recursive: true, force: true });
    }
  });

  function makeRepo(): MediaRepository {
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
    return repo;
  }

  it('always selects video thumbnail frame inside the first third [0, duration/3]', () => {
    expect(selectRandomFirstThirdFrameMs(0)).toBe(0);
    expect(selectRandomFirstThirdFrameMs(-500)).toBe(0);

    const durationMs = 90_000;
    for (let i = 0; i < 100; i += 1) {
      const t = selectRandomFirstThirdFrameMs(durationMs);
      expect(t).toBeGreaterThanOrEqual(0);
      expect(t).toBeLessThanOrEqual(durationMs / 3);
    }
  });

  it('persists chosen video thumbnail across restarts without re-rolling (I1)', async () => {
    const dir = await mkdtemp(path.join(os.tmpdir(), 'mg-thumb-'));
    tempDirs.push(dir);
    const dbPath = path.join(dir, 'gallery.db');
    const cacheDir = path.join(dir, 'thumbs');

    const video: MediaRecord = {
      id: 'vid-1',
      deviceId: 'pc-1',
      path: path.join(dir, 'sample.mp4'),
      folderPath: dir.replace(/\\/g, '/'),
      fileName: 'sample.mp4',
      type: 'video',
      mime: 'video/mp4',
      size: 1024,
      hash: 'h',
      createdAt: Date.now(),
      modifiedAt: Date.now(),
      durationMs: 120_000,
      width: 1920,
      height: 1080,
      gpsLat: null,
      gpsLon: null,
      place: null,
      tags: {},
      alias: 'Sample Video',
      autoAlias: 'Sample Video',
      coverId: null,
      syncState: 'Local',
    };

    const repo1 = new MediaRepository(new NodeSqliteAdapter(dbPath));
    repo1.upsertDevice({
      id: 'pc-1',
      name: 'PC',
      type: 'pc',
      publicKey: 'pk',
      lastSeen: Date.now(),
      transports: ['wifi_lan'],
      isSelf: true,
      isOnline: true,
    });
    repo1.upsertMedia(video);

    const service1 = new ThumbnailService(repo1);
    const firstCover = await service1.ensureThumbnail(video, {
      cacheDir,
      useFfmpeg: false,
      rng: () => 0.5,
    });
    expect(firstCover.frameTimeMs).toBeGreaterThan(0);
    expect(firstCover.frameTimeMs).toBeLessThanOrEqual(40_000);
    repo1.close();

    // Simulate app restart against same SQLite file with a different rng
    const repo2 = new MediaRepository(new NodeSqliteAdapter(dbPath));
    const service2 = new ThumbnailService(repo2);
    const reloadedCover = await service2.ensureThumbnail(video, {
      cacheDir,
      useFfmpeg: false,
      rng: () => 0.99,
    });
    expect(reloadedCover.frameTimeMs).toBe(firstCover.frameTimeMs);
    expect(reloadedCover.filePath).toBe(firstCover.filePath);
    repo2.close();
  });

  it('supports all aspect presets (4:3, 9:16, 16:9, 1:1, custom) and audio cover confirmation (I3, I4)', async () => {
    const dir = await mkdtemp(path.join(os.tmpdir(), 'mg-cov-'));
    tempDirs.push(dir);
    const cacheDir = path.join(dir, 'cache');
    const customImg = path.join(dir, 'cover.jpg');
    await writeFile(customImg, Buffer.from('fake-jpeg-data'));

    expect(computeCropForAspect(1600, 1200, '1:1')).toMatchObject({ width: 1200, height: 1200 });
    expect(computeCropForAspect(1920, 1080, '9:16').width).toBe(608);
    expect(computeCropForAspect(1920, 1080, '4:3').width).toBe(1440);
    expect(computeCropForAspect(1080, 1920, '16:9').height).toBe(608);

    const repo = makeRepo();
    const audio: MediaRecord = {
      id: 'aud-1',
      deviceId: 'pc-1',
      path: path.join(dir, 'song.mp3'),
      folderPath: dir.replace(/\\/g, '/'),
      fileName: 'song.mp3',
      type: 'audio',
      mime: 'audio/mpeg',
      size: 4096,
      hash: 'ha',
      createdAt: Date.now(),
      modifiedAt: Date.now(),
      durationMs: 180_000,
      width: 0,
      height: 0,
      gpsLat: null,
      gpsLon: null,
      place: null,
      tags: { artist: 'Artist', title: 'Track' },
      alias: 'Artist - Track',
      autoAlias: 'Artist - Track',
      coverId: null,
      syncState: 'Local',
    };
    repo.upsertMedia(audio);

    const thumbs = new ThumbnailService(repo);
    const initialCover = await thumbs.ensureThumbnail(audio, { cacheDir, useFfmpeg: false });

    const editor = new CoverEditorService(repo);
    await expect(
      editor.replaceAudioCover({
        mediaId: audio.id,
        imagePath: customImg,
        cacheDir,
        aspect: '16:9',
        confirmed: false,
      }),
    ).rejects.toThrow(/confirmation/i);

    const updated = await editor.replaceAudioCover({
      mediaId: audio.id,
      imagePath: customImg,
      cacheDir,
      aspect: '16:9',
      confirmed: true,
    });
    expect(updated.source).toBe('custom');
    expect(updated.aspect).toBe('16:9');
    expect(updated.crop?.aspectRatio).toBeCloseTo(16 / 9, 2);

    const restored = editor.restoreAudioCover(audio.id);
    expect(restored?.filePath).toBe(initialCover.filePath);

    repo.close();
  });
});
