import { cleanFileNameForAlias, formatDateToken, formatTimePeriodToken, generateAutoAlias } from '../../features/alias/aliasEngine.ts';
import type {
  AppSettings,
  AspectPreset,
  AutoSyncRuleRecord,
  CoverRecord,
  CropRect,
  DeviceRecord,
  DownloadFormat,
  DownloadJobRecord,
  FolderCardSummary,
  MediaRecord,
  MediaSearchFilters,
  MediaWithCover,
  PinnedFolderRecord,
  RenameLogRecord,
  SavedSearchRecord,
  TransferRecord,
  TransportKind,
} from '../../core/types/domain.ts';

export interface GalleryAppState {
  devices: DeviceRecord[];
  media: MediaRecord[];
  covers: CoverRecord[];
  transfers: TransferRecord[];
  pinnedFolders: PinnedFolderRecord[];
  renameLogs: RenameLogRecord[];
  downloadJobs: DownloadJobRecord[];
  savedSearches: SavedSearchRecord[];
  autoSyncRules: AutoSyncRuleRecord[];
  settings: AppSettings;
  googleAccountEmail: string | null;
  activeTransport: TransportKind | null;
}

const STORAGE_KEY = 'media_gallery_state_v1';

export function makeThumbDataUri(
  title: string,
  subtitle: string,
  bgStart: string,
  bgEnd: string,
  aspect: AspectPreset = '16:9',
): string {
  const esc = (s: string) =>
    s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
  const dims =
    aspect === '9:16'
      ? { w: 270, h: 480 }
      : aspect === '1:1'
        ? { w: 360, h: 360 }
        : aspect === '4:3'
          ? { w: 400, h: 300 }
          : { w: 480, h: 270 };
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${dims.w}" height="${dims.h}" viewBox="0 0 ${dims.w} ${dims.h}">
    <defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0%" stop-color="${bgStart}"/><stop offset="100%" stop-color="${bgEnd}"/>
    </linearGradient></defs>
    <rect width="${dims.w}" height="${dims.h}" fill="url(#g)"/>
    <circle cx="${dims.w - 50}" cy="52" r="28" fill="rgba(255,255,255,0.14)"/>
    <text x="20" y="${dims.h - 46}" fill="#ffffff" font-family="Segoe UI, sans-serif" font-size="18" font-weight="700">${esc(title.slice(0, 26))}</text>
    <text x="20" y="${dims.h - 22}" fill="#e2e8f0" font-family="Segoe UI, sans-serif" font-size="12">${esc(subtitle.slice(0, 34))}</text>
  </svg>`;
  return `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
}

