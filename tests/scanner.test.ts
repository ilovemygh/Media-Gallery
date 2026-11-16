import { mkdtemp, mkdir, rm, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { afterEach, describe, expect, it } from 'vitest';
import { NodeSqliteAdapter } from '../src/core/database/nodeSqliteAdapter.ts';
import { MediaRepository } from '../src/core/database/repository.ts';
import { DuplicateDetector } from '../src/features/scanner/duplicateDetector.ts';
import { ScannerService } from '../src/features/scanner/scannerService.ts';

describe('ScannerService & DuplicateDetector (M1)', () => {
  const tempDirs: string[] = [];

  afterEach(async () => {
    for (const dir of tempDirs.splice(0)) {
      await rm(dir, { recursive: true, force: true });
    }
  });

  async function createFixtureEnv() {
    const root = await mkdtemp(path.join(os.tmpdir(), 'mg-scan-'));
    tempDirs.push(root);
    const db = new NodeSqliteAdapter(':memory:');
    const repo = new MediaRepository(db);
    repo.upsertDevice({
      id: 'pc-test',
      name: 'Test PC',
      type: 'pc',
      publicKey: 'pk-test',
      lastSeen: Date.now(),
      transports: ['wifi_lan'],
      isSelf: true,
      isOnline: true,
    });
    return { root, repo };
  }

  it('scans all PRD A1 supported formats and respects excluded folders (A4)', async () => {
    const { root, repo } = await createFixtureEnv();
    const mediaDir = path.join(root, 'library');
    const excludedDir = path.join(mediaDir, 'ignored_cache');
    await mkdir(mediaDir, { recursive: true });
    await mkdir(excludedDir, { recursive: true });

    const supportedFiles = [
      'clip.mp4', 'movie.mkv', 'web.webm', 'phone.mov', 'old.avi',
      'song.mp3', 'voice.m4a', 'lossless.flac', 'wave.wav', 'podcast.ogg',
      'shot.png', 'photo.jpg', 'cam.jpeg', 'anim.gif', 'pic.webp', 'ios.heic',
    ];
    for (const f of supportedFiles) {
      await writeFile(path.join(mediaDir, f), Buffer.from(`data-${f}`));
    }
    await writeFile(path.join(mediaDir, 'readme.txt'), 'not media');
    await writeFile(path.join(excludedDir, 'secret.jpg'), Buffer.from('skip-me'));

    const scanner = new ScannerService(repo);
    const progress = await scanner.scanFolders({
      deviceId: 'pc-test',
      deviceName: 'Test PC',
      roots: [mediaDir],
      excludedFolders: [excludedDir],
    });

    expect(progress.scannedFiles).toBe(supportedFiles.length);
    expect(progress.addedFiles).toBe(supportedFiles.length);
    expect(repo.countMedia('', {})).toBe(supportedFiles.length);

    // Incremental rescan skips unchanged files
    const secondPass = await scanner.scanFolders({
      deviceId: 'pc-test',
      deviceName: 'Test PC',
      roots: [mediaDir],
      excludedFolders: [excludedDir],
    });
    expect(secondPass.skippedFiles).toBe(supportedFiles.length);
    expect(secondPass.addedFiles).toBe(0);

    repo.close();
  });

  it('flags duplicate media by size + partial hash without deleting files (A5)', async () => {
    const { root, repo } = await createFixtureEnv();
    const dirA = path.join(root, 'a');
    const dirB = path.join(root, 'b');
    await mkdir(dirA, { recursive: true });
    await mkdir(dirB, { recursive: true });

    const payload = Buffer.alloc(1024 * 8, 42);
    await writeFile(path.join(dirA, 'IMG_0001.jpg'), payload);
    await writeFile(path.join(dirB, 'copy_IMG_0001.jpg'), payload);
    await writeFile(path.join(dirB, 'unique.jpg'), Buffer.from('completely-different-content'));

    const scanner = new ScannerService(repo);
    await scanner.scanFolders({
      deviceId: 'pc-test',
      deviceName: 'Test PC',
      roots: [root],
    });

    const detector = new DuplicateDetector(repo);
    const groups = detector.detectAndFlagDuplicates('pc-test');

    expect(groups).toHaveLength(1);
    expect(groups[0].items).toHaveLength(2);
    const flagged = repo.queryMedia('', {}).filter((item) => item.isDuplicate);
    expect(flagged).toHaveLength(2);
    expect(repo.countMedia('', {})).toBe(3);

    repo.close();
  });

  it('indexes 10,000 mixed media records and runs paged FTS5 search without stalling (PRD Sec 13 & Criterion 1)', async () => {
    const { repo } = await createFixtureEnv();
    const batch = [];
    const baseTime = Date.UTC(2026, 2, 14, 12, 0, 0);

    for (let i = 0; i < 10_000; i += 1) {
      const isTarget = i === 7777;
      const type = i % 3 === 0 ? 'video' : i % 3 === 1 ? 'image' : 'audio';
      const place = isTarget ? 'Kuta Beach Bali' : 'Home';
      const fileName = isTarget ? 'IMG_2031_beach.jpg' : `IMG_${String(i).padStart(5, '0')}.jpg`;
      batch.push({
        id: `perf-${i}`,
        deviceId: 'pc-test',
        path: `C:/Media/Batch/${Math.floor(i / 500)}/${fileName}`,
        folderPath: `C:/Media/Batch/${Math.floor(i / 500)}`,
        fileName,
        type: type as 'video' | 'image' | 'audio',
        mime: type === 'video' ? 'video/mp4' : type === 'audio' ? 'audio/mpeg' : 'image/jpeg',
        size: 1024 + i,
        hash: `h-${i}`,
        partialHash: `ph-${i}`,
        createdAt: baseTime + i * 1000,
        modifiedAt: baseTime + i * 1000,
        durationMs: type === 'image' ? 0 : 45_000,
        width: 1920,
        height: 1080,
        gpsLat: null,
        gpsLon: null,
        place,
        tags: {},
        alias: `2026-03-14 ${place} ${i}`,
        autoAlias: `2026-03-14 ${place} ${i}`,
        coverId: null,
        syncState: 'Local' as const,
      });
    }

    const startInsert = performance.now();
    repo.upsertMediaBatch(batch);
    const insertMs = performance.now() - startInsert;

    const startSearch = performance.now();
    const hits = repo.queryMedia('2026 beach', { limit: 50, offset: 0 });
    const searchMs = performance.now() - startSearch;

    expect(repo.countMedia('', {})).toBe(10_000);
    expect(hits).toHaveLength(1);
    expect(hits[0].id).toBe('perf-7777');
    expect(insertMs).toBeLessThan(5000);
    expect(searchMs).toBeLessThan(500);

    repo.close();
  });
});
