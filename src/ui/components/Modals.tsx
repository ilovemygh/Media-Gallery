import React, { useMemo, useState } from 'react';
import { QrCode, ShieldCheck, Smartphone, Wifi, X, Youtube } from 'lucide-react';
import type {
  DeviceRecord,
  DownloadFormat,
  MediaWithCover,
  TransportKind,
} from '../../core/types/domain.ts';
import { previewBulkPattern } from '../state/browserEngine.ts';

export const MediaViewerModal: React.FC<{
  media: MediaWithCover | null;
  onClose: () => void;
}> = ({ media, onClose }) => {
  if (!media) return null;
  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div
        className="modal-panel"
        style={{ maxWidth: 820 }}
        onClick={(e) => e.stopPropagation()}
      >
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <strong>{media.alias || media.fileName}</strong>
          <button type="button" className="crop-chip" onClick={onClose}>
            <X size={14} />
          </button>
        </div>

        {media.type === 'image' && (
          <img
            src={media.cover?.filePath}
            alt={media.alias || media.fileName}
            style={{ width: '100%', maxHeight: '65vh', objectFit: 'contain', borderRadius: 8 }}
          />
        )}

        {media.type === 'video' && (
          <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
            <video
              controls
              poster={media.cover?.filePath}
              style={{ width: '100%', maxHeight: '60vh', borderRadius: 8, background: '#000' }}
            />
            <span style={{ fontSize: 12, color: 'var(--text-secondary)' }}>
              Fixed thumbnail frame: {(media.cover?.frameTimeMs ?? 0) / 1000}s · Source: {media.path}
            </span>
          </div>
        )}
      </div>
    </div>
  );
};

export const BulkRenameModal: React.FC<{
  open: boolean;
  items: MediaWithCover[];
  onClose: () => void;
  onApply: (template: string) => void;
}> = ({ open, items, onClose, onApply }) => {
  const [template, setTemplate] = useState('{date} {place} {name} {index}');
  const previews = useMemo(() => previewBulkPattern(items, template), [items, template]);

  if (!open) return null;

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div className="modal-panel" onClick={(e) => e.stopPropagation()}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <strong>Bulk Rename ({items.length} items)</strong>
          <button type="button" className="crop-chip" onClick={onClose}>
            <X size={14} />
          </button>
        </div>

        <label style={{ fontSize: 12.5, color: 'var(--text-secondary)' }}>
          Pattern tokens: {'{date}'}, {'{place}'}, {'{name}'}, {'{alias}'}, {'{index}'}
        </label>
        <input
          className="text-control"
          value={template}
          onChange={(e) => setTemplate(e.target.value)}
        />

        <div style={{ maxHeight: 240, overflowY: 'auto', display: 'flex', flexDirection: 'column', gap: 6 }}>
          {previews.map((p) => (
            <div
              key={p.id}
              style={{
                display: 'flex',
                justifyContent: 'space-between',
                gap: 10,
                fontSize: 12,
                padding: '6px 8px',
                borderRadius: 6,
                background: 'var(--bg-elevated)',
              }}
            >
              <span style={{ color: 'var(--text-muted)' }}>{p.oldName}</span>
              <span>→ {p.newName}</span>
            </div>
          ))}
        </div>

        <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 8 }}>
          <button type="button" className="btn-secondary" onClick={onClose}>
            Cancel
          </button>
          <button
            type="button"
            className="btn-primary"
            onClick={() => {
              onApply(template);
              onClose();
            }}
          >
            Apply Bulk Rename
          </button>
        </div>
      </div>
    </div>
  );
};