function createInitialSeedState(): GalleryAppState {
  const now = Date.now();
  const march2026 = Date.UTC(2026, 2, 14, 17, 25, 0);
  const april2026 = Date.UTC(2026, 3, 2, 9, 15, 0);

  const devices: DeviceRecord[] = [
    {
      id: 'pc-local',
      name: 'Windows Workstation',
      type: 'pc',
      publicKey: 'ed25519:pc-local-fingerprint-demo',
      lastSeen: now,
      transports: ['wired', 'wifi_lan', 'bluetooth'],
      host: '127.0.0.1',
      port: 9443,
      isSelf: true,
      isOnline: true,
      pairedAt: now,
    },
    {
      id: 'phone-pixel',
      name: 'Pixel 9 Pro',
      type: 'phone',
      publicKey: 'ed25519:pixel9-fingerprint-demo',
      lastSeen: now - 15000,
      transports: ['wifi_lan', 'wired', 'bluetooth'],
      host: '192.168.1.44',
      port: 9443,
      isSelf: false,
      isOnline: true,
      pairedAt: now - 86400000,
    },
  ];

  const seedMediaSpecs: Array<Partial<MediaRecord> & { id: string; bg: [string, string] }> = [
    {
      id: 'm-1',
      deviceId: 'pc-local',
      path: 'C:/Users/Media/Camera/IMG_2031.jpg',
      folderPath: 'C:/Users/Media/Camera',
      fileName: 'IMG_2031.jpg',
      type: 'image',
      mime: 'image/jpeg',
      size: 4_820_110,
      createdAt: march2026,
      modifiedAt: march2026,
      width: 4032,
      height: 3024,
      gpsLat: -8.7185,
      gpsLon: 115.1686,
      syncState: 'Local',
      bg: ['#0284c7', '#0f172a'],
    },
    {
      id: 'm-2',
      deviceId: 'pc-local',
      path: 'C:/Users/Media/Trips/VID_0042.mp4',
      folderPath: 'C:/Users/Media/Trips',
      fileName: 'VID_0042.mp4',
      type: 'video',
      mime: 'video/mp4',
      size: 184_500_000,
      createdAt: march2026 + 3600000,
      modifiedAt: march2026 + 3600000,
      durationMs: 96_000,
      width: 3840,
      height: 2160,
      gpsLat: -8.7185,
      gpsLon: 115.1686,
      syncState: 'Local',
      bg: ['#2563eb', '#1e1b4b'],
    },
    {
      id: 'm-3',
      deviceId: 'pc-local',
      path: 'C:/Users/Media/Camera/sunsetWalk_08.png',
      folderPath: 'C:/Users/Media/Camera',
      fileName: 'sunsetWalk_08.png',
      type: 'image',
      mime: 'image/png',
      size: 6_120_000,
      createdAt: april2026,
      modifiedAt: april2026,
      width: 2560,
      height: 1440,
      gpsLat: 35.6762,
      gpsLon: 139.6503,
      syncState: 'Local',
      bg: ['#ea580c', '#451a03'],
    },
    {
      id: 'm-4',
      deviceId: 'pc-local',
      path: 'C:/Users/Media/Music/midnight_echoes.mp3',
      folderPath: 'C:/Users/Media/Music',
      fileName: 'midnight_echoes.mp3',
      type: 'audio',
      mime: 'audio/mpeg',
      size: 8_940_000,
      createdAt: april2026 + 7200000,
      modifiedAt: april2026 + 7200000,
      durationMs: 214_000,
      tags: { artist: 'Nova Drift', title: 'Midnight Echoes', album: 'Neon Skies' },
      syncState: 'Local',
      bg: ['#7c3aed', '#1e1b4b'],
    },
    {
      id: 'm-5',
      deviceId: 'phone-pixel',
      path: '/storage/emulated/0/DCIM/Camera/VID_20260405_surfClip.mp4',
      folderPath: '/storage/emulated/0/DCIM/Camera',
      fileName: 'VID_20260405_surfClip.mp4',
      type: 'video',
      mime: 'video/mp4',
      size: 245_000_000,
      createdAt: april2026 + 86400000,
      modifiedAt: april2026 + 86400000,
      durationMs: 48_000,
      width: 1920,
      height: 1080,
      gpsLat: -33.8915,
      gpsLon: 151.2767,
      syncState: 'Remote',
      bg: ['#0d9488', '#042f2e'],
    },
    {
      id: 'm-6',
      deviceId: 'phone-pixel',
      path: '/storage/emulated/0/Music/solar_flare.flac',
      folderPath: '/storage/emulated/0/Music',
      fileName: 'solar_flare.flac',
      type: 'audio',
      mime: 'audio/flac',
      size: 28_400_000,
      createdAt: april2026 + 172800000,
      modifiedAt: april2026 + 172800000,
      durationMs: 198_000,
      tags: { artist: 'Kairo', title: 'Solar Flare', album: 'Horizon' },
      syncState: 'Remote',
      bg: ['#db2777', '#3b0764'],
    },
  ];

  const covers: CoverRecord[] = [];
  const media: MediaRecord[] = seedMediaSpecs.map((spec, idx) => {
    const dev = devices.find((d) => d.id === spec.deviceId)!;
    const { autoAlias, resolvedPlace } = generateAutoAlias({
      fileName: spec.fileName!,
      type: spec.type!,
      createdAt: spec.createdAt!,
      modifiedAt: spec.modifiedAt!,
      gpsLat: spec.gpsLat ?? null,
      gpsLon: spec.gpsLon ?? null,
      tags: spec.tags,
      deviceName: dev.name,
    });
    const frameTimeMs =
      spec.type === 'video' ? Math.floor(((spec.durationMs ?? 0) / 3) * (0.25 + idx * 0.1)) : 0;
    const coverId = `cov-${spec.id}`;
    const aspect: AspectPreset = spec.type === 'audio' ? '1:1' : '16:9';
    covers.push({
      id: coverId,
      mediaId: spec.id,
      source: 'generated',
      filePath: makeThumbDataUri(
        autoAlias,
        spec.type === 'video' ? `Frame ${(frameTimeMs / 1000).toFixed(1)}s` : spec.fileName!,
        spec.bg[0],
        spec.bg[1],
        aspect,
      ),
      cloudId: null,
      aspect,
      crop: null,
      frameTimeMs,
      updatedAt: now,
    });

    return {
      id: spec.id,
      deviceId: spec.deviceId!,
      path: spec.path!,
      folderPath: spec.folderPath!,
      fileName: spec.fileName!,
      type: spec.type!,
      mime: spec.mime!,
      size: spec.size!,
      hash: `sha256-seed-${spec.id}`,
      partialHash: `partial-${spec.id}`,
      createdAt: spec.createdAt!,
      modifiedAt: spec.modifiedAt!,
      durationMs: spec.durationMs ?? 0,
      width: spec.width ?? 0,
      height: spec.height ?? 0,
      gpsLat: spec.gpsLat ?? null,
      gpsLon: spec.gpsLon ?? null,
      place: resolvedPlace,
      tags: spec.tags ?? {},
      alias: autoAlias,
      autoAlias,
      coverId,
      syncState: spec.syncState ?? 'Local',
      isDuplicate: false,
      duplicateGroupId: null,
      isTrashed: false,
      trashedPath: null,
      updatedAt: now,
    };
  });

  const pinnedFolders: PinnedFolderRecord[] = [
    { id: 'pin-camera', deviceId: 'pc-local', path: 'C:/Users/Media/Camera', label: 'Camera', icon: 'folder', position: 0 },
    { id: 'pin-trips', deviceId: 'pc-local', path: 'C:/Users/Media/Trips', label: 'Trips', icon: 'folder', position: 1 },
    { id: 'pin-music', deviceId: 'pc-local', path: 'C:/Users/Media/Music', label: 'Music', icon: 'folder', position: 2 },
    { id: 'pin-youtube', deviceId: 'pc-local', path: 'C:/Users/Media/YouTube', label: 'YouTube', icon: 'folder', position: 3 },
  ];

  return {
    devices,
    media,
    covers,
    transfers: [],
    pinnedFolders,
    renameLogs: [],
    downloadJobs: [],
    savedSearches: [
      {
        id: 'ss-bali',
        name: '2026 Beach Memories',
        query: '2026 beach',
        filters: {},
        createdAt: now,
      },
    ],
    autoSyncRules: [
      {
        id: 'rule-cam',
        name: 'Keep Phone Camera synced to PC',
        sourceDeviceId: 'phone-pixel',
        targetDeviceId: 'pc-local',
        folderPattern: 'DCIM/Camera',
        mediaTypes: ['image', 'video'],
        enabled: true,
        createdAt: now,
      },
    ],
    settings: {
      sidebarWidth: 248,
      sidebarCollapsed: false,
      theme: 'dark',
      transportPriority: ['wired', 'wifi_lan', 'wifi_direct', 'bluetooth'],
      manualTransportOverride: 'auto',
      aliasTemplate: '{date} {place} {name}',
      scanFolders: ['C:/Users/Media'],
      excludedFolders: [],
      onlineGeocodingEnabled: false,
      youtubeDefaultVideoFolder: 'C:/Users/Media/YouTube',
      youtubeDefaultAudioFolder: 'C:/Users/Media/Music',
    },
    googleAccountEmail: null,
    activeTransport: 'wifi_lan',
  };
}

