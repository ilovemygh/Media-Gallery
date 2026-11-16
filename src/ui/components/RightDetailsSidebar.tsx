import React, { useEffect, useState } from 'react';
import {
  Check,
  Crop,
  Download,
  FolderOpen,
  ImagePlus,
  RefreshCw,
  RotateCcw,
  Send,
  Trash2,
  X,
} from 'lucide-react';
import type {
  AspectPreset,
  CropRect,
  DeviceRecord,
  MediaWithCover,
} from '../../core/types/domain.ts';
import { formatDateToken } from '../../features/alias/aliasEngine.ts';
import { formatBytes, formatDuration } from './MainGalleryArea.tsx';

export interface RightDetailsSidebarProps {
  media: MediaWithCover | null;
  devices: DeviceRecord[];
  canUndoRename: boolean;
  onClose: () => void;
  onSaveAlias: (mediaId: string, alias: string) => void;
  onApplyRealRename: (mediaId: string, newName: string) => void;
  onUndoRename: (mediaId: string) => void;
  onChangeAspect: (mediaId: string, aspect: AspectPreset, customCrop?: CropRect) => void;
  onRerollVideoFrame: (mediaId: string) => void;
  onReplaceCoverImage: (mediaId: string, dataUrl: string, aspect: AspectPreset) => void;
  onRemoveAudioCover: (mediaId: string) => void;
  onRestoreAudioCover: (mediaId: string) => void;
  onSendToDevice: (mediaId: string, targetDeviceId: string) => void;
  onDownloadRemote: (mediaId: string) => void;
  onOpenContainingFolder: (folderPath: string) => void;
  onMoveToTrash: (mediaId: string) => void;
}

const ASPECT_PRESETS: AspectPreset[] = ['4:3', '9:16', '16:9', '1:1', 'custom'];

function aspectCssRatio(aspect: AspectPreset, crop?: CropRect | null): string {
  switch (aspect) {
    case '4:3':
      return '4 / 3';
    case '9:16':
      return '9 / 16';
    case '1:1':
      return '1 / 1';
    case 'custom':
      return crop ? `${crop.width} / ${Math.max(1, crop.height)}` : '16 / 10';
    case '16:9':
    default:
      return '16 / 9';
  }
}

/**
 * PRD Section 7.5 Right Details Sidebar:
 * - Cover preview with aspect ratio crop presets (4:3, 9:16, 16:9, 1:1, custom)
 * - Name / editable Alias / Apply as real rename + Undo
 * - Full metadata grid + device + sync state + path
 * - Change cover (video first-third re-roll or custom image; audio cover edit with confirmation)
 * - Send to device, Download/Sync, Open folder, Delete to Trash
 */