export const ConnectPairModal: React.FC<{
  open: boolean;
  devices: DeviceRecord[];
  manualOverride: TransportKind | 'auto';
  onClose: () => void;
  onPairPin: (pin: string, deviceName: string) => void;
  onToggleOnline: (deviceId: string) => void;
  onForgetDevice: (deviceId: string) => void;
  onChangeTransportOverride: (override: TransportKind | 'auto') => void;
}> = ({
  open,
  devices,
  manualOverride,
  onClose,
  onPairPin,
  onToggleOnline,
  onForgetDevice,
  onChangeTransportOverride,
}) => {
  const [pin] = useState(() => String(Math.floor(100000 + Math.random() * 900000)));
  const [newDeviceName, setNewDeviceName] = useState('Galaxy S25');
  const [enteredPin, setEnteredPin] = useState('');

  if (!open) return null;

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div className="modal-panel" onClick={(e) => e.stopPropagation()}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <strong style={{ display: 'inline-flex', alignItems: 'center', gap: 8 }}>
            <Wifi size={17} /> Pair &amp; Connect Devices (No Cloud Required)
          </strong>
          <button type="button" className="crop-chip" onClick={onClose}>
            <X size={14} />
          </button>
        </div>

        <div
          style={{
            display: 'grid',
            gridTemplateColumns: '130px 1fr',
            gap: 14,
            padding: 14,
            borderRadius: 10,
            background: 'var(--bg-elevated)',
          }}
        >
          <div
            style={{
              display: 'flex',
              flexDirection: 'column',
              alignItems: 'center',
              justifyContent: 'center',
              padding: 10,
              borderRadius: 8,
              background: '#fff',
              color: '#0f172a',
            }}
          >
            <QrCode size={76} />
            <span style={{ fontSize: 10, fontWeight: 700, marginTop: 4 }}>TLS Mutual Auth</span>
          </div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
            <div style={{ fontSize: 12.5, color: 'var(--text-secondary)' }}>
              Scan QR or enter this one-time 6-digit PIN on your phone:
            </div>
            <div style={{ fontSize: 26, fontWeight: 800, letterSpacing: '0.18em' }}>{pin}</div>
            <div style={{ display: 'flex', alignItems: 'center', gap: 6, fontSize: 11.5, color: 'var(--success)' }}>
              <ShieldCheck size={14} /> Ed25519 certificate pinning active · LAN offline-first
            </div>
          </div>
        </div>

        <div style={{ display: 'flex', gap: 8 }}>
          <input
            className="text-control"
            style={{ flex: 1 }}
            value={newDeviceName}
            onChange={(e) => setNewDeviceName(e.target.value)}
            placeholder="Phone name"
          />
          <input
            className="text-control"
            style={{ width: 110 }}
            value={enteredPin}
            onChange={(e) => setEnteredPin(e.target.value)}
            placeholder={pin}
          />
          <button
            type="button"
            className="btn-primary"
            onClick={() => onPairPin(enteredPin || pin, newDeviceName)}
          >
            Pair Device
          </button>
        </div>

        <div>
          <label style={{ fontSize: 12, color: 'var(--text-secondary)' }}>
            Transport Selection (Auto priority: Wired USB &gt; Wi-Fi LAN &gt; Wi-Fi Direct &gt; Bluetooth)
          </label>
          <select
            className="select-control"
            style={{ width: '100%', marginTop: 6 }}
            value={manualOverride}
            onChange={(e) => onChangeTransportOverride(e.target.value as TransportKind | 'auto')}
          >
            <option value="auto">Automatic (Fastest Available)</option>
            <option value="wired">Wired USB (ADB / Tethering)</option>
            <option value="wifi_lan">Wi-Fi LAN (mDNS + HTTPS/WS)</option>
            <option value="wifi_direct">Wi-Fi Direct</option>
            <option value="bluetooth">Bluetooth (Fallback - Slow)</option>
          </select>
          {manualOverride === 'bluetooth' && (
            <div style={{ fontSize: 11.5, color: 'var(--warning)', marginTop: 6 }}>
              Warning: Bluetooth transfer is slow (~1–2 MB/s) and best for small files only.
            </div>
          )}
        </div>

        <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
          <strong style={{ fontSize: 12.5 }}>Paired Devices</strong>
          {devices
            .filter((d) => !d.isSelf)
            .map((dev) => (
              <div
                key={dev.id}
                style={{
                  display: 'flex',
                  justifyContent: 'space-between',
                  alignItems: 'center',
                  padding: '8px 10px',
                  borderRadius: 8,
                  background: 'var(--bg-elevated)',
                }}
              >
                <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                  <Smartphone size={16} />
                  <div>
                    <div style={{ fontSize: 13, fontWeight: 600 }}>{dev.name}</div>
                    <div style={{ fontSize: 11, color: 'var(--text-secondary)' }}>
                      {dev.isOnline ? 'Online' : 'Offline (cached thumbnails browsable)'} · {dev.transports.join(', ')}
                    </div>
                  </div>
                </div>
                <div style={{ display: 'flex', gap: 6 }}>
                  <button
                    type="button"
                    className="btn-secondary"
                    onClick={() => onToggleOnline(dev.id)}
                  >
                    {dev.isOnline ? 'Simulate Offline' : 'Go Online'}
                  </button>
                  <button
                    type="button"
                    className="btn-danger"
                    onClick={() => onForgetDevice(dev.id)}
                  >
                    Forget
                  </button>
                </div>
              </div>
            ))}
        </div>
      </div>
    </div>
  );
};