export function loadBrowserState(): GalleryAppState {
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (!raw) return createInitialSeedState();
    const parsed = JSON.parse(raw) as Partial<GalleryAppState>;
    const seed = createInitialSeedState();
    return {
      ...seed,
      ...parsed,
      settings: { ...seed.settings, ...(parsed.settings ?? {}) },
    };
  } catch {
    return createInitialSeedState();
  }
}

export function saveBrowserState(state: GalleryAppState): void {
  try {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
  } catch {
    // Ignore storage quota errors in preview mode.
  }
}

export function resetBrowserState(): GalleryAppState {
  const seed = createInitialSeedState();
  saveBrowserState(seed);
  return seed;
}

export function hydrateMediaList(state: GalleryAppState): MediaWithCover[] {
  const deviceMap = new Map(state.devices.map((d) => [d.id, d]));
  const coverMap = new Map(state.covers.map((c) => [c.mediaId, c]));
  return state.media.map((m) => {
    const dev = deviceMap.get(m.deviceId);
    return {
      ...m,
      cover: coverMap.get(m.id) ?? null,
      deviceName: dev?.name,
      deviceType: dev?.type,
      deviceOnline: dev?.isOnline ?? false,
    };
  });
}

export function filterAndSortMedia(
  state: GalleryAppState,
  queryText: string,
  filters: MediaSearchFilters,
): MediaWithCover[] {
  const tokens = queryText
    .trim()
    .toLowerCase()
    .split(/\s+/)
    .filter(Boolean);

  const hydrated = hydrateMediaList(state).filter((item) => {
    if (!filters.includeTrashed && item.isTrashed) return false;
    if (filters.type && filters.type !== 'all' && item.type !== filters.type) return false;
    if (filters.deviceId && filters.deviceId !== 'all' && item.deviceId !== filters.deviceId) return false;
    if (filters.syncState && filters.syncState !== 'all' && item.syncState !== filters.syncState) return false;
    if (filters.folderPath && item.folderPath !== filters.folderPath) return false;
    if (filters.dateFrom != null && item.createdAt < filters.dateFrom) return false;
    if (filters.dateTo != null && item.createdAt > filters.dateTo) return false;
    if (filters.minSize != null && item.size < filters.minSize) return false;
    if (filters.minDurationMs != null && item.durationMs < filters.minDurationMs) return false;
    if (filters.minWidth != null && item.width < filters.minWidth) return false;
    if (filters.timeOfDay && filters.timeOfDay !== 'all') {
      if (formatTimePeriodToken(item.createdAt) !== filters.timeOfDay) return false;
    }

    if (tokens.length > 0) {
      const haystack = [
        item.fileName,
        item.alias,
        item.autoAlias,
        item.place ?? '',
        item.folderPath,
        item.deviceName ?? item.deviceId,
        formatDateToken(item.createdAt, true),
        JSON.stringify(item.tags ?? {}),
      ]
        .join(' ')
        .toLowerCase();
      return tokens.every((tok) => haystack.includes(tok));
    }
    return true;
  });

  const sortField = filters.sortBy ?? 'date';
  const dir = filters.sortDir === 'asc' ? 1 : -1;

  hydrated.sort((a, b) => {
    switch (sortField) {
      case 'name':
        return (a.alias || a.fileName).localeCompare(b.alias || b.fileName) * dir;
      case 'size':
        return (a.size - b.size) * dir;
      case 'duration':
        return (a.durationMs - b.durationMs) * dir;
      case 'type':
        return a.type.localeCompare(b.type) * dir;
      case 'date':
      default:
        return (a.createdAt - b.createdAt) * dir;
    }
  });

  return hydrated;
}

