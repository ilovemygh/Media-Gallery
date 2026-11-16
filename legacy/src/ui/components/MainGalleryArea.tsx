import React, { useRef, useState } from 'react';
import {
  Cloud,
  Download,
  Film,
  FolderOpen,
  Grid2x2,
  Grid3x3,
  Image as ImageIcon,
  Laptop,
  LayoutGrid,
  List,
  Music,
  Play,
  Smartphone,
  Sparkles,
  Upload,
} from 'lucide-react';
import type {
  FolderCardSummary,
  GridDensity,
  GroupByOption,
  MediaWithCover,
  SortDirection,
  SortField,
} from '../../core/types/domain.ts';
import { formatDateToken } from '../../features/alias/aliasEngine.ts';

export interface MainGalleryAreaProps {
  title: string;
  activeFolderPath: string | null;
  folders: FolderCardSummary[];
  items: MediaWithCover[];
  selectedMediaId: string | null;
  gridDensity: GridDensity;
  sortBy: SortField;
  sortDir: SortDirection;
  groupBy: GroupByOption;
  onChangeGridDensity: (density: GridDensity) => void;
  onChangeSort: (field: SortField, dir: SortDirection) => void;
  onChangeGroupBy: (group: GroupByOption) => void;
  onSelectMedia: (media: MediaWithCover) => void;
  onPreviewMedia: (media: MediaWithCover) => void;
  onOpenFolder: (folderPath: string | null) => void;
  onDropMediaToFolder: (mediaId: string, folderPath: string) => void;
  onDownloadRemote: (mediaId: string) => void;
  onOpenBulkRename: () => void;
  onImportLocalFiles: (files: FileList) => void;
}

const SPRING_OPEN_DELAY_MS = 700;

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

export function formatDuration(ms: number): string {
  if (!ms || ms <= 0) return '0:00';
  const totalSec = Math.floor(ms / 1000);
  const mins = Math.floor(totalSec / 60);
  const secs = totalSec % 60;
  return `${mins}:${String(secs).padStart(2, '0')}`;
}

/**
 * PRD Section 7.4 Main Content Area:
 * - Folder cards with 2x2 visual thumbnail previews
 * - 700ms spring-loaded hover opening on drag-and-drop
 * - Videos, Images, Audio views with grid sizes (small/medium/large) + list view
 * - Sort (name, date, size, duration, type) & Group (none, date, folder, device)
 * - Remote items display cloud badge + Download / Sync button (disabled cleanly when owner offline)
 */
