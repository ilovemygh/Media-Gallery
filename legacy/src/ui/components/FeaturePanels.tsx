import React from 'react';
import {
  Copy,
  Download,
  ExternalLink,
  HardDrive,
  Music,
  Pause,
  Play,
  RefreshCw,
  Smartphone,
  Trash2,
  X,
  Youtube,
} from 'lucide-react';
import type {
  AppSettings,
  AutoSyncRuleRecord,
  DeviceRecord,
  DownloadJobRecord,
  MediaWithCover,
  TransferRecord,
  TransportKind,
} from '../../core/types/domain.ts';
import {
  groupUnifiedTracks,
  type MusicGroupBy,
  type UnifiedMusicTrack,
} from '../../features/player/musicPlayerService.ts';
import { YOUTUBE_PORTAL_URLS } from '../../features/youtube/youtubeConstants.ts';
import { formatBytes, formatDuration } from './MainGalleryArea.tsx';

export const UnifiedAudioView: React.FC<{
  tracks: UnifiedMusicTrack[];
  groupBy: MusicGroupBy;
  onChangeGroupBy: (g: MusicGroupBy) => void;
  onPlayTrack: (tracks: UnifiedMusicTrack[], index: number) => void;
}> = ({ tracks, groupBy, onChangeGroupBy, onPlayTrack }) => {
  const groups = groupUnifiedTracks(tracks, groupBy);

  return (
    <main className="main-content">
      <div className="toolbar-row">
        <div className="toolbar-group">
          <h2 style={{ margin: 0, fontSize: 18 }}>Unified Music Library (PC + Phone)</h2>
          <span className="badge-subtle">{tracks.length} de-duplicated tracks</span>
        </div>
        <select
          className="select-control"
          value={groupBy}
          onChange={(e) => onChangeGroupBy(e.target.value as MusicGroupBy)}
        >
          <option value="all">Group: All Tracks</option>
          <option value="artist">Group: Artist</option>
          <option value="album">Group: Album</option>
          <option value="folder">Group: Folder</option>
          <option value="device">Group: Device</option>
        </select>
      </div>

      {groups.map((grp) => (
        <section key={grp.groupName}>
          <div className="sidebar-section-label" style={{ paddingLeft: 0, marginBottom: 8 }}>
            {grp.groupName} ({grp.tracks.length})
          </div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
            {grp.tracks.map((track, idx) => (
              <div
                key={track.canonicalKey}
                className="folder-card"
                style={{
                  flexDirection: 'row',
                  alignItems: 'center',
                  justifyContent: 'space-between',
                }}
              >
                <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
                  <img
                    src={track.primary.cover?.filePath}
                    alt={track.title}
                    style={{ width: 46, height: 46, borderRadius: 6, objectFit: 'cover' }}
                  />
                  <div>
                    <div className="media-title">{track.title}</div>
                    <div className="media-subtitle">
                      {track.artist} · {track.album} ·{' '}
                      {track.sources.map((s) => s.deviceName).join(' + ')}
                    </div>
                  </div>
                </div>
                <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
                  <span className="badge-subtle">{formatDuration(track.durationMs)}</span>
                  <button
                    type="button"
                    className="btn-primary"
                    onClick={() => onPlayTrack(grp.tracks, idx)}
                  >
                    <Play size={13} /> Play
                  </button>
                </div>
              </div>
            ))}
          </div>
        </section>
      ))}
    </main>
  );
};