export function buildFolderSummaries(
  state: GalleryAppState,
  typeFilter?: MediaRecord['type'] | 'all',
): FolderCardSummary[] {
  const hydrated = hydrateMediaList(state).filter(
    (m) => !m.isTrashed && (!typeFilter || typeFilter === 'all' || m.type === typeFilter),
  );
  const byFolder = new Map<string, MediaWithCover[]>();
  for (const item of hydrated) {
    const key = `${item.deviceId}::${item.folderPath}`;
    const list = byFolder.get(key) ?? [];
    list.push(item);
    byFolder.set(key, list);
  }
  return Array.from(byFolder.entries()).map(([, items]) => {
    const folderPath = items[0].folderPath;
    const segments = folderPath.split('/').filter(Boolean);
    return {
      path: folderPath,
      name: segments[segments.length - 1] ?? folderPath,
      deviceId: items[0].deviceId,
      itemCount: items.length,
      previewMedia: items.slice(0, 4),
    };
  });
}

export function applyAspectCropToCover(
  cover: CoverRecord,
  aspect: AspectPreset,
  customCrop?: CropRect,
): CoverRecord {
  const ratioMap: Record<Exclude<AspectPreset, 'custom'>, number> = {
    '4:3': 4 / 3,
    '9:16': 9 / 16,
    '16:9': 16 / 9,
    '1:1': 1,
  };
  const ratio =
    aspect === 'custom'
      ? (customCrop ? customCrop.width / Math.max(1, customCrop.height) : 1.5)
      : ratioMap[aspect];
  return {
    ...cover,
    aspect,
    crop: customCrop ?? { x: 0, y: 0, width: Math.round(1000 * ratio), height: 1000, aspectRatio: ratio },
    updatedAt: Date.now(),
  };
}