export const RightDetailsSidebar: React.FC<RightDetailsSidebarProps> = ({
  media,
  devices,
  canUndoRename,
  onClose,
  onSaveAlias,
  onApplyRealRename,
  onUndoRename,
  onChangeAspect,
  onRerollVideoFrame,
  onReplaceCoverImage,
  onRemoveAudioCover,
  onRestoreAudioCover,
  onSendToDevice,
  onDownloadRemote,
  onOpenContainingFolder,
  onMoveToTrash,
}) => {
  const [aliasDraft, setAliasDraft] = useState('');
  const [customW, setCustomW] = useState(1200);
  const [customH, setCustomH] = useState(800);
  const [sendTargetId, setSendTargetId] = useState('');

  useEffect(() => {
    if (media) {
      setAliasDraft(media.alias || media.autoAlias || media.fileName);
    }
    const peer = devices.find((d) => !d.isSelf);
    setSendTargetId(peer?.id ?? '');
  }, [media, devices]);

  if (!media) return null;

  const currentAspect: AspectPreset = media.cover?.aspect ?? '16:9';

  const handleCoverUpload = (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (!file) return;
    if (media.type === 'audio') {
      const confirmed = window.confirm(
        'Write new cover image into embedded audio tag metadata for this file?',
      );
      if (!confirmed) return;
    }
    const reader = new FileReader();
    reader.onload = () => {
      if (typeof reader.result === 'string') {
        onReplaceCoverImage(media.id, reader.result, currentAspect);
      }
    };
    reader.readAsDataURL(file);
  };

  return (
    <aside className="details-sidebar" aria-label="Details sidebar">
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <strong style={{ fontSize: 13, letterSpacing: '0.04em' }}>DETAILS</strong>
        <button
          type="button"
          className="crop-chip"
          onClick={onClose}
          title="Hide details pane for this session"
          aria-label="Close details panel"
        >
          <X size={14} />
        </button>
      </div>

      <div
        className="details-cover-preview"
        style={{ aspectRatio: aspectCssRatio(currentAspect, media.cover?.crop) }}
      >
        {media.cover?.filePath ? (
          <img src={media.cover.filePath} alt={media.alias || media.fileName} />
        ) : (
          <div style={{ padding: 24, textAlign: 'center', color: 'var(--text-muted)' }}>
            No cover image
          </div>
        )}
      </div>

      <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 6, flexWrap: 'wrap' }}>
          <label className="btn-secondary" style={{ cursor: 'pointer' }}>
            <ImagePlus size={14} /> Change cover
            <input
              type="file"
              accept="image/*"
              style={{ display: 'none' }}
              onChange={handleCoverUpload}
            />
          </label>

          {media.type === 'video' && (
            <button
              type="button"
              className="btn-secondary"
              onClick={() => onRerollVideoFrame(media.id)}
              title="Re-roll random frame from first third"
            >
              <RefreshCw size={14} /> Re-roll frame
            </button>
          )}

          {media.type === 'audio' && (
            <>
              <button
                type="button"
                className="btn-secondary"
                onClick={() => {
                  if (window.confirm('Remove embedded cover art from this track?')) {
                    onRemoveAudioCover(media.id);
                  }
                }}
              >
                Remove cover
              </button>
              <button
                type="button"
                className="btn-secondary"
                onClick={() => onRestoreAudioCover(media.id)}
              >
                Restore cover
              </button>
            </>
          )}
        </div>

        <div>
          <div className="meta-key" style={{ fontSize: 11.5, marginBottom: 6, display: 'flex', alignItems: 'center', gap: 5 }}>
            <Crop size={12} /> Crop aspect ratio
          </div>
          <div className="crop-preset-row">
            {ASPECT_PRESETS.map((preset) => (
              <button
                key={preset}
                type="button"
                className={`crop-chip ${currentAspect === preset ? 'active' : ''}`}
                onClick={() =>
                  onChangeAspect(
                    media.id,
                    preset,
                    preset === 'custom'
                      ? { x: 0, y: 0, width: customW, height: customH, aspectRatio: customW / Math.max(1, customH) }
                      : undefined,
                  )
                }
              >
                {preset}
              </button>
            ))}
          </div>
          {currentAspect === 'custom' && (
            <div style={{ display: 'flex', gap: 6, marginTop: 8 }}>
              <input
                type="number"
                className="text-control"
                style={{ width: 80 }}
                value={customW}
                min={100}
                onChange={(e) => {
                  const w = Number(e.target.value) || 100;
                  setCustomW(w);
                  onChangeAspect(media.id, 'custom', { x: 0, y: 0, width: w, height: customH });
                }}
              />
              <span style={{ alignSelf: 'center', color: 'var(--text-muted)' }}>×</span>
              <input
                type="number"
                className="text-control"
                style={{ width: 80 }}
                value={customH}
                min={100}
                onChange={(e) => {
                  const h = Number(e.target.value) || 100;
                  setCustomH(h);
                  onChangeAspect(media.id, 'custom', { x: 0, y: 0, width: customW, height: h });
                }}
              />
            </div>
          )}
        </div>
      </div>

      <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
        <label style={{ fontSize: 12, color: 'var(--text-secondary)' }}>Display Alias (non-destructive)</label>
        <input
          className="text-control"
          value={aliasDraft}
          onChange={(e) => setAliasDraft(e.target.value)}
        />
        <div style={{ display: 'flex', gap: 6, flexWrap: 'wrap' }}>
          <button
            type="button"
            className="btn-primary"
            onClick={() => onSaveAlias(media.id, aliasDraft)}
          >
            <Check size={14} /> Save Alias
          </button>
          <button
            type="button"
            className="btn-secondary"
            onClick={() => onApplyRealRename(media.id, aliasDraft)}
            title="Rename actual file on disk and log for undo"
          >
            Apply as real rename
          </button>
          {canUndoRename && (
            <button
              type="button"
              className="btn-secondary"
              onClick={() => onUndoRename(media.id)}
            >
              <RotateCcw size={13} /> Undo
            </button>
          )}
        </div>
      </div>

      <div className="meta-table">
        <span className="meta-key">File name</span>
        <span className="meta-val">{media.fileName}</span>

        <span className="meta-key">Type</span>
        <span className="meta-val">{media.type} ({media.mime})</span>

        <span className="meta-key">Size</span>
        <span className="meta-val">{formatBytes(media.size)}</span>

        <span className="meta-key">Created</span>
        <span className="meta-val">{formatDateToken(media.createdAt, true)}</span>

        {media.durationMs > 0 && (
          <>
            <span className="meta-key">Duration</span>
            <span className="meta-val">{formatDuration(media.durationMs)}</span>
          </>
        )}

        {media.width > 0 && (
          <>
            <span className="meta-key">Resolution</span>
            <span className="meta-val">{media.width} × {media.height}</span>
          </>
        )}

        {media.tags?.artist && (
          <>
            <span className="meta-key">Artist</span>
            <span className="meta-val">{media.tags.artist}</span>
          </>
        )}

        {media.place && (
          <>
            <span className="meta-key">Location</span>
            <span className="meta-val">{media.place}</span>
          </>
        )}

        <span className="meta-key">Device</span>
        <span className="meta-val">{media.deviceName ?? media.deviceId}</span>

        <span className="meta-key">Sync State</span>
        <span className="meta-val">{media.syncState}</span>

        <span className="meta-key">Path</span>
        <span className="meta-val">{media.path}</span>
      </div>

      <div style={{ display: 'flex', flexDirection: 'column', gap: 8, marginTop: 'auto' }}>
        {media.syncState === 'Remote' && (
          <button
            type="button"
            className="btn-primary"
            disabled={media.deviceOnline === false}
            onClick={() => onDownloadRemote(media.id)}
          >
            <Download size={14} />
            {media.deviceOnline === false ? 'Owner Device Offline' : 'Download / Sync to PC'}
          </button>
        )}

        {sendTargetId && (
          <div style={{ display: 'flex', gap: 6 }}>
            <select
              className="select-control"
              style={{ flex: 1 }}
              value={sendTargetId}
              onChange={(e) => setSendTargetId(e.target.value)}
            >
              {devices
                .filter((d) => d.id !== media.deviceId)
                .map((d) => (
                  <option key={d.id} value={d.id}>
                    {d.name}
                  </option>
                ))}
            </select>
            <button
              type="button"
              className="btn-secondary"
              onClick={() => onSendToDevice(media.id, sendTargetId)}
            >
              <Send size={13} /> Send
            </button>
          </div>
        )}

        <div style={{ display: 'flex', gap: 6 }}>
          <button
            type="button"
            className="btn-secondary"
            style={{ flex: 1 }}
            onClick={() => onOpenContainingFolder(media.folderPath)}
          >
            <FolderOpen size={14} /> Open folder
          </button>
          <button
            type="button"
            className="btn-danger"
            onClick={() => onMoveToTrash(media.id)}
            title="Moves file to Recycle Bin / Trash only"
          >
            <Trash2 size={14} /> Trash
          </button>
        </div>
      </div>
    </aside>
  );
};
