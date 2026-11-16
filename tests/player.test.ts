import { describe, expect, it } from 'vitest';
import { mapMediaStoreRowsToMediaRecords } from '../android-bridge/MediaGalleryPlugin.ts';
import type { MediaWithCover } from '../src/core/types/domain.ts';
import {
  buildUnifiedMusicLibrary,
  groupUnifiedTracks,
  MusicPlayerController,
} from '../src/features/player/musicPlayerService.ts';

function makeAudio(overrides: Partial<MediaWithCover>): MediaWithCover {
  return {
    id: overrides.id ?? 'a1',
    deviceId: overrides.deviceId ?? 'pc-local',
    deviceName: overrides.deviceName ?? 'PC',
    deviceType: overrides.deviceType ?? 'pc',
    path: overrides.path ?? 'C:/Music/song.mp3',
    folderPath: overrides.folderPath ?? 'C:/Music',
    fileName: overrides.fileName ?? 'song.mp3',
    type: 'audio',
    mime: 'audio/mpeg',
    size: overrides.size ?? 5_000_000,
    hash: overrides.hash ?? 'same-track-hash',
    createdAt: Date.UTC(2026, 2, 14),
    modifiedAt: Date.UTC(2026, 2, 14),
    durationMs: overrides.durationMs ?? 180_000,
    width: 0,
    height: 0,
    gpsLat: null,
    gpsLon: null,
    place: null,
    tags: overrides.tags ?? { artist: 'Nova Drift', title: 'Midnight Echoes', album: 'Neon Skies' },
    alias: 'Nova Drift - Midnight Echoes',
    autoAlias: 'Nova Drift - Midnight Echoes',
    coverId: null,
    syncState: overrides.syncState ?? 'Local',
  };
}

describe('Unified Music Player & Android MediaStore Bridge (M2)', () => {
  it('merges and de-duplicates PC and Phone tracks while keeping all source devices (F2)', () => {
    const pcTrack = makeAudio({
      id: 'pc-song',
      deviceId: 'pc-local',
      deviceName: 'Windows PC',
      syncState: 'Local',
      hash: 'hash-echoes',
    });
    const phoneDupTrack = makeAudio({
      id: 'phone-song',
      deviceId: 'phone-pixel',
      deviceName: 'Pixel 9 Pro',
      syncState: 'Remote',
      hash: 'hash-echoes',
    });
    const ytTrack = makeAudio({
      id: 'yt-song',
      deviceId: 'pc-local',
      deviceName: 'Windows PC',
      hash: 'hash-yt',
      tags: { artist: 'Kairo', title: 'Solar Flare', album: 'YouTube Downloads', source: 'youtube' },
    });

    const unified = buildUnifiedMusicLibrary([phoneDupTrack, pcTrack, ytTrack]);
    expect(unified).toHaveLength(2);
    expect(unified[0].sources).toHaveLength(2);
    expect(unified[0].primary.id).toBe('pc-song'); // Prefers local primary over remote

    const byArtist = groupUnifiedTracks(unified, 'artist');
    expect(byArtist).toHaveLength(2);
  });

  it('controls playback queue, seek, shuffle, repeat, and next/prev (F1)', () => {
    const tracks = buildUnifiedMusicLibrary([
      makeAudio({ id: 't1', hash: 'h1', tags: { artist: 'A', title: 'One' } }),
      makeAudio({ id: 't2', hash: 'h2', tags: { artist: 'B', title: 'Two' } }),
    ]);

    const player = new MusicPlayerController();
    player.playTrackFromList(tracks, 0);
    expect(player.getCurrentTrack()?.title).toBe('One');
    expect(player.getState().isPlaying).toBe(true);

    player.seek(45_000);
    expect(player.getState().positionMs).toBe(45_000);

    player.next();
    expect(player.getCurrentTrack()?.title).toBe('Two');

    player.cycleRepeat(); // off -> all
    player.next();
    expect(player.getCurrentTrack()?.title).toBe('One');

    player.togglePlayPause();
    expect(player.getState().isPlaying).toBe(false);
  });

  it('maps Android MediaStore cursor rows to MediaRecord entries with auto-alias', () => {
    const mapped = mapMediaStoreRowsToMediaRecords(
      [
        {
          uri: 'content://media/external/images/media/42',
          displayName: 'IMG_2031.jpg',
          relativePath: 'DCIM/Camera/',
          mimeType: 'image/jpeg',
          size: 3_500_000,
          dateAddedSec: Math.floor(Date.UTC(2026, 2, 14, 12, 0, 0) / 1000),
          dateModifiedSec: Math.floor(Date.UTC(2026, 2, 14, 12, 0, 0) / 1000),
          width: 4000,
          height: 3000,
          latitude: -8.7185,
          longitude: 115.1686,
        },
      ],
      'phone-pixel',
      'Pixel 9 Pro',
    );

    expect(mapped).toHaveLength(1);
    expect(mapped[0].folderPath).toBe('/storage/emulated/0/DCIM/Camera');
    expect(mapped[0].place).toBe('Kuta Beach Bali');
    expect(mapped[0].autoAlias).toBe('2026-03-14 Kuta Beach Bali');
  });
});
