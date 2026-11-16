import React, { useState } from 'react';
import { LeftSidebar } from './components/LeftSidebar.tsx';
import { TopBar } from './components/TopBar.tsx';
import { BottomShortcutBar } from './components/BottomShortcutBar.tsx';
import type { NavSection } from './state/navigation.ts';
import type { PinnedFolderRecord } from '../core/types/domain.ts';

const INITIAL_PINNED: PinnedFolderRecord[] = [
  { id: 'pin-camera', deviceId: 'pc-local', path: 'C:/Users/Media/Camera', label: 'Camera', icon: 'folder', position: 0 },
  { id: 'pin-music', deviceId: 'pc-local', path: 'C:/Users/Media/Music', label: 'Music', icon: 'folder', position: 1 },
  { id: 'pin-youtube', deviceId: 'pc-local', path: 'C:/Users/Media/YouTube', label: 'YouTube', icon: 'folder', position: 2 },
];

export const App: React.FC = () => {
  const [activeSection, setActiveSection] = useState<NavSection>('home');
  const [activeFolderPath, setActiveFolderPath] = useState<string | null>(null);
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
  const [sidebarWidth, setSidebarWidth] = useState(248);
  const [searchQuery, setSearchQuery] = useState('');
  const [theme, setTheme] = useState<'dark' | 'light'>('dark');
  const [pinnedFolders, setPinnedFolders] = useState<PinnedFolderRecord[]>(INITIAL_PINNED);

  return (
    <div className="desktop-shell" data-theme={theme}>
      <div className="desktop-upper">
        <LeftSidebar
          activeSection={activeSection}
          activeFolderPath={activeFolderPath}
          collapsed={sidebarCollapsed}
          width={sidebarWidth}
          pinnedFolders={pinnedFolders}
          onToggleCollapse={() => setSidebarCollapsed((c) => !c)}
          onWidthChange={setSidebarWidth}
          onSelectSection={(sec) => {
            setActiveSection(sec);
            setActiveFolderPath(null);
          }}
          onOpenFolder={(folderPath) => setActiveFolderPath(folderPath)}
          onDropMediaToFolder={() => {}}
        />

        <div className="right-column">
          <TopBar
            searchQuery={searchQuery}
            onSearchChange={setSearchQuery}
            connectedDevices={[]}
            activeTransport={null}
            googleAccountEmail={null}
            theme={theme}
            showFilters={false}
            onToggleFilters={() => {}}
            onOpenConnectModal={() => setActiveSection('devices')}
            onYouTubeAction={() => setActiveSection('youtube')}
            onToggleGoogleAccount={() => {}}
            onToggleTheme={() => setTheme((t) => (t === 'dark' ? 'light' : 'dark'))}
          />

          <div className="workspace-body">
            <main className="main-content">
              <h2>Section: {activeFolderPath ?? activeSection}</h2>
              <p style={{ color: 'var(--text-secondary)' }}>
                Foundation shell active. Library scanning and gallery modules loading in Milestone M1.
              </p>
            </main>
          </div>
        </div>
      </div>

      <BottomShortcutBar
        pinnedFolders={pinnedFolders}
        activeFolderPath={activeFolderPath}
        onOpenFolder={(path) => setActiveFolderPath(path)}
        onAddPinnedFolder={() => {}}
        onRenamePinnedFolder={(id, label) =>
          setPinnedFolders((prev) => prev.map((f) => (f.id === id ? { ...f, label } : f)))
        }
        onRemovePinnedFolder={(id) =>
          setPinnedFolders((prev) => prev.filter((f) => f.id !== id))
        }
        onReorderPinnedFolders={(ids) =>
          setPinnedFolders((prev) =>
            ids
              .map((id, idx) => {
                const found = prev.find((f) => f.id === id);
                return found ? { ...found, position: idx } : null;
              })
              .filter((x): x is PinnedFolderRecord => x !== null),
          )
        }
        onDropMediaToFolder={() => {}}
      />
    </div>
  );
};
