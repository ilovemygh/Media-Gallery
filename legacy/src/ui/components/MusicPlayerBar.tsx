import React, { useEffect } from 'react';
import {
  Pause,
  Play,
  Repeat,
  Repeat1,
  Shuffle,
  SkipBack,
  SkipForward,
  Volume2,
} from 'lucide-react';
import type { PlayerState } from '../../features/player/musicPlayerService.ts';
import { formatDuration } from './MainGalleryArea.tsx';

export interface MusicPlayerBarProps {
  playerState: PlayerState;
  onTogglePlay: () => void;
  onNext: () => void;
  onPrev: () => void;
  onSeek: (positionMs: number) => void;
  onToggleShuffle: () => void;
  onCycleRepeat: () => void;
  onVolumeChange: (volume: number) => void;
}

/**
 * Persistent unified audio player bar with MediaSession hardware media-key support (PRD F1, F4).
 */
export const MusicPlayerBar: React.FC<MusicPlayerBarProps> = ({
  playerState,
  onTogglePlay,
  onNext,
  onPrev,
  onSeek,
  onToggleShuffle,
  onCycleRepeat,
  onVolumeChange,
}) => {
  const track = playerState.queue[playerState.currentIndex] ?? null;

  useEffect(() => {
    if (!track || typeof navigator === 'undefined' || !('mediaSession' in navigator)) return;
    try {
      navigator.mediaSession.metadata = new MediaMetadata({
        title: track.title,
        artist: track.artist,
        album: track.album,
      });
      navigator.mediaSession.setActionHandler('play', onTogglePlay);
      navigator.mediaSession.setActionHandler('pause', onTogglePlay);
      navigator.mediaSession.setActionHandler('previoustrack', onPrev);
      navigator.mediaSession.setActionHandler('nexttrack', onNext);
    } catch {
      // Ignore in environments where MediaSession is partially implemented.
    }
  }, [track, onTogglePlay, onNext, onPrev]);

  if (!track) return null;

  return (
    <div className="music-bar" aria-label="Music player">
      <div style={{ display: 'flex', alignItems: 'center', gap: 10, minWidth: 200 }}>
        {track.primary.cover?.filePath && (
          <img
            src={track.primary.cover.filePath}
            alt={track.title}
            style={{ width: 42, height: 42, borderRadius: 6, objectFit: 'cover' }}
          />
        )}
        <div style={{ minWidth: 0 }}>
          <div className="media-title">{track.title}</div>
          <div className="media-subtitle">
            {track.artist} · {track.sources.map((s) => s.deviceName).join(' & ')}
          </div>
        </div>
      </div>

      <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', flex: 1, maxWidth: 460, gap: 4 }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
          <button
            type="button"
            className={`crop-chip ${playerState.shuffle ? 'active' : ''}`}
            onClick={onToggleShuffle}
            title="Shuffle"
          >
            <Shuffle size={14} />
          </button>
          <button type="button" className="crop-chip" onClick={onPrev} title="Previous track">
            <SkipBack size={15} />
          </button>
          <button type="button" className="btn-primary" onClick={onTogglePlay} title="Play / Pause">
            {playerState.isPlaying ? <Pause size={15} /> : <Play size={15} />}
          </button>
          <button type="button" className="crop-chip" onClick={onNext} title="Next track">
            <SkipForward size={15} />
          </button>
          <button
            type="button"
            className={`crop-chip ${playerState.repeat !== 'off' ? 'active' : ''}`}
            onClick={onCycleRepeat}
            title={`Repeat: ${playerState.repeat}`}
          >
            {playerState.repeat === 'one' ? <Repeat1 size={14} /> : <Repeat size={14} />}
          </button>
        </div>

        <div style={{ display: 'flex', alignItems: 'center', gap: 8, width: '100%' }}>
          <span style={{ fontSize: 11, color: 'var(--text-muted)' }}>
            {formatDuration(playerState.positionMs)}
          </span>
          <input
            type="range"
            min={0}
            max={Math.max(1000, track.durationMs)}
            value={playerState.positionMs}
            onChange={(e) => onSeek(Number(e.target.value))}
            style={{ flex: 1 }}
            aria-label="Seek track position"
          />
          <span style={{ fontSize: 11, color: 'var(--text-muted)' }}>
            {formatDuration(track.durationMs)}
          </span>
        </div>
      </div>

      <div style={{ display: 'flex', alignItems: 'center', gap: 8, width: 130 }}>
        <Volume2 size={15} color="var(--text-secondary)" />
        <input
          type="range"
          min={0}
          max={1}
          step={0.05}
          value={playerState.volume}
          onChange={(e) => onVolumeChange(Number(e.target.value))}
          style={{ width: 90 }}
          aria-label="Volume"
        />
      </div>
    </div>
  );
};
