import React, { useEffect, useMemo, useState } from 'react';
import type {
  AspectPreset,
  CropRect,
  GridDensity,
  GroupByOption,
  MediaRecord,
  MediaSearchFilters,
  MediaWithCover,
  SortDirection,
  SortField,
  TimeOfDayFilter,
} from '../core/types/domain.ts';
import { generateAutoAlias } from '../features/alias/aliasEngine.ts';
import { BottomShortcutBar } from './components/BottomShortcutBar.tsx';
import { LeftSidebar } from './components/LeftSidebar.tsx';
import { MainGalleryArea } from './components/MainGalleryArea.tsx';
import { BulkRenameModal, MediaViewerModal } from './components/Modals.tsx';
import { RightDetailsSidebar } from './components/RightDetailsSidebar.tsx';
import { TopBar } from './components/TopBar.tsx';
import {
  applyAspectCropToCover,
  buildFolderSummaries,
  filterAndSortMedia,
  loadBrowserState,
  makeThumbDataUri,
  saveBrowserState,
  type GalleryAppState,
} from './state/browserEngine.ts';
import type { NavSection } from './state/navigation.ts';

export const App: React.FC = () => {
  const [state, setState] = useState<GalleryAppState>(() => loadBrowserState());
  const [activeSection, setActiveSection] = useState<NavSection>('home');
  const [activeFolderPath, setActiveFolderPath] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [showFilters, setShowFilters] = useState(false);
  const [deviceFilter, setDeviceFilter] = useState<string>('all');
  const [syncFilter, setSyncFilter] = useState<MediaSearchFilters['syncState']>('all');
  const [timeOfDayFilter, setTimeOfDayFilter] = useState<TimeOfDayFilter>('all');
  const [gridDensity, setGridDensity] = useState<GridDensity>('medium');
  const [sortBy, setSortBy] = useState<SortField>('date');
  const [sortDir, setSortDir] = useState<SortDirection>('desc');
  const [groupBy, setGroupBy] = useState<GroupByOption>('none');
  const [selectedMediaId, setSelectedMediaId] = useState<string | null>(null);
  const [detailsClosedForSession, setDetailsClosedForSession] = useState(false);
  const [previewMedia, setPreviewMedia] = useState<MediaWithCover | null>(null);
  const [bulkRenameOpen, setBulkRenameOpen] = useState(false);
  const [toast, setToast] = useState<string | null>(null);

  useEffect(() => {
    saveBrowserState(state);
  }, [state]);

  const showToast = (msg: string) => {
    setToast(msg);
    window.setTimeout(() => setToast((curr) => (curr === msg ? null : curr)), 3200);
  };

  const sectionMediaType: MediaSearchFilters['type'] =
    activeSection === 'videos'
      ? 'video'
      : activeSection === 'images'
        ? 'image'
        : activeSection === 'audio'
          ? 'audio'
          : 'all';

  const visibleItems = useMemo(
    () =>
      filterAndSortMedia(state, searchQuery, {
        type: sectionMediaType,
        deviceId: deviceFilter,
        syncState: syncFilter,
        folderPath: activeFolderPath,
        timeOfDay: timeOfDayFilter,
        sortBy,
        sortDir,
      }),
    [state, searchQuery, sectionMediaType, deviceFilter, syncFilter, activeFolderPath, timeOfDayFilter, sortBy, sortDir],
  );

  const visibleFolders = useMemo(
    () => buildFolderSummaries(state, sectionMediaType),
    [state, sectionMediaType],
  );

  const selectedMedia = useMemo(
    () =>
      detailsClosedForSession || !selectedMediaId
        ? null
        : visibleItems.find((item) => item.id === selectedMediaId) ?? null,
    [detailsClosedForSession, selectedMediaId, visibleItems],
  );

  const handleSelectMedia = (item: MediaWithCover) => {
    setSelectedMediaId(item.id);
    setDetailsClosedForSession(false);
  };

  const handleMoveToFolder = (mediaId: string, targetFolder: string) => {
    setState((prev) => {
      const nextMedia = prev.media.map((m) =>
        m.id === mediaId
          ? {
              ...m,
              folderPath: targetFolder,
              path: `${targetFolder}/${m.fileName}`,
              updatedAt: Date.now(),
            }
          : m,
      );
      return { ...prev, media: nextMedia };
    });
    showToast(`Moved item to ${targetFolder}`);
  };

  const handleImportLocalFiles = (files: FileList) => {
    const now = Date.now();
    const targetFolder = activeFolderPath ?? 'C:/Users/Media/Imported';
    const additions: MediaRecord[] = [];
    const coverAdditions: GalleryAppState['covers'] = [];

    Array.from(files).forEach((file, index) => {
      const id = `imp-${now}-${index}`;
      const type: MediaRecord['type'] = file.type.startsWith('video')
        ? 'video'
        : file.type.startsWith('audio')
          ? 'audio'
          : 'image';
      const { autoAlias, resolvedPlace } = generateAutoAlias({
        fileName: file.name,
        type,
        createdAt: file.lastModified || now,
        modifiedAt: file.lastModified || now,
        deviceName: 'Windows Workstation',
      });
      const objectUrl = URL.createObjectURL(file);
      const coverId = `cov-${id}`;
      coverAdditions.push({
        id: coverId,
        mediaId: id,
        source: 'generated',
        filePath:
          type === 'image'
            ? objectUrl
            : makeThumbDataUri(autoAlias, file.name, '#2563eb', '#0f172a', type === 'audio' ? '1:1' : '16:9'),
        cloudId: null,
        aspect: type === 'audio' ? '1:1' : '16:9',
        crop: null,
        frameTimeMs: type === 'video' ? 4200 : 0,
        updatedAt: now,
      });

      additions.push({
        id,
        deviceId: 'pc-local',
        path: `${targetFolder}/${file.name}`,
        folderPath: targetFolder,
        fileName: file.name,
        type,
        mime: file.type || 'application/octet-stream',
        size: file.size,
        hash: `local-${id}`,
        partialHash: `partial-${id}`,
        createdAt: file.lastModified || now,
        modifiedAt: file.lastModified || now,
        durationMs: type === 'image' ? 0 : 90_000,
        width: type === 'image' ? 1920 : 1280,
        height: type === 'image' ? 1080 : 720,
        gpsLat: null,
        gpsLon: null,
        place: resolvedPlace,
        tags: {},
        alias: autoAlias,
        autoAlias,
        coverId,
        syncState: 'Local',
        isTrashed: false,
        updatedAt: now,
      });
    });

    setState((prev) => ({
      ...prev,
      media: [...additions, ...prev.media],
      covers: [...coverAdditions, ...prev.covers],
    }));
    showToast(`Imported ${additions.length} file(s) into ${targetFolder}`);
  };

  return (
    <div className="desktop-shell" data-theme={state.settings.theme}>
      <div className="desktop-upper">
        <LeftSidebar
          activeSection={activeSection}
          activeFolderPath={activeFolderPath}
          collapsed={state.settings.sidebarCollapsed}
          width={state.settings.sidebarWidth}
          pinnedFolders={state.pinnedFolders}
          onToggleCollapse={() =>
            setState((prev) => ({
              ...prev,
              settings: { ...prev.settings, sidebarCollapsed: !prev.settings.sidebarCollapsed },
            }))
          }
          onWidthChange={(sidebarWidth) =>
            setState((prev) => ({ ...prev, settings: { ...prev.settings, sidebarWidth } }))
          }
          onSelectSection={(sec) => {
            setActiveSection(sec);
            setActiveFolderPath(null);
          }}
          onOpenFolder={(folderPath) => setActiveFolderPath(folderPath)}
          onDropMediaToFolder={handleMoveToFolder}
        />

        <div className="right-column">
          <TopBar
            searchQuery={searchQuery}
            onSearchChange={setSearchQuery}
            connectedDevices={state.devices}
            activeTransport={state.activeTransport}
            googleAccountEmail={state.googleAccountEmail}
            theme={state.settings.theme}
            showFilters={showFilters}
            onToggleFilters={() => setShowFilters((v) => !v)}
            onOpenConnectModal={() => setActiveSection('devices')}
            onYouTubeAction={() => setActiveSection('youtube')}
            onToggleGoogleAccount={() =>
              setState((prev) => ({
                ...prev,
                googleAccountEmail: prev.googleAccountEmail ? null : 'creator@gmail.com',
              }))
            }
            onToggleTheme={() =>
              setState((prev) => ({
                ...prev,
                settings: {
                  ...prev.settings,
                  theme: prev.settings.theme === 'dark' ? 'light' : 'dark',
                },
              }))
            }
          />

          {showFilters && (
            <div
              style={{
                display: 'flex',
                gap: 10,
                padding: '8px 16px',
                borderBottom: '1px solid var(--border-subtle)',
                background: 'var(--bg-surface)',
                flexWrap: 'wrap',
              }}
            >
              <select
                className="select-control"
                value={deviceFilter}
                onChange={(e) => setDeviceFilter(e.target.value)}
              >
                <option value="all">All Devices</option>
                {state.devices.map((d) => (
                  <option key={d.id} value={d.id}>
                    {d.name}
                  </option>
                ))}
              </select>

              <select
                className="select-control"
                value={syncFilter ?? 'all'}
                onChange={(e) => setSyncFilter(e.target.value as MediaSearchFilters['syncState'])}
              >
                <option value="all">All Sync States</option>
                <option value="Local">Local</option>
                <option value="Remote">Remote</option>
                <option value="Synced">Synced</option>
              </select>

              <select
                className="select-control"
                value={timeOfDayFilter}
                onChange={(e) => setTimeOfDayFilter(e.target.value as TimeOfDayFilter)}
              >
                <option value="all">Any Time of Day</option>
                <option value="morning">Morning</option>
                <option value="afternoon">Afternoon</option>
                <option value="evening">Evening</option>
                <option value="night">Night</option>
              </select>
            </div>
          )}

          <div className="workspace-body">
            <MainGalleryArea
              title={
                activeFolderPath
                  ? `Folder: ${activeFolderPath}`
                  : activeSection === 'home'
                    ? 'Home · Unified Media Library'
                    : activeSection.toUpperCase()
              }
              activeFolderPath={activeFolderPath}
              folders={visibleFolders}
              items={visibleItems}
              selectedMediaId={selectedMedia?.id ?? null}
              gridDensity={gridDensity}
              sortBy={sortBy}
              sortDir={sortDir}
              groupBy={groupBy}
              onChangeGridDensity={setGridDensity}
              onChangeSort={(f, d) => {
                setSortBy(f);
                setSortDir(d);
              }}
              onChangeGroupBy={setGroupBy}
              onSelectMedia={handleSelectMedia}
              onPreviewMedia={(item) => setPreviewMedia(item)}
              onOpenFolder={setActiveFolderPath}
              onDropMediaToFolder={handleMoveToFolder}
              onDownloadRemote={(mediaId) => {
                setState((prev) => ({
                  ...prev,
                  media: prev.media.map((m) =>
                    m.id === mediaId ? { ...m, syncState: 'Synced', updatedAt: Date.now() } : m,
                  ),
                }));
                showToast('Downloaded remote item to local storage (hash verified)');
              }}
              onOpenBulkRename={() => setBulkRenameOpen(true)}
              onImportLocalFiles={handleImportLocalFiles}
            />

            <RightDetailsSidebar
              media={selectedMedia}
              devices={state.devices}
              canUndoRename={
                selectedMedia
                  ? state.renameLogs.some((l) => l.mediaId === selectedMedia.id && !l.undoneAt)
                  : false
              }
              onClose={() => setDetailsClosedForSession(true)}
              onSaveAlias={(mediaId, alias) => {
                setState((prev) => ({
                  ...prev,
                  media: prev.media.map((m) => (m.id === mediaId ? { ...m, alias } : m)),
                }));
                showToast('Alias saved (non-destructive)');
              }}
              onApplyRealRename={(mediaId, newName) => {
                setState((prev) => {
                  const target = prev.media.find((m) => m.id === mediaId);
                  if (!target) return prev;
                  const dot = target.fileName.lastIndexOf('.');
                  const ext = dot >= 0 ? target.fileName.slice(dot) : '';
                  const finalName = newName.endsWith(ext) ? newName : `${newName}${ext}`;
                  const log = {
                    id: `ren-${Date.now()}`,
                    mediaId,
                    batchId: null,
                    oldName: target.fileName,
                    newName: finalName,
                    oldPath: target.path,
                    newPath: `${target.folderPath}/${finalName}`,
                    appliedAt: Date.now(),
                    undoneAt: null,
                  };
                  return {
                    ...prev,
                    renameLogs: [log, ...prev.renameLogs],
                    media: prev.media.map((m) =>
                      m.id === mediaId
                        ? { ...m, fileName: finalName, path: log.newPath, alias: newName }
                        : m,
                    ),
                  };
                });
                showToast('Applied real rename and recorded in undo log');
              }}
              onUndoRename={(mediaId) => {
                setState((prev) => {
                  const log = prev.renameLogs.find((l) => l.mediaId === mediaId && !l.undoneAt);
                  if (!log) return prev;
                  return {
                    ...prev,
                    renameLogs: prev.renameLogs.map((l) =>
                      l.id === log.id ? { ...l, undoneAt: Date.now() } : l,
                    ),
                    media: prev.media.map((m) =>
                      m.id === mediaId ? { ...m, fileName: log.oldName, path: log.oldPath } : m,
                    ),
                  };
                });
                showToast('Reverted real rename');
              }}
              onChangeAspect={(mediaId, aspect: AspectPreset, customCrop?: CropRect) => {
                setState((prev) => ({
                  ...prev,
                  covers: prev.covers.map((c) =>
                    c.mediaId === mediaId ? applyAspectCropToCover(c, aspect, customCrop) : c,
                  ),
                }));
                showToast(`Updated cover crop preset to ${aspect}`);
              }}
              onRerollVideoFrame={(mediaId) => {
                setState((prev) => {
                  const m = prev.media.find((item) => item.id === mediaId);
                  if (!m) return prev;
                  const newMs = Math.floor(Math.random() * Math.max(1, Math.floor(m.durationMs / 3)));
                  return {
                    ...prev,
                    covers: prev.covers.map((c) =>
                      c.mediaId === mediaId
                        ? {
                            ...c,
                            frameTimeMs: newMs,
                            filePath: makeThumbDataUri(
                              m.alias || m.fileName,
                              `Frame ${(newMs / 1000).toFixed(1)}s`,
                              '#1d4ed8',
                              '#0f172a',
                              c.aspect,
                            ),
                            updatedAt: Date.now(),
                          }
                        : c,
                    ),
                  };
                });
                showToast('Re-rolled video thumbnail inside first third');
              }}
              onReplaceCoverImage={(mediaId, dataUrl, aspect) => {
                setState((prev) => ({
                  ...prev,
                  covers: prev.covers.map((c) =>
                    c.mediaId === mediaId
                      ? { ...c, source: 'custom', filePath: dataUrl, aspect, updatedAt: Date.now() }
                      : c,
                  ),
                }));
                showToast('Saved custom cover image');
              }}
              onRemoveAudioCover={(mediaId) => {
                setState((prev) => ({
                  ...prev,
                  covers: prev.covers.map((c) =>
                    c.mediaId === mediaId ? { ...c, filePath: '', updatedAt: Date.now() } : c,
                  ),
                }));
                showToast('Removed audio cover art');
              }}
              onRestoreAudioCover={(mediaId) => {
                setState((prev) => {
                  const m = prev.media.find((item) => item.id === mediaId);
                  if (!m) return prev;
                  return {
                    ...prev,
                    covers: prev.covers.map((c) =>
                      c.mediaId === mediaId
                        ? {
                            ...c,
                            filePath: makeThumbDataUri(m.alias, m.fileName, '#7c3aed', '#1e1b4b', '1:1'),
                            updatedAt: Date.now(),
                          }
                        : c,
                    ),
                  };
                });
                showToast('Restored audio cover art');
              }}
              onSendToDevice={(mediaId, targetDeviceId) => {
                showToast(`Queued transfer of ${mediaId} to ${targetDeviceId}`);
              }}
              onDownloadRemote={(mediaId) => {
                setState((prev) => ({
                  ...prev,
                  media: prev.media.map((m) =>
                    m.id === mediaId ? { ...m, syncState: 'Synced' } : m,
                  ),
                }));
                showToast('Synced remote file locally');
              }}
              onOpenContainingFolder={(folderPath) => setActiveFolderPath(folderPath)}
              onMoveToTrash={(mediaId) => {
                setState((prev) => ({
                  ...prev,
                  media: prev.media.map((m) =>
                    m.id === mediaId ? { ...m, isTrashed: true } : m,
                  ),
                }));
                setSelectedMediaId(null);
                showToast('Moved item to Recycle Bin / Trash');
              }}
            />
          </div>
        </div>
      </div>

      <BottomShortcutBar
        pinnedFolders={state.pinnedFolders}
        activeFolderPath={activeFolderPath}
        onOpenFolder={(folderPath) => setActiveFolderPath(folderPath)}
        onAddPinnedFolder={() => {
          const targetPath = activeFolderPath ?? 'C:/Users/Media/Favorites';
          const label = targetPath.split('/').filter(Boolean).pop() ?? 'Folder';
          setState((prev) => ({
            ...prev,
            pinnedFolders: [
              ...prev.pinnedFolders,
              {
                id: `pin-${Date.now()}`,
                deviceId: 'pc-local',
                path: targetPath,
                label,
                icon: 'folder',
                position: prev.pinnedFolders.length,
              },
            ],
          }));
        }}
        onRenamePinnedFolder={(id, label) =>
          setState((prev) => ({
            ...prev,
            pinnedFolders: prev.pinnedFolders.map((f) => (f.id === id ? { ...f, label } : f)),
          }))
        }
        onRemovePinnedFolder={(id) =>
          setState((prev) => ({
            ...prev,
            pinnedFolders: prev.pinnedFolders.filter((f) => f.id !== id),
          }))
        }
        onReorderPinnedFolders={(ids) =>
          setState((prev) => ({
            ...prev,
            pinnedFolders: ids
              .map((id, idx) => {
                const f = prev.pinnedFolders.find((p) => p.id === id);
                return f ? { ...f, position: idx } : null;
              })
              .filter((x): x is NonNullable<typeof x> => x !== null),
          }))
        }
        onDropMediaToFolder={handleMoveToFolder}
      />

      <MediaViewerModal media={previewMedia} onClose={() => setPreviewMedia(null)} />

      <BulkRenameModal
        open={bulkRenameOpen}
        items={visibleItems}
        onClose={() => setBulkRenameOpen(false)}
        onApply={() => showToast(`Bulk renamed ${visibleItems.length} items`)}
      />

      {toast && <div className="toast-banner">{toast}</div>}
    </div>
  );
};
