import React, { useState } from 'react';
import {
  ChevronDown,
  Download,
  ExternalLink,
  Link2,
  LogOut,
  Search,
  SlidersHorizontal,
  SunMoon,
  Upload,
  UserCircle2,
  Wifi,
  Youtube,
} from 'lucide-react';
import type { DeviceRecord, TransportKind } from '../../core/types/domain.ts';
import type { YouTubeAction } from '../state/navigation.ts';

export interface TopBarProps {
  searchQuery: string;
  onSearchChange: (value: string) => void;
  connectedDevices: DeviceRecord[];
  activeTransport: TransportKind | null;
  googleAccountEmail: string | null;
  theme: 'dark' | 'light';
  showFilters: boolean;
  onToggleFilters: () => void;
  onOpenConnectModal: () => void;
  onYouTubeAction: (action: YouTubeAction) => void;
  onToggleGoogleAccount: () => void;
  onToggleTheme: () => void;
}

/**
 * PRD Section 7.3 Top Bar (placed beside the left sidebar):
 * Order left-to-right:
 *   Connect button/status chip -> Search bar -> YouTube dropdown -> Google account button
 */
export const TopBar: React.FC<TopBarProps> = ({
  searchQuery,
  onSearchChange,
  connectedDevices,
  activeTransport,
  googleAccountEmail,
  theme,
  showFilters,
  onToggleFilters,
  onOpenConnectModal,
  onYouTubeAction,
  onToggleGoogleAccount,
  onToggleTheme,
}) => {
  const [ytMenuOpen, setYtMenuOpen] = useState(false);

  const onlinePeers = connectedDevices.filter((d) => !d.isSelf && d.isOnline);
  const isConnected = onlinePeers.length > 0;

  return (
    <header className="top-bar">
      <button
        type="button"
        className={`connect-btn ${isConnected ? 'connected' : ''}`}
        onClick={onOpenConnectModal}
        title="Pair or manage connected devices"
      >
        {isConnected ? <Wifi size={16} /> : <Link2 size={16} />}
        <span>
          {isConnected
            ? `${onlinePeers[0].name} (${activeTransport ?? 'wifi_lan'})`
            : 'Connect'}
        </span>
      </button>

      <div className="search-box">
        <Search size={15} color="var(--text-muted)" />
        <input
          className="search-input"
          type="search"
          value={searchQuery}
          onChange={(event) => onSearchChange(event.target.value)}
          placeholder='Search by name, alias, date, place ("2026 beach")...'
          aria-label="Search media"
        />
        <button
          type="button"
          className="crop-chip"
          style={{
            borderColor: showFilters ? 'var(--accent)' : undefined,
            color: showFilters ? 'var(--accent)' : undefined,
          }}
          onClick={onToggleFilters}
          title="Toggle advanced search filters"
        >
          <SlidersHorizontal size={13} />
        </button>
      </div>

      <div className="dropdown-anchor">
        <button
          type="button"
          className="icon-pill-btn youtube"
          onClick={() => setYtMenuOpen((prev) => !prev)}
          aria-expanded={ytMenuOpen}
          title="YouTube actions"
        >
          <Youtube size={16} />
          <span>YouTube</span>
          <ChevronDown size={14} />
        </button>

        {ytMenuOpen && (
          <div className="dropdown-menu" role="menu">
            <button
              type="button"
              className="dropdown-item"
              onClick={() => {
                setYtMenuOpen(false);
                onYouTubeAction('download');
              }}
            >
              <span style={{ display: 'inline-flex', alignItems: 'center', gap: 8 }}>
                <Download size={15} /> Download from YouTube
              </span>
              <span className="badge-subtle">yt-dlp</span>
            </button>
            <button
              type="button"
              className="dropdown-item"
              onClick={() => {
                setYtMenuOpen(false);
                onYouTubeAction('upload');
              }}
            >
              <span style={{ display: 'inline-flex', alignItems: 'center', gap: 8 }}>
                <Upload size={15} /> Upload
              </span>
              {!googleAccountEmail && <span className="badge-subtle">Sign in</span>}
            </button>
            <button
              type="button"
              className="dropdown-item"
              onClick={() => {
                setYtMenuOpen(false);
                onYouTubeAction('channel');
              }}
            >
              <span style={{ display: 'inline-flex', alignItems: 'center', gap: 8 }}>
                <ExternalLink size={15} /> Your channel
              </span>
              {!googleAccountEmail && <span className="badge-subtle">Sign in</span>}
            </button>
            <button
              type="button"
              className="dropdown-item"
              onClick={() => {
                setYtMenuOpen(false);
                onYouTubeAction('studio');
              }}
            >
              <span style={{ display: 'inline-flex', alignItems: 'center', gap: 8 }}>
                <ExternalLink size={15} /> YouTube Studio
              </span>
              {!googleAccountEmail && <span className="badge-subtle">Sign in</span>}
            </button>
          </div>
        )}
      </div>

      <button
        type="button"
        className="icon-pill-btn"
        onClick={onToggleTheme}
        title={`Switch to ${theme === 'dark' ? 'light' : 'dark'} theme`}
      >
        <SunMoon size={16} />
      </button>

      <button
        type="button"
        className="icon-pill-btn"
        onClick={onToggleGoogleAccount}
        title={googleAccountEmail ? `Signed in as ${googleAccountEmail} (click to sign out)` : 'Optional Google sign-in for cloud metadata sync & YouTube'}
      >
        {googleAccountEmail ? <LogOut size={15} /> : <UserCircle2 size={16} />}
        <span>{googleAccountEmail ? googleAccountEmail.split('@')[0] : 'Google'}</span>
      </button>
    </header>
  );
};