export const MainGalleryArea: React.FC<MainGalleryAreaProps> = ({
  title,
  activeFolderPath,
  folders,
  items,
  selectedMediaId,
  gridDensity,
  sortBy,
  sortDir,
  groupBy,
  onChangeGridDensity,
  onChangeSort,
  onChangeGroupBy,
  onSelectMedia,
  onPreviewMedia,
  onOpenFolder,
  onDropMediaToFolder,
  onDownloadRemote,
  onOpenBulkRename,
  onImportLocalFiles,
}) => {
  const [dragHoverFolder, setDragHoverFolder] = useState<string | null>(null);
  const springTimerRef = useRef<number | null>(null);
  const fileInputRef = useRef<HTMLInputElement | null>(null);

  const clearSpring = () => {
    if (springTimerRef.current !== null) {
      window.clearTimeout(springTimerRef.current);
      springTimerRef.current = null;
    }
  };

  const handleFolderDragEnter = (folderPath: string) => {
    setDragHoverFolder(folderPath);
    clearSpring();
    springTimerRef.current = window.setTimeout(() => {
      onOpenFolder(folderPath);
    }, SPRING_OPEN_DELAY_MS);
  };

  const handleFolderDrop = (event: React.DragEvent, folderPath: string) => {
    event.preventDefault();
    clearSpring();
    setDragHoverFolder(null);
    const mediaId = event.dataTransfer.getData('text/media-id');
    if (mediaId) {
      onDropMediaToFolder(mediaId, folderPath);
    }
  };

  const groupedItems = React.useMemo(() => {
    if (groupBy === 'none') {
      return [{ label: '', items }];
    }
    const groups = new Map<string, MediaWithCover[]>();
    for (const item of items) {
      const key =
        groupBy === 'date'
          ? formatDateToken(item.createdAt, true)
          : groupBy === 'folder'
            ? item.folderPath
            : item.deviceName ?? item.deviceId;
      const list = groups.get(key) ?? [];
      list.push(item);
      groups.set(key, list);
    }
    return Array.from(groups.entries()).map(([label, list]) => ({ label, items: list }));
  }, [items, groupBy]);

  return (
    <main
      className="main-content"
      onDragOver={(e) => e.preventDefault()}
      onDrop={(e) => {
        if (e.dataTransfer.files && e.dataTransfer.files.length > 0) {
          e.preventDefault();
          onImportLocalFiles(e.dataTransfer.files);
        }
      }}
    >
      <div className="toolbar-row">
        <div className="toolbar-group">
          <h2 style={{ margin: 0, fontSize: 18 }}>{title}</h2>
          {activeFolderPath && (
            <button
              type="button"
              className="btn-secondary"
              onClick={() => onOpenFolder(null)}
            >
              Back to all
            </button>
          )}
          <span className="badge-subtle">{items.length} items</span>
        </div>

        <div className="toolbar-group">
          <input
            ref={fileInputRef}
            type="file"
            multiple
            accept="video/*,image/*,audio/*"
            style={{ display: 'none' }}
            onChange={(e) => {
              if (e.target.files && e.target.files.length > 0) {
                onImportLocalFiles(e.target.files);
              }
            }}
          />
          <button
            type="button"
            className="btn-secondary"
            onClick={() => fileInputRef.current?.click()}
            title="Import files from disk into library"
          >
            <Upload size={14} /> Add Files
          </button>

          <button
            type="button"
            className="btn-secondary"
            onClick={onOpenBulkRename}
            title="Bulk rename visible items with pattern & undo"
          >
            <Sparkles size={14} /> Bulk Rename
          </button>

          <select
            className="select-control"
            value={sortBy}
            onChange={(e) => onChangeSort(e.target.value as SortField, sortDir)}
            aria-label="Sort by"
          >
            <option value="date">Sort: Date</option>
            <option value="name">Sort: Name</option>
            <option value="size">Sort: Size</option>
            <option value="duration">Sort: Duration</option>
            <option value="type">Sort: Type</option>
          </select>

          <button
            type="button"
            className="btn-secondary"
            onClick={() => onChangeSort(sortBy, sortDir === 'asc' ? 'desc' : 'asc')}
          >
            {sortDir === 'asc' ? 'Asc' : 'Desc'}
          </button>

          <select
            className="select-control"
            value={groupBy}
            onChange={(e) => onChangeGroupBy(e.target.value as GroupByOption)}
            aria-label="Group by"
          >
            <option value="none">Group: None</option>
            <option value="date">Group: Date</option>
            <option value="folder">Group: Folder</option>
            <option value="device">Group: Device</option>
          </select>

          <div className="toolbar-group" role="group" aria-label="Grid density">
            <button
              type="button"
              className={`crop-chip ${gridDensity === 'small' ? 'active' : ''}`}
              onClick={() => onChangeGridDensity('small')}
              title="Small grid"
            >
              <Grid3x3 size={14} />
            </button>
            <button
              type="button"
              className={`crop-chip ${gridDensity === 'medium' ? 'active' : ''}`}
              onClick={() => onChangeGridDensity('medium')}
              title="Medium grid"
            >
              <Grid2x2 size={14} />
            </button>
            <button
              type="button"
              className={`crop-chip ${gridDensity === 'large' ? 'active' : ''}`}
              onClick={() => onChangeGridDensity('large')}
              title="Large grid"
            >
              <LayoutGrid size={14} />
            </button>
            <button
              type="button"
              className={`crop-chip ${gridDensity === 'list' ? 'active' : ''}`}
              onClick={() => onChangeGridDensity('list')}
              title="List view"
            >
              <List size={14} />
            </button>
          </div>
        </div>
      </div>

      {!activeFolderPath && folders.length > 0 && (
        <section>
          <div className="sidebar-section-label" style={{ paddingLeft: 0, marginBottom: 8 }}>
            Folders (drag files &amp; hover 700ms to spring-open)
          </div>
          <div className="folder-grid">
            {folders.map((folder) => (
              <div
                key={`${folder.deviceId}:${folder.path}`}
                className={`folder-card ${dragHoverFolder === folder.path ? 'drag-hover' : ''}`}
                onClick={() => onOpenFolder(folder.path)}
                onDragOver={(e) => e.preventDefault()}
                onDragEnter={() => handleFolderDragEnter(folder.path)}
                onDragLeave={() => {
                  setDragHoverFolder(null);
                  clearSpring();
                }}
                onDrop={(e) => handleFolderDrop(e, folder.path)}
              >
                <div className="folder-mosaic">
                  {[0, 1, 2, 3].map((slot) => {
                    const preview = folder.previewMedia[slot];
                    return preview?.cover?.filePath ? (
                      <img key={slot} src={preview.cover.filePath} alt={preview.alias} loading="lazy" />
                    ) : (
                      <div key={slot} className="folder-mosaic-empty">
                        {slot === 0 ? <FolderOpen size={16} /> : null}
                      </div>
                    );
                  })}
                </div>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                  <span style={{ fontSize: 13, fontWeight: 600 }}>{folder.name}</span>
                  <span className="badge-subtle">{folder.itemCount}</span>
                </div>
              </div>
            ))}
          </div>
        </section>
      )}

      {groupedItems.map((group) => (
        <section key={group.label || 'all'}>
          {group.label && (
            <div className="sidebar-section-label" style={{ paddingLeft: 0, marginBottom: 8 }}>
              {group.label} ({group.items.length})
            </div>
          )}

          {gridDensity === 'list' ? (
            <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
              {group.items.map((item) => (
                <div
                  key={item.id}
                  draggable
                  onDragStart={(e) => e.dataTransfer.setData('text/media-id', item.id)}
                  onClick={() => onSelectMedia(item)}
                  onDoubleClick={() => onPreviewMedia(item)}
                  className={`folder-card ${selectedMediaId === item.id ? 'drag-hover' : ''}`}
                  style={{
                    flexDirection: 'row',
                    alignItems: 'center',
                    justifyContent: 'space-between',
                    padding: '8px 12px',
                  }}
                >
                  <div style={{ display: 'flex', alignItems: 'center', gap: 12, minWidth: 0 }}>
                    <img
                      src={item.cover?.filePath}
                      alt={item.alias}
                      style={{ width: 52, height: 38, borderRadius: 4, objectFit: 'cover' }}
                    />
                    <div style={{ minWidth: 0 }}>
                      <div className="media-title">{item.alias || item.fileName}</div>
                      <div className="media-subtitle">
                        {item.fileName} · {item.deviceName ?? item.deviceId} · {formatBytes(item.size)}
                      </div>
                    </div>
                  </div>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                    <span className="badge-subtle">{item.syncState}</span>
                    {item.syncState === 'Remote' && (
                      <button
                        type="button"
                        className="btn-primary"
                        disabled={item.deviceOnline === false}
                        title={
                          item.deviceOnline === false
                            ? 'Owning device is offline; download unavailable'
                            : 'Download file from peer device'
                        }
                        onClick={(e) => {
                          e.stopPropagation();
                          onDownloadRemote(item.id);
                        }}
                      >
                        <Download size={13} /> {item.deviceOnline === false ? 'Offline' : 'Download'}
                      </button>
                    )}
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <div className={`media-grid ${gridDensity}`}>
              {group.items.map((item) => (
                <article
                  key={item.id}
                  draggable
                  onDragStart={(e) => e.dataTransfer.setData('text/media-id', item.id)}
                  onClick={() => onSelectMedia(item)}
                  onDoubleClick={() => onPreviewMedia(item)}
                  className={`media-card ${selectedMediaId === item.id ? 'selected' : ''}`}
                >
                  <div className="media-thumb-box">
                    {item.cover?.filePath ? (
                      <img src={item.cover.filePath} alt={item.alias || item.fileName} loading="lazy" />
                    ) : null}

                    <div className="badge-overlay-top">
                      <span className="pill-badge">
                        {item.type === 'video' ? (
                          <Film size={11} />
                        ) : item.type === 'audio' ? (
                          <Music size={11} />
                        ) : (
                          <ImageIcon size={11} />
                        )}
                        {item.deviceType === 'phone' ? <Smartphone size={11} /> : <Laptop size={11} />}
                      </span>

                      {item.syncState === 'Remote' && (
                        <span className="pill-badge" title="Remote item (metadata & thumbnail only)">
                          <Cloud size={11} /> Remote
                        </span>
                      )}
                      {item.isDuplicate && (
                        <span className="pill-badge" style={{ background: 'rgba(245, 158, 11, 0.9)' }}>
                          Dup
                        </span>
                      )}
                    </div>

                    {item.durationMs > 0 && (
                      <span className="duration-badge">{formatDuration(item.durationMs)}</span>
                    )}
                  </div>

                  <div className="media-card-body">
                    <div className="media-title" title={item.alias || item.fileName}>
                      {item.alias || item.fileName}
                    </div>
                    <div className="media-subtitle">
                      {item.fileName} · {formatBytes(item.size)}
                    </div>

                    {item.syncState === 'Remote' ? (
                      <button
                        type="button"
                        className="btn-primary"
                        style={{ marginTop: 4, width: '100%', justifyContent: 'center' }}
                        disabled={item.deviceOnline === false}
                        title={
                          item.deviceOnline === false
                            ? 'Owner device is offline — cannot download right now'
                            : 'Download real file on demand'
                        }
                        onClick={(e) => {
                          e.stopPropagation();
                          onDownloadRemote(item.id);
                        }}
                      >
                        <Download size={13} />
                        {item.deviceOnline === false ? 'Owner Offline' : 'Download / Sync'}
                      </button>
                    ) : item.type === 'audio' ? (
                      <button
                        type="button"
                        className="btn-secondary"
                        style={{ marginTop: 4, width: '100%', justifyContent: 'center' }}
                        onClick={(e) => {
                          e.stopPropagation();
                          onPreviewMedia(item);
                        }}
                      >
                        <Play size={13} /> Play Track
                      </button>
                    ) : null}
                  </div>
                </article>
              ))}
            </div>
          )}
        </section>
      ))}
    </main>
  );
};
