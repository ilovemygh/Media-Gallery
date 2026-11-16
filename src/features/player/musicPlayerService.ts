import type { MediaWithCover, SyncState } from '../../core/types/domain.ts';

export type RepeatMode = 'off' | 'one' | 'all';
export type MusicGroupBy = 'all' | 'artist' | 'album' | 'folder' | 'device';

export interface UnifiedTrackSource {
  mediaId: string;
  deviceId: string;
  deviceName: string;
  syncState: SyncState;
  path: string;
}

export interface UnifiedMusicTrack {
  canonicalKey: string;
  primary: MediaWithCover;
  title: string;
  artist: string;
  album: string;
  durationMs: number;
  sources: UnifiedTrackSource[];
}

export interface PlaylistRecord {
  id: string;
  name: string;
  trackKeys: string[];
  createdAt: number;
}

export interface PlayerState {
  queue: UnifiedMusicTrack[];
  currentIndex: number;
  isPlaying: boolean;
  positionMs: number;
  volume: number;
  shuffle: boolean;
  repeat: RepeatMode;
}

function buildCanonicalTrackKey(track: MediaWithCover): string {
  if (track.hash && !track.hash.startsWith('sha256-seed-')) {
    return `hash:${track.hash}`;
  }
  const artist = String(track.tags?.artist ?? '').trim().toLowerCase();
  const title = String(track.tags?.title ?? track.alias ?? track.fileName)
    .trim()
    .toLowerCase();
  const durBucket = Math.round((track.durationMs || 0) / 2000);
  if (artist && title) {
    return `meta:${artist}::${title}::${durBucket}`;
  }
  return `file:${track.fileName.toLowerCase()}::${track.size}`;
}

/**
 * Merges audio tracks from PC, Phone, and YouTube downloads into a single
 * de-duplicated library while preserving source device attribution (PRD F2, F5).
 */
export function buildUnifiedMusicLibrary(audioItems: MediaWithCover[]): UnifiedMusicTrack[] {
  const byKey = new Map<string, UnifiedMusicTrack>();

  for (const item of audioItems) {
    if (item.type !== 'audio' || item.isTrashed) continue;
    const key = buildCanonicalTrackKey(item);
    const source: UnifiedTrackSource = {
      mediaId: item.id,
      deviceId: item.deviceId,
      deviceName: item.deviceName ?? item.deviceId,
      syncState: item.syncState,
      path: item.path,
    };

    const existing = byKey.get(key);
    if (existing) {
      existing.sources.push(source);
      if (existing.primary.syncState === 'Remote' && item.syncState !== 'Remote') {
        existing.primary = item;
      }
    } else {
      byKey.set(key, {
        canonicalKey: key,
        primary: item,
        title: String(item.tags?.title ?? item.alias ?? item.fileName),
        artist: String(item.tags?.artist ?? 'Unknown Artist'),
        album: String(item.tags?.album ?? 'Singles & Downloads'),
        durationMs: item.durationMs,
        sources: [source],
      });
    }
  }

  return Array.from(byKey.values());
}

export function groupUnifiedTracks(
  tracks: UnifiedMusicTrack[],
  groupBy: MusicGroupBy,
): Array<{ groupName: string; tracks: UnifiedMusicTrack[] }> {
  if (groupBy === 'all') {
    return [{ groupName: 'All Tracks', tracks }];
  }
  const buckets = new Map<string, UnifiedMusicTrack[]>();
  for (const track of tracks) {
    const key =
      groupBy === 'artist'
        ? track.artist
        : groupBy === 'album'
          ? track.album
          : groupBy === 'folder'
            ? track.primary.folderPath
            : track.sources.map((s) => s.deviceName).join(' + ');
    const list = buckets.get(key) ?? [];
    list.push(track);
    buckets.set(key, list);
  }
  return Array.from(buckets.entries()).map(([groupName, list]) => ({
    groupName,
    tracks: list,
  }));
}

/**
 * Pure state machine for audio playback controls (PRD F1).
 */
export class MusicPlayerController {
  private state: PlayerState = {
    queue: [],
    currentIndex: -1,
    isPlaying: false,
    positionMs: 0,
    volume: 0.85,
    shuffle: false,
    repeat: 'off',
  };

  getState(): PlayerState {
    return { ...this.state };
  }

  getCurrentTrack(): UnifiedMusicTrack | null {
    return this.state.queue[this.state.currentIndex] ?? null;
  }

  playTrackFromList(tracks: UnifiedMusicTrack[], startIndex: number): PlayerState {
    this.state = {
      ...this.state,
      queue: [...tracks],
      currentIndex: Math.max(0, Math.min(tracks.length - 1, startIndex)),
      isPlaying: tracks.length > 0,
      positionMs: 0,
    };
    return this.getState();
  }

  togglePlayPause(): PlayerState {
    if (this.state.queue.length === 0) return this.getState();
    this.state = { ...this.state, isPlaying: !this.state.isPlaying };
    return this.getState();
  }

  seek(positionMs: number): PlayerState {
    const track = this.getCurrentTrack();
    const max = track?.durationMs ?? positionMs;
    this.state = { ...this.state, positionMs: Math.max(0, Math.min(max, positionMs)) };
    return this.getState();
  }

  setVolume(volume: number): PlayerState {
    this.state = { ...this.state, volume: Math.max(0, Math.min(1, volume)) };
    return this.getState();
  }

  toggleShuffle(): PlayerState {
    this.state = { ...this.state, shuffle: !this.state.shuffle };
    return this.getState();
  }

  cycleRepeat(): PlayerState {
    const next: RepeatMode =
      this.state.repeat === 'off' ? 'all' : this.state.repeat === 'all' ? 'one' : 'off';
    this.state = { ...this.state, repeat: next };
    return this.getState();
  }

  next(rng: () => number = Math.random): PlayerState {
    const len = this.state.queue.length;
    if (len === 0) return this.getState();
    if (this.state.repeat === 'one') {
      this.state = { ...this.state, positionMs: 0, isPlaying: true };
      return this.getState();
    }
    if (this.state.shuffle && len > 1) {
      let randIdx = Math.floor(rng() * len);
      if (randIdx === this.state.currentIndex) randIdx = (randIdx + 1) % len;
      this.state = { ...this.state, currentIndex: randIdx, positionMs: 0, isPlaying: true };
      return this.getState();
    }
    const nextIdx = this.state.currentIndex + 1;
    if (nextIdx >= len) {
      if (this.state.repeat === 'all') {
        this.state = { ...this.state, currentIndex: 0, positionMs: 0, isPlaying: true };
      } else {
        this.state = { ...this.state, isPlaying: false, positionMs: 0 };
      }
      return this.getState();
    }
    this.state = { ...this.state, currentIndex: nextIdx, positionMs: 0, isPlaying: true };
    return this.getState();
  }

  previous(): PlayerState {
    const len = this.state.queue.length;
    if (len === 0) return this.getState();
    if (this.state.positionMs > 3000) {
      this.state = { ...this.state, positionMs: 0 };
      return this.getState();
    }
    const prevIdx = this.state.currentIndex > 0 ? this.state.currentIndex - 1 : len - 1;
    this.state = { ...this.state, currentIndex: prevIdx, positionMs: 0, isPlaying: true };
    return this.getState();
  }
}
