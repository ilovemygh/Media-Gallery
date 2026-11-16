import React, { useState } from 'react';
import {
  ChevronUp,
  Download,
  Film,
  HardDrive,
  Home,
  Image as ImageIcon,
  Library,
  Music,
  Pause,
  Play,
  Search,
  Share2,
  UserCircle2,
  Wifi,
  X,
  Youtube,
} from 'lucide-react';
import type { MediaType, MediaWithCover } from '../../core/types/domain.ts';
import type { PlayerState } from '../../features/player/musicPlayerService.ts';
import { formatBytes, formatDuration } from './MainGalleryArea.tsx';

export interface AndroidMobileShellProps {
  searchQuery: string;
  onSearchChange: (q: string) => void;
  items: MediaWithCover[];
  playerState: PlayerState;
  googleAccountEmail: string | null;
  connectedPeerName: string | null;
  onTogglePlay: () => void;
  onPlayMedia: (item: MediaWithCover) => void;
  onDownloadRemote: (mediaId: string) => void;
  onOpenConnect: () => void;
  onOpenYouTube: () => void;
  onShareSheetSend: (item: MediaWithCover) => void;
}

type MobileTab = 'home' | 'library' | 'youtube' | 'devices';

/**
 * PRD Section 8 Android Mobile UI:
 * - Bottom navigation: Home, Library (Videos / Images / Audio tabs), YouTube, Devices
 * - Top bar: Search, Connect status chip, YouTube menu, Account avatar
 * - Details panel as a bottom sheet
 * - Mini player at the bottom, full-screen player on tap
 * - Android share-sheet integration ("Send with Media Gallery")
 */