export const DevicesAndTransfersView: React.FC<{
  devices: DeviceRecord[];
  transfers: TransferRecord[];
  autoSyncRules: AutoSyncRuleRecord[];
  onOpenConnect: () => void;
  onDropMediaToDevice: (mediaId: string, deviceId: string) => void;
  onToggleTransferPause: (id: string) => void;
  onCancelTransfer: (id: string) => void;
  onToggleSyncRule: (ruleId: string) => void;
}> = ({
  devices,
  transfers,
  autoSyncRules,
  onOpenConnect,
  onDropMediaToDevice,
  onToggleTransferPause,
  onCancelTransfer,
  onToggleSyncRule,
}) => (
  <main className="main-content">
    <div className="toolbar-row">
      <h2 style={{ margin: 0, fontSize: 18 }}>Devices, Transfers &amp; Auto-Sync Rules</h2>
      <button type="button" className="btn-primary" onClick={onOpenConnect}>
        <HardDrive size={14} /> Pair / Connect Device
      </button>
    </div>

    <div className="folder-grid">
      {devices.map((dev) => (
        <div
          key={dev.id}
          className="folder-card"
          onDragOver={(e) => e.preventDefault()}
          onDrop={(e) => {
            e.preventDefault();
            const mediaId = e.dataTransfer.getData('text/media-id');
            if (mediaId && !dev.isSelf) {
              onDropMediaToDevice(mediaId, dev.id);
            }
          }}
        >
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <strong style={{ display: 'inline-flex', alignItems: 'center', gap: 6 }}>
              <Smartphone size={15} /> {dev.name}
            </strong>
            <span className="badge-subtle">{dev.isOnline ? 'Online' : 'Offline'}</span>
          </div>
          <div style={{ fontSize: 12, color: 'var(--text-secondary)' }}>
            Transports: {dev.transports.join(', ')}
          </div>
          {!dev.isSelf && (
            <div style={{ fontSize: 11.5, color: 'var(--accent)' }}>
              Drop media files here to send directly (PRD J4)
            </div>
          )}
        </div>
      ))}
    </div>

    <section>
      <div className="sidebar-section-label" style={{ paddingLeft: 0, marginBottom: 8 }}>
        Transfer Queue ({transfers.length})
      </div>
      {transfers.length === 0 ? (
        <div style={{ color: 'var(--text-secondary)', fontSize: 13 }}>
          No active transfers. Send a file or drag onto a device chip above.
        </div>
      ) : (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
          {transfers.map((tx) => {
            const pct = tx.bytesTotal > 0 ? Math.round((tx.bytesDone / tx.bytesTotal) * 100) : 100;
            return (
              <div
                key={tx.id}
                className="folder-card"
                style={{ flexDirection: 'row', justifyContent: 'space-between', alignItems: 'center' }}
              >
                <div>
                  <div className="media-title">{tx.fileName}</div>
                  <div className="media-subtitle">
                    {tx.transport} · {tx.status} · {pct}% ({formatBytes(tx.bytesDone)} / {formatBytes(tx.bytesTotal)})
                  </div>
                </div>
                <div style={{ display: 'flex', gap: 6 }}>
                  {tx.status !== 'completed' && tx.status !== 'cancelled' && (
                    <>
                      <button
                        type="button"
                        className="btn-secondary"
                        onClick={() => onToggleTransferPause(tx.id)}
                      >
                        {tx.status === 'paused' ? <Play size={13} /> : <Pause size={13} />}
                      </button>
                      <button
                        type="button"
                        className="btn-danger"
                        onClick={() => onCancelTransfer(tx.id)}
                      >
                        <X size={13} />
                      </button>
                    </>
                  )}
                </div>
              </div>
            );
          })}
        </div>
      )}
    </section>

    <section>
      <div className="sidebar-section-label" style={{ paddingLeft: 0, marginBottom: 8 }}>
        Auto-Sync Rules (PRD E3)
      </div>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
        {autoSyncRules.map((rule) => (
          <label
            key={rule.id}
            className="folder-card"
            style={{ flexDirection: 'row', justifyContent: 'space-between', alignItems: 'center' }}
          >
            <span>
              <strong>{rule.name}</strong> ({rule.folderPattern} · {rule.mediaTypes.join(', ')})
            </span>
            <input
              type="checkbox"
              checked={rule.enabled}
              onChange={() => onToggleSyncRule(rule.id)}
            />
          </label>
        ))}
      </div>
    </section>
  </main>
);

export const YouTubeWorkspaceView: React.FC<{
  jobs: DownloadJobRecord[];
  googleAccountEmail: string | null;
  onOpenDownloadModal: () => void;
  onUpdateYtDlp: () => void;
}> = ({ jobs, googleAccountEmail, onOpenDownloadModal, onUpdateYtDlp }) => (
  <main className="main-content">
    <div className="toolbar-row">
      <h2 style={{ margin: 0, fontSize: 18 }}>YouTube Downloader &amp; Channel</h2>
      <div className="toolbar-group">
        <button type="button" className="btn-primary" onClick={onOpenDownloadModal}>
          <Download size={14} /> Download Video / MP3
        </button>
        <button type="button" className="btn-secondary" onClick={onUpdateYtDlp}>
          <RefreshCw size={14} /> Update yt-dlp
        </button>
        <a
          className="btn-secondary"
          href={YOUTUBE_PORTAL_URLS.channel}
          target="_blank"
          rel="noreferrer"
          style={{ textDecoration: 'none' }}
        >
          <ExternalLink size={14} /> My Channel {googleAccountEmail ? '' : '(Sign in)'}
        </a>
        <a
          className="btn-secondary"
          href={YOUTUBE_PORTAL_URLS.studio}
          target="_blank"
          rel="noreferrer"
          style={{ textDecoration: 'none' }}
        >
          <Youtube size={14} /> YouTube Studio
        </a>
      </div>
    </div>

    <div style={{ fontSize: 12.5, color: 'var(--text-secondary)' }}>
      Downloaded MP3 tracks are automatically tagged with cover art and merged into your Unified Music Library (PRD F5, G2).
    </div>

    <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
      {jobs.length === 0 ? (
        <div className="folder-card">No downloads yet. Click &quot;Download Video / MP3&quot; to begin.</div>
      ) : (
        jobs.map((job) => (
          <div
            key={job.id}
            className="folder-card"
            style={{ flexDirection: 'row', justifyContent: 'space-between', alignItems: 'center' }}
          >
            <div>
              <div className="media-title">{job.title ?? job.url}</div>
              <div className="media-subtitle">
                {job.format.toUpperCase()} ({job.quality}) → {job.destinationFolder}
              </div>
            </div>
            <span className="badge-subtle">
              {job.status} ({job.progress}%)
            </span>
          </div>
        ))
      )}
    </div>
  </main>
);

