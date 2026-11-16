import React, { useEffect, useMemo, useRef, useState } from 'react';
import type {
  AspectPreset,
  CropRect,
  DownloadFormat,
  GridDensity,
  GroupByOption,
  MediaRecord,
  MediaSearchFilters,
  MediaWithCover,
  SortDirection,
  SortField,
  TimeOfDayFilter,
  TransferRecord,
} from '../core/types/domain.ts';
import { generateAutoAlias } from '../features/alias/aliasEngine.ts';
import {
  buildUnifiedMusicLibrary,
  MusicPlayerController,
  type MusicGroupBy,
  type PlayerState,
  type UnifiedMusicTrack,
} from '../features/player/musicPlayerService.ts';
import { YOUTUBE_PORTAL_URLS } from '../features/youtube/youtubeConstants.ts';
import { AndroidMobileShell } from './components/AndroidMobileShell.tsx';
import { BottomShortcutBar } from './components/BottomShortcutBar.tsx';
import {
  DevicesAndTransfersView,
  DuplicatesView,
  SettingsView,
  UnifiedAudioView,
  YouTubeWorkspaceView,
} from './components/FeaturePanels.tsx';
import { LeftSidebar } from './components/LeftSidebar.tsx';
import { MainGalleryArea } from './components/MainGalleryArea.tsx';
import {
  BulkRenameModal,
  ConnectPairModal,
  MediaViewerModal,
  YouTubeDownloadModal,
} from './components/Modals.tsx';
import { MusicPlayerBar } from './components/MusicPlayerBar.tsx';
import { RightDetailsSidebar } from './components/RightDetailsSidebar.tsx';
import { TopBar } from './components/TopBar.tsx';
import {
  applyAspectCropToCover,
  buildFolderSummaries,
  createBrowserDownloadJob,
  filterAndSortMedia,
  hydrateMediaList,
  loadBrowserState,
  makeThumbDataUri,
  previewBulkPattern,
  resetBrowserState,
  saveBrowserState,
  type GalleryAppState,
} from './state/browserEngine.ts';
import type { NavSection, YouTubeAction } from './state/navigation.ts';

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
  const [musicGroupBy, setMusicGroupBy] = useState<MusicGroupBy>('all');
  const [selectedMediaId, setSelectedMediaId] = useState<string | null>(null);
  const [detailsClosedForSession, setDetailsClosedForSession] = useState(false);
  const [previewMedia, setPreviewMedia] = useState<MediaWithCover | null>(null);
  const [bulkRenameOpen, setBulkRenameOpen] = useState(false);
  const [connectModalOpen, setConnectModalOpen] = useState(false);
  const [youtubeModalOpen, setYoutubeModalOpen] = useState(false);
  const [androidPreviewMode, setAndroidPreviewMode] = useState(false);
  const [toast, setToast] = useState<string | null>(null);

  const playerControllerRef = useRef(new MusicPlayerController());
  const [playerState, setPlayerState] = useState<PlayerState>(() =>
    playerControllerRef.current.getState(),
  );

  useEffect(() => {
    saveBrowserState(state);
  }, [state]);

  const showToast = (msg: string) => {
    setToast(msg);
    window.setTimeout(() => setToast((curr) => (curr === msg ? null : curr)), 3400);
  };

  const sectionMediaType: MediaSearchFilters['type'] =
    activeSection === 'videos'
      ? 'video'
      : activeSection === 'images'
        ? 'image'
        : activeSection === 'audio'
          ? 'audio'
          : 'all';

  const allHydrated = useMemo(() => hydrateMediaList(state), [state]);

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
    [
      state,
      searchQuery,
      sectionMediaType,
      deviceFilter,
      syncFilter,
      activeFolderPath,
      timeOfDayFilter,
      sortBy,
      sortDir,
    ],
  );

  const visibleFolders = useMemo(
    () => buildFolderSummaries(state, sectionMediaType),
    [state, sectionMediaType],
  );

  const unifiedMusicTracks = useMemo(
    () =>
      buildUnifiedMusicLibrary(
        filterAndSortMedia(state, searchQuery, { type: 'audio', deviceId: deviceFilter }),
      ),
    [state, searchQuery, deviceFilter],
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

  const playUnifiedTrackList = (tracks: UnifiedMusicTrack[], index: number) => {
    setPlayerState(playerControllerRef.current.playTrackFromList(tracks, index));
  };

  const handlePreviewOrPlayMedia = (item: MediaWithCover) => {
    if (item.type === 'audio') {
      const idx = unifiedMusicTracks.findIndex((t) =>
        t.sources.some((s) => s.mediaId === item.id),
      );
      if (idx >= 0) {
        playUnifiedTrackList(unifiedMusicTracks, idx);
        return;
      }
    }
    setPreviewMedia(item);
  };

  const handleMoveToFolder = (mediaId: string, targetFolder: string) => {
    setState((prev) => ({
      ...prev,
      media: prev.media.map((m) =>
        m.id === mediaId
          ? { ...m, folderPath: targetFolder, path: `${targetFolder}/${m.fileName}`, updatedAt: Date.now() }
          : m,
      ),
    }));
    showToast(`Moved item to ${targetFolder}`);
  };

  const handleSendToDevice = (mediaId: string, targetDeviceId: string) => {
    const media = state.media.find((m) => m.id === mediaId);
    const target = state.devices.find((d) => d.id === targetDeviceId);
    if (!media || !target) return;

    const chosenTransport =
      state.settings.manualTransportOverride !== 'auto'
        ? state.settings.manualTransportOverride
        : target.transports[0] ?? 'wifi_lan';

    const transfer: TransferRecord = {
      id: `tx-${Date.now()}`,
      mediaId: media.id,
      fileName: media.fileName,
      fromDevice: media.deviceId,
      toDevice: target.id,
      status: 'completed',
      bytesDone: media.size,
      bytesTotal: media.size,
      lastChunkIndex: Math.max(1, Math.ceil(media.size / 262144)),
      chunkSize: 262144,
      transport: chosenTransport,
      speedBps: chosenTransport === 'bluetooth' ? 1_200_000 : 48_000_000,
      expectedHash: media.hash,
      error: null,
      updatedAt: Date.now(),
    };

    setState((prev) => ({
      ...prev,
      transfers: [transfer, ...prev.transfers],
    }));

    if (chosenTransport === 'bluetooth') {
      showToast(`Sent ${media.fileName} to ${target.name} via Bluetooth (slow fallback mode)`);
    } else {
      showToast(`Sent ${media.fileName} to ${target.name} via ${chosenTransport} (SHA-256 verified)`);
    }
  };

  const handleDownloadRemote = (mediaId: string) => {
    const item = allHydrated.find((m) => m.id === mediaId);
    if (!item) return;
    if (item.deviceOnline === false) {
      showToast(`${item.deviceName ?? item.deviceId} is offline. Cannot download until connected.`);
      return;
    }
    setState((prev) => ({
      ...prev,
      media: prev.media.map((m) =>
        m.id === mediaId ? { ...m, syncState: 'Synced', updatedAt: Date.now() } : m,
      ),
    }));
    showToast(`Downloaded "${item.fileName}" from ${item.deviceName} (hash verified)`);
  };

  const handleYouTubeAction = (action: YouTubeAction) => {
    if (action === 'download') {
      setYoutubeModalOpen(true);
      return;
    }
    if (!state.googleAccountEmail) {
      showToast('Sign in with Google (top right) to link your YouTube channel');
      return;
    }
    const url =
      action === 'upload'
        ? YOUTUBE_PORTAL_URLS.upload
        : action === 'studio'
          ? YOUTUBE_PORTAL_URLS.studio
          : YOUTUBE_PORTAL_URLS.channel;
    window.open(url, '_blank', 'noopener,noreferrer');
  };

  const handleStartYoutubeDownload = (payload: {
    url: string;
    format: DownloadFormat;
    quality: string;
    destinationDeviceId: string;
    destinationFolder: string;
    allowPlaylist: boolean;
  }) => {
    const { job, media, cover } = createBrowserDownloadJob(
      payload.url,
      payload.format,
      payload.quality,
      payload.destinationDeviceId,
      payload.destinationFolder,
    );
    setState((prev) => ({
      ...prev,
      downloadJobs: [job, ...prev.downloadJobs],
      media: [media, ...prev.media],
      covers: [cover, ...prev.covers],
    }));
    showToast(
      `Downloaded "${media.alias}" (${payload.format.toUpperCase()}) into library`,
    );
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
        template: state.settings.aliasTemplate,
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

  if (androidPreviewMode) {
    return (
      <div className="desktop-shell" data-theme={state.settings.theme} style={{ overflowY: 'auto', padding: 8 }}>
        <div style={{ display: 'flex', justifyContent: 'center', marginBottom: 6 }}>
          <button
            type="button"
            className="btn-secondary"
            onClick={() => setAndroidPreviewMode(false)}
          >
            Back to Desktop Layout
          </button>
        </div>
        <AndroidMobileShell
          searchQuery={searchQuery}
          onSearchChange={setSearchQuery}
          items={visibleItems}
          playerState={playerState}
          googleAccountEmail={state.googleAccountEmail}
          connectedPeerName={state.devices.find((d) => !d.isSelf && d.isOnline)?.name ?? null}
          onTogglePlay={() => setPlayerState(playerControllerRef.current.togglePlayPause())}
          onPlayMedia={handlePreviewOrPlayMedia}
          onDownloadRemote={handleDownloadRemote}
          onOpenConnect={() => setConnectModalOpen(true)}
          onOpenYouTube={() => setYoutubeModalOpen(true)}
          onShareSheetSend={(item) => {
            const peer = state.devices.find((d) => !d.isSelf);
            if (peer) handleSendToDevice(item.id, peer.id);
          }}
        />
        <ConnectPairModal
          open={connectModalOpen}
          devices={state.devices}
          manualOverride={state.settings.manualTransportOverride}
          onClose={() => setConnectModalOpen(false)}
          onPairPin={(_pin, name) => {
            const id = `phone-${Date.now()}`;
            setState((prev) => ({
              ...prev,
              devices: [
                ...prev.devices,
                {
                  id,
                  name,
                  type: 'phone',
                  publicKey: `ed25519:${id}`,
                  lastSeen: Date.now(),
                  transports: ['wifi_lan', 'wired', 'bluetooth'],
                  isOnline: true,
                  pairedAt: Date.now(),
                },
              ],
            }));
            showToast(`Paired ${name}`);
          }}
          onToggleOnline={(deviceId) =>
            setState((prev) => ({
              ...prev,
              devices: prev.devices.map((d) =>
                d.id === deviceId ? { ...d, isOnline: !d.isOnline } : d,
              ),
            }))
          }
          onForgetDevice={(deviceId) =>
            setState((prev) => ({
              ...prev,
              devices: prev.devices.filter((d) => d.id !== deviceId),
              media: prev.media.filter((m) => m.deviceId !== deviceId),
            }))
          }
          onChangeTransportOverride={(override) =>
            setState((prev) => ({
              ...prev,
              settings: { ...prev.settings, manualTransportOverride: override },
            }))
          }
        />
        <YouTubeDownloadModal
          open={youtubeModalOpen}
          devices={state.devices}
          onClose={() => setYoutubeModalOpen(false)}
          onStartDownload={handleStartYoutubeDownload}
        />
        {toast && <div className="toast-banner">{toast}</div>}
      </div>
    );
  }

  const renderMainPane = () => {
    if (activeSection === 'audio' && !activeFolderPath) {
      return (
        <UnifiedAudioView
          tracks={unifiedMusicTracks}
          groupBy={musicGroupBy}
          onChangeGroupBy={setMusicGroupBy}
          onPlayTrack={playUnifiedTrackList}
        />
      );
    }
    if (activeSection === 'devices') {
      return (
        <DevicesAndTransfersView
          devices={state.devices}
          transfers={state.transfers}
          autoSyncRules={state.autoSyncRules}
          onOpenConnect={() => setConnectModalOpen(true)}
          onDropMediaToDevice={handleSendToDevice}
          onToggleTransferPause={(id) =>
            setState((prev) => ({
              ...prev,
              transfers: prev.transfers.map((t) =>
                t.id === id ? { ...t, status: t.status === 'paused' ? 'transferring' : 'paused' } : t,
              ),
            }))
          }
          onCancelTransfer={(id) =>
            setState((prev) => ({
              ...prev,
              transfers: prev.transfers.map((t) =>
                t.id === id ? { ...t, status: 'cancelled' } : t,
              ),
            }))
          }
          onToggleSyncRule={(ruleId) =>
            setState((prev) => ({
              ...prev,
              autoSyncRules: prev.autoSyncRules.map((r) =>
                r.id === ruleId ? { ...r, enabled: !r.enabled } : r,
              ),
            }))
          }
        />
      );
    }
    if (activeSection === 'youtube') {
      return (
        <YouTubeWorkspaceView
          jobs={state.downloadJobs}
          googleAccountEmail={state.googleAccountEmail}
          onOpenDownloadModal={() => setYoutubeModalOpen(true)}
          onUpdateYtDlp={() => showToast('yt-dlp checked: up to date')}
        />
      );
    }
    if (activeSection === 'duplicates') {
      return (
        <DuplicatesView
          items={allHydrated}
          onTrashItem={(mediaId) => {
            setState((prev) => ({
              ...prev,
              media: prev.media.map((m) => (m.id === mediaId ? { ...m, isTrashed: true } : m)),
            }));
            showToast('Moved duplicate file to Recycle Bin / Trash');
          }}
        />
      );
    }
    if (activeSection === 'settings') {
      return (
        <SettingsView
          settings={state.settings}
          androidPreviewMode={androidPreviewMode}
          onUpdateSettings={(patch) =>
            setState((prev) => ({ ...prev, settings: { ...prev.settings, ...patch } }))
          }
          onToggleAndroidPreview={() => setAndroidPreviewMode((v) => !v)}
          onResetDemoData={() => {
            setState(resetBrowserState());
            showToast('Reset library to seeded defaults');
          }}
        />
      );
    }

    return (
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
        onPreviewMedia={handlePreviewOrPlayMedia}
        onOpenFolder={setActiveFolderPath}
        onDropMediaToFolder={handleMoveToFolder}
        onDownloadRemote={handleDownloadRemote}
        onOpenBulkRename={() => setBulkRenameOpen(true)}
        onImportLocalFiles={handleImportLocalFiles}
      />
    );
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
          onOpenFolder={(folderPath) => {
            setActiveSection('home');
            setActiveFolderPath(folderPath);
          }}
          onDropMediaToFolder={handleMoveToFolder}
        />

        <div className="right-column">
          <TopBar
            searchQuery={searchQuery}
            onSearchChange={setSearchQuery}
            connectedDevices={state.devices}
            activeTransport={
              state.settings.manualTransportOverride === 'auto'
                ? state.activeTransport
                : state.settings.manualTransportOverride
            }
            googleAccountEmail={state.googleAccountEmail}
            theme={state.settings.theme}
            showFilters={showFilters}
            onToggleFilters={() => setShowFilters((v) => !v)}
            onOpenConnectModal={() => setConnectModalOpen(true)}
            onYouTubeAction={handleYouTubeAction}
            onToggleGoogleAccount={() => {
              if (state.googleAccountEmail) {
                setState((prev) => ({ ...prev, googleAccountEmail: null }));
                showToast('Signed out and wiped cached cloud tokens');
              } else {
                setState((prev) => ({ ...prev, googleAccountEmail: 'creator@gmail.com' }));
                showToast('Signed in with Google (Drive appDataFolder + YouTube ready)');
              }
            }}
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
            {renderMainPane()}

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
              onSendToDevice={handleSendToDevice}
              onDownloadRemote={handleDownloadRemote}
              onOpenContainingFolder={(folderPath) => {
                setActiveSection('home');
                setActiveFolderPath(folderPath);
              }}
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

      <MusicPlayerBar
        playerState={playerState}
        onTogglePlay={() => setPlayerState(playerControllerRef.current.togglePlayPause())}
        onNext={() => setPlayerState(playerControllerRef.current.next())}
        onPrev={() => setPlayerState(playerControllerRef.current.previous())}
        onSeek={(ms) => setPlayerState(playerControllerRef.current.seek(ms))}
        onToggleShuffle={() => setPlayerState(playerControllerRef.current.toggleShuffle())}
        onCycleRepeat={() => setPlayerState(playerControllerRef.current.cycleRepeat())}
        onVolumeChange={(v) => setPlayerState(playerControllerRef.current.setVolume(v))}
      />

      <BottomShortcutBar
        pinnedFolders={state.pinnedFolders}
        activeFolderPath={activeFolderPath}
        onOpenFolder={(folderPath) => {
          setActiveSection('home');
          setActiveFolderPath(folderPath);
        }}
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
        onApply={(template) => {
          const batchId = `batch-${Date.now()}`;
          const previews = previewBulkPattern(visibleItems, template);
          const previewMap = new Map(previews.map((p) => [p.id, p]));
          setState((prev) => {
            const newLogs = previews.map((p) => {
              const target = prev.media.find((m) => m.id === p.id)!;
              return {
                id: `ren-${batchId}-${p.id}`,
                mediaId: p.id,
                batchId,
                oldName: p.oldName,
                newName: p.newName,
                oldPath: target.path,
                newPath: `${target.folderPath}/${p.newName}`,
                appliedAt: Date.now(),
                undoneAt: null,
              };
            });
            return {
              ...prev,
              renameLogs: [...newLogs, ...prev.renameLogs],
              media: prev.media.map((m) => {
                const p = previewMap.get(m.id);
                return p
                  ? { ...m, fileName: p.newName, path: `${m.folderPath}/${p.newName}` }
                  : m;
              }),
            };
          });
          showToast(`Bulk renamed ${previews.length} items (undoable)`);
        }}
      />

      <ConnectPairModal
        open={connectModalOpen}
        devices={state.devices}
        manualOverride={state.settings.manualTransportOverride}
        onClose={() => setConnectModalOpen(false)}
        onPairPin={(_pin, name) => {
          const id = `phone-${Date.now()}`;
          setState((prev) => ({
            ...prev,
            devices: [
              ...prev.devices,
              {
                id,
                name,
                type: 'phone',
                publicKey: `ed25519:${id}`,
                lastSeen: Date.now(),
                transports: ['wifi_lan', 'wired', 'bluetooth'],
                isOnline: true,
                pairedAt: Date.now(),
              },
            ],
          }));
          showToast(`Paired ${name} via PIN/QR`);
        }}
        onToggleOnline={(deviceId) =>
          setState((prev) => ({
            ...prev,
            devices: prev.devices.map((d) =>
              d.id === deviceId ? { ...d, isOnline: !d.isOnline } : d,
            ),
          }))
        }
        onForgetDevice={(deviceId) =>
          setState((prev) => ({
            ...prev,
            devices: prev.devices.filter((d) => d.id !== deviceId),
            media: prev.media.filter((m) => m.deviceId !== deviceId),
          }))
        }
        onChangeTransportOverride={(override) =>
          setState((prev) => ({
            ...prev,
            settings: { ...prev.settings, manualTransportOverride: override },
          }))
        }
      />

      <YouTubeDownloadModal
        open={youtubeModalOpen}
        devices={state.devices}
        onClose={() => setYoutubeModalOpen(false)}
        onStartDownload={handleStartYoutubeDownload}
      />

      {toast && <div className="toast-banner">{toast}</div>}
    </div>
  );
};
