/**
 * Core domain types matching PRD Section 5, Section 9, Section 10, and Section 11.
 */

export type MediaType = 'video' | 'image' | 'audio';

export type SyncState = 'Local' | 'Remote' | 'Downloading' | 'Synced' | 'Error';

export type DeviceType = 'phone' | 'pc';

export type TransportKind = 'wired' | 'wifi_lan' | 'wifi_direct' | 'bluetooth';

export type CoverSource = 'generated' | 'custom' | 'embedded';

export type AspectPreset = '4:3' | '9:16' | '16:9' | '1:1' | 'custom';

export type TransferStatus =
  | 'queued'
  | 'transferring'
  | 'paused'
  | 'verifying'
  | 'completed'
  | 'cancelled'
  | 'error';

export type DownloadFormat = 'mp4' | 'mp3';

export type DownloadStatus = 'queued' | 'downloading' | 'processing' | 'completed' | 'error';

export interface CropRect {
  x: number;
  y: number;
  width: number;
  height: number;
  aspectRatio?: number;
}

export interface MediaTags {
  title?: string;
  artist?: string;
  album?: string;
  uploader?: string;
  source?: string;
  youtubeId?: string;
  playlistName?: string;
  year?: number;
  [key: string]: string | number | boolean | undefined;
}

export interface DeviceRecord {
  id: string;
  name: string;
  type: DeviceType;
  publicKey: string;
  lastSeen: number;
  transports: TransportKind[];
  host?: string;
  port?: number;
  isSelf?: boolean;
  isOnline?: boolean;
  pairedAt?: number;
}

export interface MediaRecord {
  id: string;
  deviceId: string;
  path: string;
  folderPath: string;
  fileName: string;
  type: MediaType;
  mime: string;
  size: number;
  hash: string;
  partialHash?: string;
  createdAt: number;
  modifiedAt: number;
  durationMs: number;
  width: number;
  height: number;
  gpsLat: number | null;
  gpsLon: number | null;
  place: string | null;
  tags: MediaTags;
  alias: string;
  autoAlias: string;
  coverId: string | null;
  syncState: SyncState;
  isDuplicate?: boolean;
  duplicateGroupId?: string | null;
  isTrashed?: boolean;
  trashedPath?: string | null;
  updatedAt?: number;
}

export interface CoverRecord {
  id: string;
  mediaId: string;
  source: CoverSource;
  filePath: string;
  cloudId: string | null;
  aspect: AspectPreset;
  crop: CropRect | null;
  frameTimeMs: number;
  updatedAt: number;
}

export interface TransferRecord {
  id: string;
  mediaId: string;
  fileName: string;
  fromDevice: string;
  toDevice: string;
  status: TransferStatus;
  bytesDone: number;
  bytesTotal: number;
  lastChunkIndex: number;
  chunkSize: number;
  transport: TransportKind;
  speedBps: number;
  expectedHash: string;
  error: string | null;
  updatedAt: number;
}

export interface PinnedFolderRecord {
  id: string;
  deviceId: string;
  path: string;
  label: string;
  icon: string;
  position: number;
}

export interface RenameLogRecord {
  id: string;
  mediaId: string;
  batchId: string | null;
  oldName: string;
  newName: string;
  oldPath: string;
  newPath: string;
  appliedAt: number;
  undoneAt: number | null;
}

export interface DownloadJobRecord {
  id: string;
  url: string;
  title: string | null;
  format: DownloadFormat;
  quality: string;
  destinationDeviceId: string;
  destinationFolder: string;
  status: DownloadStatus;
  progress: number;
  outputMediaId: string | null;
  error: string | null;
  createdAt: number;
  updatedAt: number;
}

export interface SavedSearchRecord {
  id: string;
  name: string;
  query: string;
  filters: MediaSearchFilters;
  createdAt: number;
}

export interface AutoSyncRuleRecord {
  id: string;
  name: string;
  sourceDeviceId: string;
  targetDeviceId: string;
  folderPattern: string;
  mediaTypes: MediaType[];
  enabled: boolean;
  createdAt: number;
}

export type TimeOfDayFilter = 'all' | 'morning' | 'afternoon' | 'evening' | 'night';

export type SortField = 'name' | 'date' | 'size' | 'duration' | 'type';

export type SortDirection = 'asc' | 'desc';

export type GroupByOption = 'none' | 'date' | 'folder' | 'device';

export type GridDensity = 'small' | 'medium' | 'large' | 'list';

export interface MediaSearchFilters {
  type?: MediaType | 'all';
  deviceId?: string | 'all';
  syncState?: SyncState | 'all';
  folderPath?: string | null;
  dateFrom?: number | null;
  dateTo?: number | null;
  timeOfDay?: TimeOfDayFilter;
  minSize?: number | null;
  maxSize?: number | null;
  minDurationMs?: number | null;
  maxDurationMs?: number | null;
  minWidth?: number | null;
  sortBy?: SortField;
  sortDir?: SortDirection;
  limit?: number;
  offset?: number;
  includeTrashed?: boolean;
}

export interface MediaWithCover extends MediaRecord {
  cover?: CoverRecord | null;
  deviceName?: string;
  deviceType?: DeviceType;
  deviceOnline?: boolean;
}

export interface FolderCardSummary {
  path: string;
  name: string;
  deviceId: string;
  itemCount: number;
  previewMedia: MediaWithCover[];
}

export interface IndexDeltaPayload {
  device: DeviceRecord;
  generatedAt: number;
  sinceTimestamp: number;
  upsertedMedia: MediaRecord[];
  upsertedCovers: CoverRecord[];
  deletedMediaIds: string[];
  thumbnailBlobs: Record<string, string>;
}

export interface AppSettings {
  sidebarWidth: number;
  sidebarCollapsed: boolean;
  theme: 'dark' | 'light';
  transportPriority: TransportKind[];
  manualTransportOverride: TransportKind | 'auto';
  aliasTemplate: string;
  scanFolders: string[];
  excludedFolders: string[];
  onlineGeocodingEnabled: boolean;
  youtubeDefaultVideoFolder: string;
  youtubeDefaultAudioFolder: string;
}

export interface ScanProgress {
  scannedFiles: number;
  addedFiles: number;
  updatedFiles: number;
  skippedFiles: number;
  currentFolder: string;
  inProgress: boolean;
}