export const YouTubeDownloadModal: React.FC<{
  open: boolean;
  devices: DeviceRecord[];
  onClose: () => void;
  onStartDownload: (payload: {
    url: string;
    format: DownloadFormat;
    quality: string;
    destinationDeviceId: string;
    destinationFolder: string;
    allowPlaylist: boolean;
  }) => void;
}> = ({ open, devices, onClose, onStartDownload }) => {
  const [url, setUrl] = useState('https://www.youtube.com/watch?v=dQw4w9WgXcQ');
  const [format, setFormat] = useState<DownloadFormat>('mp4');
  const [quality, setQuality] = useState('1080p');
  const [destDeviceId, setDestDeviceId] = useState(devices[0]?.id ?? 'pc-local');
  const [destFolder, setDestFolder] = useState('C:/Users/Media/YouTube');
  const [allowPlaylist, setAllowPlaylist] = useState(false);

  if (!open) return null;

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div className="modal-panel" onClick={(e) => e.stopPropagation()}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <strong style={{ display: 'inline-flex', alignItems: 'center', gap: 8 }}>
            <Youtube size={18} color="#ef4444" /> Download from YouTube (yt-dlp + ffmpeg)
          </strong>
          <button type="button" className="crop-chip" onClick={onClose}>
            <X size={14} />
          </button>
        </div>

        <label style={{ fontSize: 12, color: 'var(--text-secondary)' }}>YouTube Video or Playlist URL</label>
        <input
          className="text-control"
          value={url}
          onChange={(e) => setUrl(e.target.value)}
          placeholder="https://www.youtube.com/watch?v=..."
        />

        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 10 }}>
          <div>
            <label style={{ fontSize: 12, color: 'var(--text-secondary)' }}>Format</label>
            <select
              className="select-control"
              style={{ width: '100%', marginTop: 4 }}
              value={format}
              onChange={(e) => {
                const next = e.target.value as DownloadFormat;
                setFormat(next);
                setQuality(next === 'mp3' ? '192k' : '1080p');
                setDestFolder(next === 'mp3' ? 'C:/Users/Media/Music' : 'C:/Users/Media/YouTube');
              }}
            >
              <option value="mp4">MP4 Video</option>
              <option value="mp3">MP3 Audio (adds to Music Library)</option>
            </select>
          </div>

          <div>
            <label style={{ fontSize: 12, color: 'var(--text-secondary)' }}>Quality</label>
            <select
              className="select-control"
              style={{ width: '100%', marginTop: 4 }}
              value={quality}
              onChange={(e) => setQuality(e.target.value)}
            >
              {format === 'mp4' ? (
                <>
                  <option value="2160p">4K (2160p)</option>
                  <option value="1080p">Full HD (1080p)</option>
                  <option value="720p">HD (720p)</option>
                </>
              ) : (
                <>
                  <option value="320k">320 kbps</option>
                  <option value="192k">192 kbps</option>
                  <option value="128k">128 kbps</option>
                </>
              )}
            </select>
          </div>
        </div>

        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 10 }}>
          <div>
            <label style={{ fontSize: 12, color: 'var(--text-secondary)' }}>Destination Device</label>
            <select
              className="select-control"
              style={{ width: '100%', marginTop: 4 }}
              value={destDeviceId}
              onChange={(e) => setDestDeviceId(e.target.value)}
            >
              {devices.map((d) => (
                <option key={d.id} value={d.id}>
                  {d.name}
                </option>
              ))}
            </select>
          </div>
          <div>
            <label style={{ fontSize: 12, color: 'var(--text-secondary)' }}>Destination Folder</label>
            <input
              className="text-control"
              style={{ width: '100%', marginTop: 4 }}
              value={destFolder}
              onChange={(e) => setDestFolder(e.target.value)}
            />
          </div>
        </div>

        <label style={{ display: 'flex', alignItems: 'center', gap: 8, fontSize: 12.5 }}>
          <input
            type="checkbox"
            checked={allowPlaylist}
            onChange={(e) => setAllowPlaylist(e.target.checked)}
          />
          Allow playlist download (if URL is a playlist)
        </label>

        <div style={{ fontSize: 11.5, color: 'var(--text-muted)', padding: 8, borderRadius: 6, background: 'var(--bg-elevated)' }}>
          Notice: Download only content you have the right to download under applicable copyright rules and YouTube&apos;s Terms of Service.
        </div>

        <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 8 }}>
          <button type="button" className="btn-secondary" onClick={onClose}>
            Cancel
          </button>
          <button
            type="button"
            className="btn-primary"
            onClick={() => {
              onStartDownload({
                url,
                format,
                quality,
                destinationDeviceId: destDeviceId,
                destinationFolder: destFolder,
                allowPlaylist,
              });
              onClose();
            }}
          >
            Download into Library
          </button>
        </div>
      </div>
    </div>
  );
};