export const DuplicatesView: React.FC<{
  items: MediaWithCover[];
  onTrashItem: (id: string) => void;
}> = ({ items, onTrashItem }) => {
  const byFingerprint = new Map<string, MediaWithCover[]>();
  for (const item of items) {
    if (item.isTrashed) continue;
    const key = `${item.size}:${item.partialHash ?? item.fileName}`;
    const list = byFingerprint.get(key) ?? [];
    list.push(item);
    byFingerprint.set(key, list);
  }
  const dupGroups = Array.from(byFingerprint.values()).filter((g) => g.length > 1);

  return (
    <main className="main-content">
      <div className="toolbar-row">
        <h2 style={{ margin: 0, fontSize: 18 }}>
          <Copy size={18} style={{ verticalAlign: 'middle', marginRight: 8 }} />
          Duplicate Detection (Flag Only — Never Auto-Deleted)
        </h2>
        <span className="badge-subtle">{dupGroups.length} duplicate group(s)</span>
      </div>
      {dupGroups.length === 0 ? (
        <div className="folder-card">No exact size + partial-hash duplicates detected across your library.</div>
      ) : (
        dupGroups.map((group, idx) => (
          <div key={idx} className="folder-card">
            <strong>
              Group #{idx + 1} · {formatBytes(group[0].size)}
            </strong>
            {group.map((item) => (
              <div
                key={item.id}
                style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginTop: 6 }}
              >
                <span style={{ fontSize: 12.5 }}>
                  {item.path} ({item.deviceName ?? item.deviceId})
                </span>
                <button
                  type="button"
                  className="btn-danger"
                  onClick={() => onTrashItem(item.id)}
                >
                  <Trash2 size={13} /> Move to Trash
                </button>
              </div>
            ))}
          </div>
        ))
      )}
    </main>
  );
};

export const SettingsView: React.FC<{
  settings: AppSettings;
  androidPreviewMode: boolean;
  onUpdateSettings: (patch: Partial<AppSettings>) => void;
  onToggleAndroidPreview: () => void;
  onResetDemoData: () => void;
}> = ({ settings, androidPreviewMode, onUpdateSettings, onToggleAndroidPreview, onResetDemoData }) => (
  <main className="main-content">
    <h2 style={{ margin: 0, fontSize: 18 }}>Settings &amp; Preferences</h2>

    <div className="folder-card" style={{ maxWidth: 640, gap: 12 }}>
      <label style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <span>Theme</span>
        <select
          className="select-control"
          value={settings.theme}
          onChange={(e) => onUpdateSettings({ theme: e.target.value as AppSettings['theme'] })}
        >
          <option value="dark">Dark</option>
          <option value="light">Light</option>
        </select>
      </label>

      <label style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <span>Auto-alias Template</span>
        <input
          className="text-control"
          style={{ width: 250 }}
          value={settings.aliasTemplate}
          onChange={(e) => onUpdateSettings({ aliasTemplate: e.target.value })}
        />
      </label>

      <label style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <span>Transport Selection Override</span>
        <select
          className="select-control"
          value={settings.manualTransportOverride}
          onChange={(e) =>
            onUpdateSettings({ manualTransportOverride: e.target.value as TransportKind | 'auto' })
          }
        >
          <option value="auto">Auto (Wired &gt; Wi-Fi LAN &gt; Wi-Fi Direct &gt; Bluetooth)</option>
          <option value="wired">Wired USB</option>
          <option value="wifi_lan">Wi-Fi LAN</option>
          <option value="bluetooth">Bluetooth (Slow fallback)</option>
        </select>
      </label>

      <label style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <span>Online Reverse Geocoding (Privacy opt-in)</span>
        <input
          type="checkbox"
          checked={settings.onlineGeocodingEnabled}
          onChange={(e) => onUpdateSettings({ onlineGeocodingEnabled: e.target.checked })}
        />
      </label>

      <div style={{ display: 'flex', gap: 10, marginTop: 6 }}>
        <button type="button" className="btn-primary" onClick={onToggleAndroidPreview}>
          <Music size={14} /> {androidPreviewMode ? 'Switch to Desktop Layout' : 'Preview Android Mobile UI'}
        </button>
        <button type="button" className="btn-secondary" onClick={onResetDemoData}>
          Reset Demo Library
        </button>
      </div>
    </div>
  </main>
);
