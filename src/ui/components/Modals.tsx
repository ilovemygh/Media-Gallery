import React, { useMemo, useState } from 'react';
import { X } from 'lucide-react';
import type { MediaWithCover } from '../../core/types/domain.ts';
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
