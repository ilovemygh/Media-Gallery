import React, { useRef, useState } from 'react';
import { Folder, Plus, X } from 'lucide-react';
import type { PinnedFolderRecord } from '../../core/types/domain.ts';

export interface BottomShortcutBarProps {
  pinnedFolders: PinnedFolderRecord[];
  activeFolderPath: string | null;
  onOpenFolder: (folderPath: string) => void;
  onAddPinnedFolder: () => void;
  onRenamePinnedFolder: (id: string, newLabel: string) => void;
  onRemovePinnedFolder: (id: string) => void;
  onReorderPinnedFolders: (orderedIds: string[]) => void;
  onDropMediaToFolder: (mediaId: string, folderPath: string) => void;
}

const SPRING_OPEN_DELAY_MS = 700;

/**
 * PRD Section 7.6 Bottom Bar:
 * - Customizable pinned folder shortcuts (icon + name)
 * - Right-click (or edit menu) to rename or remove
 * - Drag to reorder shortcuts
 * - Drop media files onto a shortcut to move them there (+ 700ms spring-open)
 */
export const BottomShortcutBar: React.FC<BottomShortcutBarProps> = ({
  pinnedFolders,
  activeFolderPath,
  onOpenFolder,
  onAddPinnedFolder,
  onRenamePinnedFolder,
  onRemovePinnedFolder,
  onReorderPinnedFolders,
  onDropMediaToFolder,
}) => {
  const [dragHoverId, setDragHoverId] = useState<string | null>(null);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [draftLabel, setDraftLabel] = useState('');
  const springTimerRef = useRef<number | null>(null);

  const clearSpring = () => {
    if (springTimerRef.current !== null) {
      window.clearTimeout(springTimerRef.current);
      springTimerRef.current = null;
    }
  };

  const handleShortcutDragStart = (event: React.DragEvent, pinnedId: string) => {
    event.dataTransfer.setData('text/pinned-id', pinnedId);
  };

  const handleShortcutDragEnter = (folder: PinnedFolderRecord) => {
    setDragHoverId(folder.id);
    clearSpring();
    springTimerRef.current = window.setTimeout(() => {
      onOpenFolder(folder.path);
    }, SPRING_OPEN_DELAY_MS);
  };

  const handleShortcutDrop = (event: React.DragEvent, targetFolder: PinnedFolderRecord) => {
    event.preventDefault();
    clearSpring();
    setDragHoverId(null);

    const draggedPinnedId = event.dataTransfer.getData('text/pinned-id');
    if (draggedPinnedId && draggedPinnedId !== targetFolder.id) {
      const ids = pinnedFolders.map((f) => f.id);
      const fromIdx = ids.indexOf(draggedPinnedId);
      const toIdx = ids.indexOf(targetFolder.id);
      if (fromIdx >= 0 && toIdx >= 0) {
        ids.splice(fromIdx, 1);
        ids.splice(toIdx, 0, draggedPinnedId);
        onReorderPinnedFolders(ids);
      }
      return;
    }

    const mediaId = event.dataTransfer.getData('text/media-id');
    if (mediaId) {
      onDropMediaToFolder(mediaId, targetFolder.path);
    }
  };

  const commitRename = (id: string) => {
    const trimmed = draftLabel.trim();
    if (trimmed) {
      onRenamePinnedFolder(id, trimmed);
    }
    setEditingId(null);
  };

  return (
    <footer className="bottom-bar" aria-label="Pinned folders bottom bar">
      {pinnedFolders.map((folder) => {
        const isActive = activeFolderPath === folder.path;
        const isHover = dragHoverId === folder.id;
        return (
          <div
            key={folder.id}
            draggable
            onDragStart={(e) => handleShortcutDragStart(e, folder.id)}
            onDragOver={(e) => e.preventDefault()}
            onDragEnter={() => handleShortcutDragEnter(folder)}
            onDragLeave={() => {
              setDragHoverId(null);
              clearSpring();
            }}
            onDrop={(e) => handleShortcutDrop(e, folder)}
            onContextMenu={(e) => {
              e.preventDefault();
              setEditingId(folder.id);
              setDraftLabel(folder.label);
            }}
            className={`pinned-chip ${isActive ? 'active' : ''} ${isHover ? 'drag-hover' : ''}`}
            onClick={() => onOpenFolder(folder.path)}
            title={`${folder.path} (right-click to rename, drag to reorder, drop files to move)`}
          >
            <Folder size={14} />
            {editingId === folder.id ? (
              <input
                className="text-control"
                style={{ height: 22, width: 100 }}
                value={draftLabel}
                autoFocus
                onClick={(e) => e.stopPropagation()}
                onChange={(e) => setDraftLabel(e.target.value)}
                onBlur={() => commitRename(folder.id)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') commitRename(folder.id);
                  if (e.key === 'Escape') setEditingId(null);
                }}
              />
            ) : (
              <span>{folder.label}</span>
            )}
            <button
              type="button"
              style={{
                border: 'none',
                background: 'transparent',
                padding: 0,
                display: 'inline-flex',
                cursor: 'pointer',
                color: 'var(--text-muted)',
              }}
              onClick={(e) => {
                e.stopPropagation();
                onRemovePinnedFolder(folder.id);
              }}
              title="Unpin folder"
              aria-label={`Remove pinned folder ${folder.label}`}
            >
              <X size={12} />
            </button>
          </div>
        );
      })}

      <button
        type="button"
        className="pinned-chip"
        onClick={onAddPinnedFolder}
        title="Pin current or new folder shortcut"
      >
        <Plus size={14} />
        <span>Pin folder</span>
      </button>
    </footer>
  );
};