export function previewBulkPattern(
  items: MediaWithCover[],
  template: string,
): Array<{ id: string; oldName: string; newName: string }> {
  return items.map((item, idx) => {
    const dot = item.fileName.lastIndexOf('.');
    const ext = dot >= 0 ? item.fileName.slice(dot) : '';
    const dateToken = formatDateToken(item.createdAt, true);
    const clean = cleanFileNameForAlias(item.fileName) || item.fileName.replace(/\.[^.]+$/, '');
    const stem = template
      .replace(/\{date\}/g, dateToken)
      .replace(/\{place\}/g, item.place ?? '')
      .replace(/\{name\}/g, clean)
      .replace(/\{alias\}/g, item.alias || clean)
      .replace(/\{index\}/g, String(idx + 1).padStart(3, '0'))
      .replace(/\s+/g, ' ')
      .trim();
    return {
      id: item.id,
      oldName: item.fileName,
      newName: `${stem || clean}${ext}`,
    };
  });
}

export function createBrowserDownloadJob(
  url: string,
  format: DownloadFormat,
  quality: string,
  destinationDeviceId: string,
  destinationFolder: string,
): { job: DownloadJobRecord; media: MediaRecord; cover: CoverRecord } {
  const now = Date.now();
  const jobId = `yt-${now}`;
  const mediaId = `yt-media-${now}`;
  const cleanTitle = url.includes('v=')
    ? `YouTube Video ${url.split('v=')[1].slice(0, 8)}`
    : `YouTube Track ${String(now).slice(-4)}`;
  const ext = format === 'mp3' ? '.mp3' : '.mp4';
  const fileName = `${cleanTitle.replace(/\s+/g, '_')}${ext}`;
  const mediaType = format === 'mp3' ? 'audio' : 'video';

  const cover: CoverRecord = {
    id: `cov-${mediaId}`,
    mediaId,
    source: 'generated',
    filePath: makeThumbDataUri(cleanTitle, `YouTube ${format.toUpperCase()} · ${quality}`, '#dc2626', '#18181b', format === 'mp3' ? '1:1' : '16:9'),
    cloudId: null,
    aspect: format === 'mp3' ? '1:1' : '16:9',
    crop: null,
    frameTimeMs: format === 'mp4' ? 12000 : 0,
    updatedAt: now,
  };

  const media: MediaRecord = {
    id: mediaId,
    deviceId: destinationDeviceId,
    path: `${destinationFolder}/${fileName}`,
    folderPath: destinationFolder,
    fileName,
    type: mediaType,
    mime: format === 'mp3' ? 'audio/mpeg' : 'video/mp4',
    size: format === 'mp3' ? 5_400_000 : 48_200_000,
    hash: `sha256-${mediaId}`,
    partialHash: `partial-${mediaId}`,
    createdAt: now,
    modifiedAt: now,
    durationMs: 215_000,
    width: format === 'mp4' ? 1920 : 0,
    height: format === 'mp4' ? 1080 : 0,
    gpsLat: null,
    gpsLon: null,
    place: null,
    tags: {
      source: 'youtube',
      title: cleanTitle,
      artist: format === 'mp3' ? 'YouTube Audio' : undefined,
      uploader: 'YouTube Creator',
    },
    alias: cleanTitle,
    autoAlias: cleanTitle,
    coverId: cover.id,
    syncState: 'Local',
    isDuplicate: false,
    duplicateGroupId: null,
    isTrashed: false,
    trashedPath: null,
    updatedAt: now,
  };

  const job: DownloadJobRecord = {
    id: jobId,
    url,
    title: cleanTitle,
    format,
    quality,
    destinationDeviceId,
    destinationFolder,
    status: 'completed',
    progress: 100,
    outputMediaId: mediaId,
    error: null,
    createdAt: now,
    updatedAt: now,
  };

  return { job, media, cover };
}