export const AndroidMobileShell: React.FC<AndroidMobileShellProps> = ({
  searchQuery,
  onSearchChange,
  items,
  playerState,
  googleAccountEmail,
  connectedPeerName,
  onTogglePlay,
  onPlayMedia,
  onDownloadRemote,
  onOpenConnect,
  onOpenYouTube,
  onShareSheetSend,
}) => {
  const [tab, setTab] = useState<MobileTab>('library');
  const [libType, setLibType] = useState<MediaType>('image');
  const [sheetMedia, setSheetMedia] = useState<MediaWithCover | null>(null);
  const [fullPlayerOpen, setFullPlayerOpen] = useState(false);

  const visibleItems = items.filter((item) => {
    if (tab === 'library') return item.type === libType;
    if (tab === 'youtube') return item.tags?.source === 'youtube';
    return true;
  });

  const currentTrack = playerState.queue[playerState.currentIndex] ?? null;

  return (
    <div className="android-shell" aria-label="Android Mobile Preview">
      <header
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 8,
          padding: '10px 12px',
          background: 'var(--bg-surface)',
          borderBottom: '1px solid var(--border-subtle)',
        }}
      >
        <div className="search-box" style={{ height: 34 }}>
          <Search size={14} color="var(--text-muted)" />
          <input
            className="search-input"
            value={searchQuery}
            onChange={(e) => onSearchChange(e.target.value)}
            placeholder="Search..."
          />
        </div>
        <button type="button" className="crop-chip" onClick={onOpenConnect} title="Connect status">
          <Wifi size={14} color={connectedPeerName ? '#4ade80' : undefined} />
        </button>
        <button type="button" className="crop-chip" onClick={onOpenYouTube} title="YouTube download">
          <Youtube size={14} color="#ef4444" />
        </button>
        <span title={googleAccountEmail ?? 'Optional account'} style={{ display: 'inline-flex' }}>
          <UserCircle2 size={20} />
        </span>
      </header>

      {tab === 'library' && (
        <div
          style={{
            display: 'grid',
            gridTemplateColumns: 'repeat(3, 1fr)',
            gap: 6,
            padding: '8px 12px',
            background: 'var(--bg-surface)',
            borderBottom: '1px solid var(--border-subtle)',
          }}
        >
          {(['video', 'image', 'audio'] as MediaType[]).map((t) => (
            <button
              key={t}
              type="button"
              className={`crop-chip ${libType === t ? 'active' : ''}`}
              style={{ display: 'inline-flex', alignItems: 'center', justifyContent: 'center', gap: 5 }}
              onClick={() => setLibType(t)}
            >
              {t === 'video' ? <Film size={13} /> : t === 'image' ? <ImageIcon size={13} /> : <Music size={13} />}
              {t === 'video' ? 'Videos' : t === 'image' ? 'Images' : 'Audio'}
            </button>
          ))}
        </div>
      )}

      <div style={{ flex: 1, overflowY: 'auto', padding: 12 }}>
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 10 }}>
          {visibleItems.map((item) => (
            <div
              key={item.id}
              className="media-card"
              onClick={() => setSheetMedia(item)}
            >
              <div className="media-thumb-box">
                {item.cover?.filePath && <img src={item.cover.filePath} alt={item.alias} />}
                {item.durationMs > 0 && (
                  <span className="duration-badge">{formatDuration(item.durationMs)}</span>
                )}
              </div>
              <div className="media-card-body">
                <div className="media-title">{item.alias || item.fileName}</div>
                <div className="media-subtitle">{item.syncState}</div>
              </div>
            </div>
          ))}
        </div>
      </div>

      {currentTrack && (
        <div
          onClick={() => setFullPlayerOpen(true)}
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            padding: '8px 12px',
            background: 'var(--bg-elevated)',
            borderTop: '1px solid var(--border-strong)',
            cursor: 'pointer',
          }}
        >
          <div style={{ minWidth: 0 }}>
            <div className="media-title">{currentTrack.title}</div>
            <div className="media-subtitle">{currentTrack.artist}</div>
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
            <button
              type="button"
              className="btn-primary"
              onClick={(e) => {
                e.stopPropagation();
                onTogglePlay();
              }}
            >
              {playerState.isPlaying ? <Pause size={14} /> : <Play size={14} />}
            </button>
            <ChevronUp size={16} />
          </div>
        </div>
      )}

      <nav className="android-bottom-nav" aria-label="Android bottom navigation">
        {[
          { id: 'home' as MobileTab, label: 'Home', icon: <Home size={18} /> },
          { id: 'library' as MobileTab, label: 'Library', icon: <Library size={18} /> },
          { id: 'youtube' as MobileTab, label: 'YouTube', icon: <Youtube size={18} /> },
          { id: 'devices' as MobileTab, label: 'Devices', icon: <HardDrive size={18} /> },
        ].map((item) => (
          <button
            key={item.id}
            type="button"
            onClick={() => setTab(item.id)}
            style={{
              display: 'flex',
              flexDirection: 'column',
              alignItems: 'center',
              justifyContent: 'center',
              gap: 3,
              border: 'none',
              background: 'transparent',
              color: tab === item.id ? 'var(--accent)' : 'var(--text-secondary)',
              fontSize: 11,
              cursor: 'pointer',
            }}
          >
            {item.icon}
            <span>{item.label}</span>
          </button>
        ))}
      </nav>

      {sheetMedia && (
        <div className="android-bottom-sheet">
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 10 }}>
            <strong>{sheetMedia.alias || sheetMedia.fileName}</strong>
            <button type="button" className="crop-chip" onClick={() => setSheetMedia(null)}>
              <X size={14} />
            </button>
          </div>
          {sheetMedia.cover?.filePath && (
            <img
              src={sheetMedia.cover.filePath}
              alt={sheetMedia.alias}
              style={{ width: '100%', borderRadius: 10, maxHeight: 190, objectFit: 'cover' }}
            />
          )}
          <div style={{ marginTop: 10, fontSize: 12.5, color: 'var(--text-secondary)' }}>
            {sheetMedia.fileName} · {formatBytes(sheetMedia.size)} · {sheetMedia.syncState}
          </div>
          <div style={{ display: 'flex', gap: 8, marginTop: 12 }}>
            {sheetMedia.type === 'audio' && (
              <button
                type="button"
                className="btn-primary"
                onClick={() => onPlayMedia(sheetMedia)}
              >
                <Music size={14} /> Play
              </button>
            )}
            {sheetMedia.syncState === 'Remote' && (
              <button
                type="button"
                className="btn-primary"
                onClick={() => onDownloadRemote(sheetMedia.id)}
              >
                <Download size={14} /> Sync
              </button>
            )}
            <button
              type="button"
              className="btn-secondary"
              onClick={() => onShareSheetSend(sheetMedia)}
              title="Android Share Sheet: Send with Media Gallery"
            >
              <Share2 size={14} /> Send with Media Gallery
            </button>
          </div>
        </div>
      )}

      {fullPlayerOpen && currentTrack && (
        <div className="android-bottom-sheet" style={{ height: '90vh', maxHeight: '90vh' }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <strong>Now Playing</strong>
            <button type="button" className="crop-chip" onClick={() => setFullPlayerOpen(false)}>
              <X size={14} />
            </button>
          </div>
          {currentTrack.primary.cover?.filePath && (
            <img
              src={currentTrack.primary.cover.filePath}
              alt={currentTrack.title}
              style={{ width: '100%', borderRadius: 14, marginTop: 16 }}
            />
          )}
          <h3 style={{ marginBottom: 4 }}>{currentTrack.title}</h3>
          <div style={{ color: 'var(--text-secondary)', fontSize: 13 }}>{currentTrack.artist}</div>
          <div style={{ marginTop: 16 }}>
            <button type="button" className="btn-primary" onClick={onTogglePlay}>
              {playerState.isPlaying ? <Pause size={16} /> : <Play size={16} />} Toggle Play
            </button>
          </div>
        </div>
      )}
    </div>
  );
};
