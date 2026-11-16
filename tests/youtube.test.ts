import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { afterEach, describe, expect, it } from 'vitest';
import { NodeSqliteAdapter } from '../src/core/database/nodeSqliteAdapter.ts';
import { MediaRepository } from '../src/core/database/repository.ts';
import { buildUnifiedMusicLibrary } from '../src/features/player/musicPlayerService.ts';
import {
  buildYtDlpArgs,
  YoutubeService,
  type ProcessRunner,
} from '../src/features/youtube/youtubeService.ts';

describe('YouTube Downloader & Auto-Library Ingestion (M5)', () => {
  const tempDirs: string[] = [];

  afterEach(async () => {
    for (const dir of tempDirs.splice(0)) {
      await rm(dir, { recursive: true, force: true });
    }
  });

  it('constructs valid yt-dlp args for MP4 quality and MP3 audio extraction with playlists', () => {
    const mp4Args = buildYtDlpArgs({
      url: 'https://www.youtube.com/watch?v=abc123',
      format: 'mp4',
      quality: '1080p',
      destinationDeviceId: 'pc-1',
      destinationFolder: '/tmp/yt',
    });
    expect(mp4Args).toContain('--no-playlist');
    expect(mp4Args).toContain('--embed-thumbnail');
    expect(mp4Args).toContain('bestvideo[height<=1080]+bestaudio/best[height<=1080]');

    const mp3Args = buildYtDlpArgs({
      url: 'https://www.youtube.com/playlist?list=PL99',
      format: 'mp3',
      quality: '320k',
      destinationDeviceId: 'pc-1',
      destinationFolder: '/tmp/music',
      allowPlaylist: true,
    });
    expect(mp3Args).toContain('--yes-playlist');
    expect(mp3Args).toContain('--extract-audio');
    expect(mp3Args).toContain('mp3');
  });

  it('downloads MP3 via runner, auto-imports into media index and unified music library (G1, G2, F5)', async () => {
    const dir = await mkdtemp(path.join(os.tmpdir(), 'mg-yt-'));
    tempDirs.push(dir);
    const outDir = path.join(dir, 'downloads');
    const cacheDir = path.join(dir, 'cache');

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

    const mockRunner: ProcessRunner = {
      async runDownload(_bin, _args, onProgress) {
        onProgress(50, 'Synthwave Study Mix');
        const outFile = path.join(outDir, 'Synthwave_Study_Mix.mp3');
        await writeFile(outFile, Buffer.from('fake-mp3-payload'));
        onProgress(100, 'Synthwave Study Mix');
        return {
          outputPaths: [outFile],
          title: 'Synthwave Study Mix',
          uploader: 'Lofi Channel',
        };
      },
      async runUpdate() {
        return 'yt-dlp is up to date';
      },
    };

    const yt = new YoutubeService(repo, mockRunner);
    expect(await yt.updateYtDlp()).toContain('up to date');

    const { job, importedMedia } = await yt.enqueueAndDownload(
      {
        url: 'https://www.youtube.com/watch?v=demo',
        format: 'mp3',
        quality: '192k',
        destinationDeviceId: 'pc-1',
        destinationFolder: outDir,
      },
      { deviceName: 'PC', cacheDir },
    );

    expect(job.status).toBe('completed');
    expect(job.progress).toBe(100);
    expect(importedMedia).toHaveLength(1);
    expect(importedMedia[0].tags.source).toBe('youtube');

    const audioItems = repo.queryMedia('', { type: 'audio' });
    const unifiedMusic = buildUnifiedMusicLibrary(audioItems);
    expect(unifiedMusic).toHaveLength(1);
    expect(unifiedMusic[0].title).toBe('Synthwave Study Mix');

    repo.close();
  });
});
