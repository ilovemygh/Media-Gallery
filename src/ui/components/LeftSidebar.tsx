import React, { useEffect, useRef, useState } from 'react';
import {
  Copy,
  Film,
  Folder,
  HardDrive,
  Home,
  Image as ImageIcon,
  Menu,
  Music,
  Settings,
  Youtube,
} from 'lucide-react';
import type { PinnedFolderRecord } from '../../core/types/domain.ts';
import type { NavSection } from '../state/navigation.ts';

export interface LeftSidebarProps {
  activeSection: NavSection;
  activeFolderPath: string | null;
  collapsed: boolean;
  width: number;
  pinnedFolders: PinnedFolderRecord[];
  onToggleCollapse: () => void;
  onWidthChange: (newWidth: number) => void;
  onSelectSection: (section: NavSection) => void;
  onOpenFolder: (folderPath: string) => void;
  onDropMediaToFolder: (mediaId: string, folderPath: string) => void;
}

const SPRING_OPEN_DELAY_MS = 700;

/**
 * PRD Section 7.2 Left Sidebar:
 * - Toggle icon at the top-left collapses to icons only and expands again.
 * - Only the left sidebar is closable.
 * - Resizable when open (width remembered between sessions).
 * - Folders inside are drop targets with ~700ms spring-loaded opening.
 */
export const LeftSidebar: React.FC<LeftSidebarProps> = ({
  activeSection,
  activeFolderPath,
  collapsed,
  width,
  pinnedFolders,
  onToggleCollapse,
  onWidthChange,
  onSelectSection,
  onOpenFolder,
  onDropMediaToFolder,
}) => {
  const [resizing, setResizing] = useState(false);
  const [dragHoverTarget, setDragHoverTarget] = useState<string | null>(null);
  const springTimerRef = useRef<number | null>(null);

  useEffect(() => {
    if (!resizing) return;
    const handleMove = (event: MouseEvent) => {
      const clamped = Math.max(180, Math.min(380, event.clientX));
      onWidthChange(clamped);
    };
    const handleUp = () => setResizing(false);
    window.addEventListener('mousemove', handleMove);
    window.addEventListener('mouseup', handleUp);
    return () => {
      window.removeEventListener('mousemove', handleMove);
      window.removeEventListener('mouseup', handleUp);
    };
  }, [resizing, onWidthChange]);

  const clearSpringTimer = () => {
    if (springTimerRef.current !== null) {
      window.clearTimeout(springTimerRef.current);
      springTimerRef.current = null;
    }
  };

  const handleFolderDragEnter = (folderPath: string) => {
    setDragHoverTarget(folderPath);
    clearSpringTimer();
    springTimerRef.current = window.setTimeout(() => {
      onOpenFolder(folderPath);
    }, SPRING_OPEN_DELAY_MS);
  };

  const handleFolderDragLeave = () => {
    setDragHoverTarget(null);
    clearSpringTimer();
  };

  const handleFolderDrop = (event: React.DragEvent, folderPath: string) => {
    event.preventDefault();
    clearSpringTimer();
    setDragHoverTarget(null);
    const mediaId = event.dataTransfer.getData('text/media-id');
    if (mediaId) {
      onDropMediaToFolder(mediaId, folderPath);
    }
  };

  const navItem = (
    section: NavSection,
    label: string,
    icon: React.ReactNode,
  ) => (
    <button
      key={section}
      type="button"
      title={label}
      className={`sidebar-nav-btn ${activeSection === section && !activeFolderPath ? 'active' : ''}`}
      onClick={() => onSelectSection(section)}
    >
      {icon}
      {!collapsed && <span>{label}</span>}
    </button>
  );

  return (
    <aside
      className={`left-sidebar ${collapsed ? 'collapsed' : ''}`}
      style={{ width: collapsed ? 72 : width }}
      aria-label="Primary Sidebar"
    >
      <div className="sidebar-top-row">
        <button
          type="button"
          className="sidebar-toggle-btn"
          onClick={onToggleCollapse}
          title={collapsed ? 'Expand sidebar' : 'Collapse sidebar to icons'}
          aria-label="Toggle left sidebar"
        >
          <Menu size={19} />
        </button>
        {!collapsed && <span className="brand-title">Media Gallery</span>}
      </div>

      <div className="sidebar-scroll">
        <div>
          {navItem('home', 'Home', <Home size={20} />)}
        </div>

        <div>
          {!collapsed && <div className="sidebar-section-label">Library</div>}
          {navItem('videos', 'Videos', <Film size={20} />)}
          {navItem('images', 'Images', <ImageIcon size={20} />)}
          {navItem('audio', 'Audio', <Music size={20} />)}
        </div>

        <div>
          {!collapsed && <div className="sidebar-section-label">YouTube</div>}
          {navItem('youtube', 'YouTube', <Youtube size={20} />)}
        </div>

        <div>
          {!collapsed && <div className="sidebar-section-label">Devices & Tools</div>}
          {navItem('devices', 'Devices & Transfers', <HardDrive size={20} />)}
          {navItem('duplicates', 'Duplicates', <Copy size={20} />)}
          {navItem('settings', 'Settings', <Settings size={20} />)}
        </div>

        {pinnedFolders.length > 0 && (
          <div>
            {!collapsed && <div className="sidebar-section-label">Folders</div>}
            {pinnedFolders.map((folder) => {
              const isHovering = dragHoverTarget === folder.path;
              const isActive = activeFolderPath === folder.path;
              return (
                <button
                  key={folder.id}
                  type="button"
                  title={folder.path}
                  className={`sidebar-nav-btn ${isActive ? 'active' : ''} ${isHovering ? 'drag-hover' : ''}`}
                  onClick={() => onOpenFolder(folder.path)}
                  onDragOver={(event) => event.preventDefault()}
                  onDragEnter={() => handleFolderDragEnter(folder.path)}
                  onDragLeave={handleFolderDragLeave}
                  onDrop={(event) => handleFolderDrop(event, folder.path)}
                >
                  <Folder size={19} />
                  {!collapsed && <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{folder.label}</span>}
                </button>
              );
            })}
          </div>
        )}
      </div>

      {!collapsed && (
        <div
          className={`sidebar-resizer ${resizing ? 'resizing' : ''}`}
          onMouseDown={() => setResizing(true)}
          title="Drag to resize sidebar"
          role="separator"
          aria-orientation="vertical"
        />
      )}
    </aside>
  );
};
