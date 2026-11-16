import type {
  AppSettings,
  AutoSyncRuleRecord,
  CoverRecord,
  CropRect,
  DeviceRecord,
  DownloadJobRecord,
  FolderCardSummary,
  IndexDeltaPayload,
  MediaRecord,
  MediaSearchFilters,
  MediaTags,
  MediaWithCover,
  PinnedFolderRecord,
  RenameLogRecord,
  SavedSearchRecord,
  SortField,
  TransferRecord,
  TransportKind,
} from '../types/domain.ts';
import type { SqlAdapter, SqlParam } from './sqlAdapter.ts';

export const DEFAULT_APP_SETTINGS: AppSettings = {
  sidebarWidth: 248,
  sidebarCollapsed: false,
  theme: 'dark',
  transportPriority: ['wired', 'wifi_lan', 'wifi_direct', 'bluetooth'],
  manualTransportOverride: 'auto',
  aliasTemplate: '{date} {place} {name}',
  scanFolders: [],
  excludedFolders: [],
  onlineGeocodingEnabled: false,
  youtubeDefaultVideoFolder: 'Videos/YouTube',
  youtubeDefaultAudioFolder: 'Music/YouTube',
};

function parseJson<T>(raw: unknown, fallback: T): T {
  if (typeof raw !== 'string' || raw.length === 0) return fallback;
  try {
    return JSON.parse(raw) as T;
  } catch {
    return fallback;
  }
}

/**
 * Converts free-form user search text into a safe FTS5 prefix query expression.
 */
export function buildFtsMatchQuery(rawQuery: string): string {
  const tokens = rawQuery
    .trim()
    .split(/[\s,;:!?'"()[\]{}<>*^~`\\/|+=-]+/)
    .map((token) => token.trim())
    .filter((token) => token.length > 0);
  if (tokens.length === 0) return '';
  return tokens.map((token) => `"${token.replace(/"/g, '')}"*`).join(' ');
}

function mapDeviceRow(row: Record<string, unknown>): DeviceRecord {
  return {
    id: String(row.id),
    name: String(row.name),
    type: row.type as DeviceRecord['type'],
    publicKey: String(row.public_key ?? ''),
    lastSeen: Number(row.last_seen ?? 0),
    transports: parseJson<TransportKind[]>(row.transports, []),
    host: row.host != null ? String(row.host) : undefined,
    port: row.port != null ? Number(row.port) : undefined,
    isSelf: Boolean(row.is_self),
    isOnline: Boolean(row.is_online),
    pairedAt: row.paired_at != null ? Number(row.paired_at) : undefined,
  };
}

function mapCoverRow(row: Record<string, unknown>): CoverRecord {
  return {
    id: String(row.id),
    mediaId: String(row.media_id),
    source: row.source as CoverRecord['source'],
    filePath: String(row.file_path),
    cloudId: row.cloud_id != null ? String(row.cloud_id) : null,
    aspect: (row.aspect as CoverRecord['aspect']) ?? '16:9',
    crop: parseJson<CropRect | null>(row.crop, null),
    frameTimeMs: Number(row.frame_time_ms ?? 0),
    updatedAt: Number(row.updated_at ?? 0),
  };
}

function mapMediaRow(row: Record<string, unknown>): MediaRecord {
  return {
    id: String(row.id),
    deviceId: String(row.device_id),
    path: String(row.path),
    folderPath: String(row.folder_path),
    fileName: String(row.file_name),
    type: row.type as MediaRecord['type'],
    mime: String(row.mime),
    size: Number(row.size ?? 0),
    hash: String(row.hash ?? ''),
    partialHash: String(row.partial_hash ?? ''),
    createdAt: Number(row.created_at ?? 0),
    modifiedAt: Number(row.modified_at ?? 0),
    durationMs: Number(row.duration_ms ?? 0),
    width: Number(row.width ?? 0),
    height: Number(row.height ?? 0),
    gpsLat: row.gps_lat != null ? Number(row.gps_lat) : null,
    gpsLon: row.gps_lon != null ? Number(row.gps_lon) : null,
    place: row.place != null ? String(row.place) : null,
    tags: parseJson<MediaTags>(row.tags, {}),
    alias: String(row.alias ?? ''),
    autoAlias: String(row.auto_alias ?? ''),
    coverId: row.cover_id != null ? String(row.cover_id) : null,
    syncState: row.sync_state as MediaRecord['syncState'],
    isDuplicate: Boolean(row.is_duplicate),
    duplicateGroupId: row.duplicate_group_id != null ? String(row.duplicate_group_id) : null,
    isTrashed: Boolean(row.is_trashed),
    trashedPath: row.trashed_path != null ? String(row.trashed_path) : null,
    updatedAt: Number(row.updated_at ?? 0),
  };
}

function mapTransferRow(row: Record<string, unknown>): TransferRecord {
  return {
    id: String(row.id),
    mediaId: String(row.media_id),
    fileName: String(row.file_name),
    fromDevice: String(row.from_device),
    toDevice: String(row.to_device),
    status: row.status as TransferRecord['status'],
    bytesDone: Number(row.bytes_done ?? 0),
    bytesTotal: Number(row.bytes_total ?? 0),
    lastChunkIndex: Number(row.last_chunk_index ?? 0),
    chunkSize: Number(row.chunk_size ?? 262144),
    transport: row.transport as TransportKind,
    speedBps: Number(row.speed_bps ?? 0),
    expectedHash: String(row.expected_hash ?? ''),
    error: row.error != null ? String(row.error) : null,
    updatedAt: Number(row.updated_at ?? 0),
  };
}

function mapPinnedFolderRow(row: Record<string, unknown>): PinnedFolderRecord {
  return {
    id: String(row.id),
    deviceId: String(row.device_id),
    path: String(row.path),
    label: String(row.label),
    icon: String(row.icon ?? 'folder'),
    position: Number(row.position ?? 0),
  };
}

function mapRenameLogRow(row: Record<string, unknown>): RenameLogRecord {
  return {
    id: String(row.id),
    mediaId: String(row.media_id),
    batchId: row.batch_id != null ? String(row.batch_id) : null,
    oldName: String(row.old_name),
    newName: String(row.new_name),
    oldPath: String(row.old_path),
    newPath: String(row.new_path),
    appliedAt: Number(row.applied_at ?? 0),
    undoneAt: row.undone_at != null ? Number(row.undone_at) : null,
  };
}

function mapDownloadJobRow(row: Record<string, unknown>): DownloadJobRecord {
  return {
    id: String(row.id),
    url: String(row.url),
    title: row.title != null ? String(row.title) : null,
    format: row.format as DownloadJobRecord['format'],
    quality: String(row.quality),
    destinationDeviceId: String(row.destination_device_id),
    destinationFolder: String(row.destination_folder),
    status: row.status as DownloadJobRecord['status'],
    progress: Number(row.progress ?? 0),
    outputMediaId: row.output_media_id != null ? String(row.output_media_id) : null,
    error: row.error != null ? String(row.error) : null,
    createdAt: Number(row.created_at ?? 0),
    updatedAt: Number(row.updated_at ?? 0),
  };
}

const SORT_COLUMNS: Record<SortField, string> = {
  name: 'LOWER(COALESCE(NULLIF(m.alias, \'\'), m.file_name))',
  date: 'm.created_at',
  size: 'm.size',
  duration: 'm.duration_ms',
  type: 'm.type',
};

/**
 * Data access layer for Media Gallery backed by SQLite + FTS5.
 */
export class MediaRepository {
  constructor(private readonly db: SqlAdapter) {}

  close(): void {
    this.db.close();
  }

  // ---------------------------------------------------------------------------
  // Devices
  // ---------------------------------------------------------------------------

  upsertDevice(device: DeviceRecord): void {
    this.db.run(
      `INSERT INTO device (id, name, type, public_key, last_seen, transports, host, port, is_self, is_online, paired_at)
       VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
       ON CONFLICT(id) DO UPDATE SET
         name = excluded.name,
         type = excluded.type,
         public_key = CASE WHEN excluded.public_key != '' THEN excluded.public_key ELSE device.public_key END,
         last_seen = excluded.last_seen,
         transports = excluded.transports,
         host = COALESCE(excluded.host, device.host),
         port = COALESCE(excluded.port, device.port),
         is_self = MAX(device.is_self, excluded.is_self),
         is_online = excluded.is_online,
         paired_at = COALESCE(excluded.paired_at, device.paired_at)`,
      [
        device.id,
        device.name,
        device.type,
        device.publicKey,
        device.lastSeen,
        JSON.stringify(device.transports),
        device.host ?? null,
        device.port ?? null,
        device.isSelf ? 1 : 0,
        device.isOnline ? 1 : 0,
        device.pairedAt ?? null,
      ],
    );
  }

  getDevice(id: string): DeviceRecord | null {
    const row = this.db.get('SELECT * FROM device WHERE id = ?', [id]);
    return row ? mapDeviceRow(row) : null;
  }

  listDevices(): DeviceRecord[] {
    return this.db
      .all('SELECT * FROM device ORDER BY is_self DESC, is_online DESC, last_seen DESC')
      .map(mapDeviceRow);
  }

  updateDeviceOnlineState(id: string, isOnline: boolean, lastSeen = Date.now()): void {
    this.db.run('UPDATE device SET is_online = ?, last_seen = ? WHERE id = ?', [
      isOnline ? 1 : 0,
      lastSeen,
      id,
    ]);
  }

  forgetDevice(id: string): void {
    const target = this.getDevice(id);
    if (!target || target.isSelf) return;
    this.db.transaction(() => {
      this.db.run('DELETE FROM media WHERE device_id = ?', [id]);
      this.db.run('DELETE FROM device WHERE id = ? AND is_self = 0', [id]);
    });
  }

  // ---------------------------------------------------------------------------
  // Media & FTS5 Search
  // ---------------------------------------------------------------------------

  upsertMedia(media: MediaRecord): void {
    this.upsertMediaBatch([media]);
  }

  upsertMediaBatch(items: MediaRecord[]): void {
    if (items.length === 0) return;
    const now = Date.now();
    this.db.transaction(() => {
      for (const item of items) {
        this.db.run(
          `INSERT INTO media (
            id, device_id, path, folder_path, file_name, type, mime, size, hash, partial_hash,
            created_at, modified_at, duration_ms, width, height, gps_lat, gps_lon, place,
            tags, alias, auto_alias, cover_id, sync_state, is_duplicate, duplicate_group_id,
            is_trashed, trashed_path, updated_at
          ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
          ON CONFLICT(id) DO UPDATE SET
            path = excluded.path,
            folder_path = excluded.folder_path,
            file_name = excluded.file_name,
            type = excluded.type,
            mime = excluded.mime,
            size = excluded.size,
            hash = CASE WHEN excluded.hash != '' THEN excluded.hash ELSE media.hash END,
            partial_hash = CASE WHEN excluded.partial_hash != '' THEN excluded.partial_hash ELSE media.partial_hash END,
            created_at = excluded.created_at,
            modified_at = excluded.modified_at,
            duration_ms = CASE WHEN excluded.duration_ms > 0 THEN excluded.duration_ms ELSE media.duration_ms END,
            width = CASE WHEN excluded.width > 0 THEN excluded.width ELSE media.width END,
            height = CASE WHEN excluded.height > 0 THEN excluded.height ELSE media.height END,
            gps_lat = COALESCE(excluded.gps_lat, media.gps_lat),
            gps_lon = COALESCE(excluded.gps_lon, media.gps_lon),
            place = COALESCE(excluded.place, media.place),
            tags = excluded.tags,
            alias = excluded.alias,
            auto_alias = excluded.auto_alias,
            cover_id = COALESCE(excluded.cover_id, media.cover_id),
            sync_state = excluded.sync_state,
            is_duplicate = excluded.is_duplicate,
            duplicate_group_id = excluded.duplicate_group_id,
            is_trashed = excluded.is_trashed,
            trashed_path = excluded.trashed_path,
            updated_at = excluded.updated_at`,
          [
            item.id,
            item.deviceId,
            item.path,
            item.folderPath,
            item.fileName,
            item.type,
            item.mime,
            item.size,
            item.hash,
            item.partialHash ?? '',
            item.createdAt,
            item.modifiedAt,
            item.durationMs,
            item.width,
            item.height,
            item.gpsLat,
            item.gpsLon,
            item.place,
            JSON.stringify(item.tags ?? {}),
            item.alias,
            item.autoAlias,
            item.coverId,
            item.syncState,
            item.isDuplicate ? 1 : 0,
            item.duplicateGroupId ?? null,
            item.isTrashed ? 1 : 0,
            item.trashedPath ?? null,
            item.updatedAt ?? now,
          ],
        );
      }
    });
  }

  getMediaById(id: string): MediaWithCover | null {
    const row = this.db.get(
      `SELECT m.*,
              d.name AS device_name, d.type AS device_type, d.is_online AS device_online,
              c.id AS c_id, c.source AS c_source, c.file_path AS c_file_path, c.cloud_id AS c_cloud_id,
              c.aspect AS c_aspect, c.crop AS c_crop, c.frame_time_ms AS c_frame_time_ms, c.updated_at AS c_updated_at
       FROM media m
       LEFT JOIN device d ON d.id = m.device_id
       LEFT JOIN cover c ON c.media_id = m.id
       WHERE m.id = ?`,
      [id],
    );
    if (!row) return null;
    return this.attachJoinColumns(row);
  }

  getMediaByPath(deviceId: string, filePath: string): MediaRecord | null {
    const row = this.db.get('SELECT * FROM media WHERE device_id = ? AND path = ?', [
      deviceId,
      filePath,
    ]);
    return row ? mapMediaRow(row) : null;
  }

  queryMedia(queryText = '', filters: MediaSearchFilters = {}): MediaWithCover[] {
    const { whereSql, params, joinFts } = this.buildWhereClause(queryText, filters);
    const sortField = filters.sortBy ?? 'date';
    const sortCol = SORT_COLUMNS[sortField] ?? SORT_COLUMNS.date;
    const sortDir = filters.sortDir === 'asc' ? 'ASC' : 'DESC';
    const limit = Math.min(Math.max(filters.limit ?? 250, 1), 10000);
    const offset = Math.max(filters.offset ?? 0, 0);

    const ftsJoin = joinFts ? 'INNER JOIN media_fts f ON f.media_id = m.id' : '';
    const sql = `
      SELECT m.*,
             d.name AS device_name, d.type AS device_type, d.is_online AS device_online,
             c.id AS c_id, c.source AS c_source, c.file_path AS c_file_path, c.cloud_id AS c_cloud_id,
             c.aspect AS c_aspect, c.crop AS c_crop, c.frame_time_ms AS c_frame_time_ms, c.updated_at AS c_updated_at
      FROM media m
      ${ftsJoin}
      LEFT JOIN device d ON d.id = m.device_id
      LEFT JOIN cover c ON c.media_id = m.id
      ${whereSql}
      ORDER BY ${sortCol} ${sortDir}, m.id ASC
      LIMIT ? OFFSET ?
    `;

    return this.db.all(sql, [...params, limit, offset]).map((row) => this.attachJoinColumns(row));
  }

  countMedia(queryText = '', filters: MediaSearchFilters = {}): number {
    const { whereSql, params, joinFts } = this.buildWhereClause(queryText, filters);
    const ftsJoin = joinFts ? 'INNER JOIN media_fts f ON f.media_id = m.id' : '';
    const row = this.db.get<{ cnt: number }>(
      `SELECT COUNT(*) AS cnt FROM media m ${ftsJoin} ${whereSql}`,
      params,
    );
    return Number(row?.cnt ?? 0);
  }

  listFolderSummaries(deviceId?: string, typeFilter?: MediaRecord['type'] | 'all'): FolderCardSummary[] {
    const clauses = ['m.is_trashed = 0'];
    const params: SqlParam[] = [];
    if (deviceId && deviceId !== 'all') {
      clauses.push('m.device_id = ?');
      params.push(deviceId);
    }
    if (typeFilter && typeFilter !== 'all') {
      clauses.push('m.type = ?');
      params.push(typeFilter);
    }
    const rows = this.db.all<{ folder_path: string; device_id: string; item_count: number }>(
      `SELECT m.folder_path, m.device_id, COUNT(*) AS item_count
       FROM media m
       WHERE ${clauses.join(' AND ')}
       GROUP BY m.device_id, m.folder_path
       ORDER BY item_count DESC, m.folder_path ASC`,
      params,
    );

    return rows.map((folder) => {
      const folderSegments = folder.folder_path.replace(/\\/g, '/').split('/').filter(Boolean);
      const name = folderSegments[folderSegments.length - 1] ?? folder.folder_path;
      const previewMedia = this.queryMedia('', {
        deviceId: folder.device_id,
        folderPath: folder.folder_path,
        type: typeFilter ?? 'all',
        limit: 4,
      });
      return {
        path: folder.folder_path,
        name,
        deviceId: folder.device_id,
        itemCount: Number(folder.item_count),
        previewMedia,
      };
    });
  }

  markMediaTrashed(id: string, trashedPath: string): void {
    this.db.run(
      'UPDATE media SET is_trashed = 1, trashed_path = ?, updated_at = ? WHERE id = ?',
      [trashedPath, Date.now(), id],
    );
  }

  restoreMediaFromTrash(id: string): void {
    this.db.run(
      'UPDATE media SET is_trashed = 0, trashed_path = NULL, updated_at = ? WHERE id = ?',
      [Date.now(), id],
    );
  }

  deleteMediaByDevicePath(deviceId: string, filePath: string): void {
    this.db.run('DELETE FROM media WHERE device_id = ? AND path = ?', [deviceId, filePath]);
  }

  updateMediaAlias(id: string, alias: string): void {
    this.db.run('UPDATE media SET alias = ?, updated_at = ? WHERE id = ?', [
      alias,
      Date.now(),
      id,
    ]);
  }

  updateMediaPathAndName(id: string, newPath: string, newFileName: string, newFolderPath?: string): void {
    const folder =
      newFolderPath ??
      newPath.replace(/\\/g, '/').split('/').slice(0, -1).join('/');
    this.db.run(
      'UPDATE media SET path = ?, file_name = ?, folder_path = ?, updated_at = ? WHERE id = ?',
      [newPath, newFileName, folder, Date.now(), id],
    );
  }

  updateMediaSyncState(id: string, syncState: MediaRecord['syncState']): void {
    this.db.run('UPDATE media SET sync_state = ?, updated_at = ? WHERE id = ?', [
      syncState,
      Date.now(),
      id,
    ]);
  }

  updateDuplicateFlags(
    updates: Array<{ id: string; isDuplicate: boolean; duplicateGroupId: string | null }>,
  ): void {
    this.db.transaction(() => {
      for (const item of updates) {
        this.db.run(
          'UPDATE media SET is_duplicate = ?, duplicate_group_id = ?, updated_at = ? WHERE id = ?',
          [item.isDuplicate ? 1 : 0, item.duplicateGroupId, Date.now(), item.id],
        );
      }
    });
  }

  // ---------------------------------------------------------------------------
  // Covers
  // ---------------------------------------------------------------------------

  upsertCover(cover: CoverRecord): void {
    this.db.transaction(() => {
      this.db.run(
        `INSERT INTO cover (id, media_id, source, file_path, cloud_id, aspect, crop, frame_time_ms, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(media_id) DO UPDATE SET
           source = excluded.source,
           file_path = excluded.file_path,
           cloud_id = COALESCE(excluded.cloud_id, cover.cloud_id),
           aspect = excluded.aspect,
           crop = excluded.crop,
           frame_time_ms = excluded.frame_time_ms,
           updated_at = excluded.updated_at`,
        [
          cover.id,
          cover.mediaId,
          cover.source,
          cover.filePath,
          cover.cloudId,
          cover.aspect,
          cover.crop ? JSON.stringify(cover.crop) : null,
          cover.frameTimeMs,
          cover.updatedAt,
        ],
      );
      this.db.run('UPDATE media SET cover_id = ?, updated_at = ? WHERE id = ?', [
        cover.id,
        cover.updatedAt,
        cover.mediaId,
      ]);
    });
  }

  getCoverByMediaId(mediaId: string): CoverRecord | null {
    const row = this.db.get('SELECT * FROM cover WHERE media_id = ?', [mediaId]);
    return row ? mapCoverRow(row) : null;
  }

  // ---------------------------------------------------------------------------
  // Transfers
  // ---------------------------------------------------------------------------

  upsertTransfer(transfer: TransferRecord): void {
    this.db.run(
      `INSERT INTO transfer (
        id, media_id, file_name, from_device, to_device, status, bytes_done, bytes_total,
        last_chunk_index, chunk_size, transport, speed_bps, expected_hash, error, updated_at
      ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
      ON CONFLICT(id) DO UPDATE SET
        status = excluded.status,
        bytes_done = excluded.bytes_done,
        bytes_total = excluded.bytes_total,
        last_chunk_index = excluded.last_chunk_index,
        chunk_size = excluded.chunk_size,
        transport = excluded.transport,
        speed_bps = excluded.speed_bps,
        expected_hash = excluded.expected_hash,
        error = excluded.error,
        updated_at = excluded.updated_at`,
      [
        transfer.id,
        transfer.mediaId,
        transfer.fileName,
        transfer.fromDevice,
        transfer.toDevice,
        transfer.status,
        transfer.bytesDone,
        transfer.bytesTotal,
        transfer.lastChunkIndex,
        transfer.chunkSize,
        transfer.transport,
        transfer.speedBps,
        transfer.expectedHash,
        transfer.error,
        transfer.updatedAt,
      ],
    );
  }

  getTransfer(id: string): TransferRecord | null {
    const row = this.db.get('SELECT * FROM transfer WHERE id = ?', [id]);
    return row ? mapTransferRow(row) : null;
  }

  listTransfers(): TransferRecord[] {
    return this.db
      .all('SELECT * FROM transfer ORDER BY updated_at DESC')
      .map(mapTransferRow);
  }

  // ---------------------------------------------------------------------------
  // Pinned Folders (Bottom Bar)
  // ---------------------------------------------------------------------------

  listPinnedFolders(): PinnedFolderRecord[] {
    return this.db
      .all('SELECT * FROM pinned_folder ORDER BY position ASC, label ASC')
      .map(mapPinnedFolderRow);
  }

  upsertPinnedFolder(folder: PinnedFolderRecord): void {
    this.db.run(
      `INSERT INTO pinned_folder (id, device_id, path, label, icon, position)
       VALUES (?, ?, ?, ?, ?, ?)
       ON CONFLICT(id) DO UPDATE SET
         device_id = excluded.device_id,
         path = excluded.path,
         label = excluded.label,
         icon = excluded.icon,
         position = excluded.position`,
      [folder.id, folder.deviceId, folder.path, folder.label, folder.icon, folder.position],
    );
  }

  removePinnedFolder(id: string): void {
    this.db.run('DELETE FROM pinned_folder WHERE id = ?', [id]);
  }

  reorderPinnedFolders(orderedIds: string[]): void {
    this.db.transaction(() => {
      orderedIds.forEach((id, index) => {
        this.db.run('UPDATE pinned_folder SET position = ? WHERE id = ?', [index, id]);
      });
    });
  }

  // ---------------------------------------------------------------------------
  // Rename Log (Undoable real renames)
  // ---------------------------------------------------------------------------

  insertRenameLog(entry: RenameLogRecord): void {
    this.db.run(
      `INSERT INTO rename_log (id, media_id, batch_id, old_name, new_name, old_path, new_path, applied_at, undone_at)
       VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)`,
      [
        entry.id,
        entry.mediaId,
        entry.batchId,
        entry.oldName,
        entry.newName,
        entry.oldPath,
        entry.newPath,
        entry.appliedAt,
        entry.undoneAt,
      ],
    );
  }

  listRenameLogs(mediaId?: string): RenameLogRecord[] {
    if (mediaId) {
      return this.db
        .all('SELECT * FROM rename_log WHERE media_id = ? ORDER BY applied_at DESC', [mediaId])
        .map(mapRenameLogRow);
    }
    return this.db
      .all('SELECT * FROM rename_log ORDER BY applied_at DESC LIMIT 200')
      .map(mapRenameLogRow);
  }

  getLatestUndoableRename(mediaId?: string): RenameLogRecord | null {
    const row = mediaId
      ? this.db.get(
          'SELECT * FROM rename_log WHERE media_id = ? AND undone_at IS NULL ORDER BY applied_at DESC LIMIT 1',
          [mediaId],
        )
      : this.db.get(
          'SELECT * FROM rename_log WHERE undone_at IS NULL ORDER BY applied_at DESC LIMIT 1',
        );
    return row ? mapRenameLogRow(row) : null;
  }

  getRenameBatch(batchId: string): RenameLogRecord[] {
    return this.db
      .all(
        'SELECT * FROM rename_log WHERE batch_id = ? AND undone_at IS NULL ORDER BY applied_at DESC',
        [batchId],
      )
      .map(mapRenameLogRow);
  }

  markRenameLogUndone(id: string, undoneAt = Date.now()): void {
    this.db.run('UPDATE rename_log SET undone_at = ? WHERE id = ?', [undoneAt, id]);
  }

  // ---------------------------------------------------------------------------
  // Download Jobs (YouTube)
  // ---------------------------------------------------------------------------

  upsertDownloadJob(job: DownloadJobRecord): void {
    this.db.run(
      `INSERT INTO download_job (
        id, url, title, format, quality, destination_device_id, destination_folder,
        status, progress, output_media_id, error, created_at, updated_at
      ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
      ON CONFLICT(id) DO UPDATE SET
        title = COALESCE(excluded.title, download_job.title),
        status = excluded.status,
        progress = excluded.progress,
        output_media_id = COALESCE(excluded.output_media_id, download_job.output_media_id),
        error = excluded.error,
        updated_at = excluded.updated_at`,
      [
        job.id,
        job.url,
        job.title,
        job.format,
        job.quality,
        job.destinationDeviceId,
        job.destinationFolder,
        job.status,
        job.progress,
        job.outputMediaId,
        job.error,
        job.createdAt,
        job.updatedAt,
      ],
    );
  }

  listDownloadJobs(): DownloadJobRecord[] {
    return this.db
      .all('SELECT * FROM download_job ORDER BY created_at DESC')
      .map(mapDownloadJobRow);
  }

  // ---------------------------------------------------------------------------
  // Settings, Saved Searches & Auto-Sync Rules
  // ---------------------------------------------------------------------------

  getSetting(key: string): string | null {
    const row = this.db.get<{ value: string }>('SELECT value FROM settings WHERE key = ?', [key]);
    return row ? row.value : null;
  }

  setSetting(key: string, value: string): void {
    this.db.run(
      `INSERT INTO settings (key, value) VALUES (?, ?)
       ON CONFLICT(key) DO UPDATE SET value = excluded.value`,
      [key, value],
    );
  }

  getAppSettings(): AppSettings {
    const raw = this.getSetting('app_settings');
    return { ...DEFAULT_APP_SETTINGS, ...parseJson<Partial<AppSettings>>(raw, {}) };
  }

  saveAppSettings(patch: Partial<AppSettings>): AppSettings {
    const merged = { ...this.getAppSettings(), ...patch };
    this.setSetting('app_settings', JSON.stringify(merged));
    return merged;
  }

  upsertSavedSearch(saved: SavedSearchRecord): void {
    this.db.run(
      `INSERT INTO saved_search (id, name, query, filters, created_at)
       VALUES (?, ?, ?, ?, ?)
       ON CONFLICT(id) DO UPDATE SET
         name = excluded.name,
         query = excluded.query,
         filters = excluded.filters`,
      [saved.id, saved.name, saved.query, JSON.stringify(saved.filters), saved.createdAt],
    );
  }

  listSavedSearches(): SavedSearchRecord[] {
    return this.db.all('SELECT * FROM saved_search ORDER BY created_at DESC').map((row) => ({
      id: String(row.id),
      name: String(row.name),
      query: String(row.query),
      filters: parseJson<MediaSearchFilters>(row.filters, {}),
      createdAt: Number(row.created_at),
    }));
  }

  deleteSavedSearch(id: string): void {
    this.db.run('DELETE FROM saved_search WHERE id = ?', [id]);
  }

  upsertAutoSyncRule(rule: AutoSyncRuleRecord): void {
    this.db.run(
      `INSERT INTO auto_sync_rule (id, name, source_device_id, target_device_id, folder_pattern, media_types, enabled, created_at)
       VALUES (?, ?, ?, ?, ?, ?, ?, ?)
       ON CONFLICT(id) DO UPDATE SET
         name = excluded.name,
         source_device_id = excluded.source_device_id,
         target_device_id = excluded.target_device_id,
         folder_pattern = excluded.folder_pattern,
         media_types = excluded.media_types,
         enabled = excluded.enabled`,
      [
        rule.id,
        rule.name,
        rule.sourceDeviceId,
        rule.targetDeviceId,
        rule.folderPattern,
        JSON.stringify(rule.mediaTypes),
        rule.enabled ? 1 : 0,
        rule.createdAt,
      ],
    );
  }

  listAutoSyncRules(): AutoSyncRuleRecord[] {
    return this.db.all('SELECT * FROM auto_sync_rule ORDER BY created_at DESC').map((row) => ({
      id: String(row.id),
      name: String(row.name),
      sourceDeviceId: String(row.source_device_id),
      targetDeviceId: String(row.target_device_id),
      folderPattern: String(row.folder_pattern),
      mediaTypes: parseJson<AutoSyncRuleRecord['mediaTypes']>(row.media_types, ['image']),
      enabled: Boolean(row.enabled),
      createdAt: Number(row.created_at),
    }));
  }

  deleteAutoSyncRule(id: string): void {
    this.db.run('DELETE FROM auto_sync_rule WHERE id = ?', [id]);
  }

  // ---------------------------------------------------------------------------
  // Serverless Index Delta Sync (PRD Section 10.4)
  // ---------------------------------------------------------------------------

  exportDeltaSince(deviceId: string, sinceTimestamp = 0): IndexDeltaPayload {
    const device = this.getDevice(deviceId);
    if (!device) {
      throw new Error(`Cannot export delta for unknown device "${deviceId}"`);
    }
    const upsertedMedia = this.db
      .all(
        'SELECT * FROM media WHERE device_id = ? AND is_trashed = 0 AND updated_at > ?',
        [deviceId, sinceTimestamp],
      )
      .map(mapMediaRow);
    const deletedMediaIds = this.db
      .all<{ id: string }>(
        'SELECT id FROM media WHERE device_id = ? AND is_trashed = 1 AND updated_at > ?',
        [deviceId, sinceTimestamp],
      )
      .map((row) => String(row.id));
    const upsertedCovers = this.db
      .all(
        `SELECT c.* FROM cover c
         INNER JOIN media m ON m.id = c.media_id
         WHERE m.device_id = ? AND c.updated_at > ?`,
        [deviceId, sinceTimestamp],
      )
      .map(mapCoverRow);

    return {
      device,
      generatedAt: Date.now(),
      sinceTimestamp,
      upsertedMedia,
      upsertedCovers,
      deletedMediaIds,
      thumbnailBlobs: {},
    };
  }

  /**
   * Applies an incoming index delta from a peer device.
   * Per PRD 10.1, each device is authoritative ONLY for its own records.
   */
  applyRemoteDelta(delta: IndexDeltaPayload, selfDeviceId: string): { importedCount: number } {
    if (delta.device.id === selfDeviceId) {
      return { importedCount: 0 };
    }
    let importedCount = 0;
    this.db.transaction(() => {
      this.upsertDevice({ ...delta.device, isSelf: false });

      const authoritativeItems = delta.upsertedMedia
        .filter((item) => item.deviceId === delta.device.id)
        .map((item) => {
          const existing = this.getMediaById(item.id);
          const preserveSynced = existing?.syncState === 'Synced' ? 'Synced' : 'Remote';
          return {
            ...item,
            syncState: preserveSynced as MediaRecord['syncState'],
          };
        });

      if (authoritativeItems.length > 0) {
        this.upsertMediaBatch(authoritativeItems);
        importedCount = authoritativeItems.length;
      }

      const validMediaIds = new Set(authoritativeItems.map((m) => m.id));
      for (const cover of delta.upsertedCovers) {
        if (validMediaIds.has(cover.mediaId) || this.getMediaById(cover.mediaId)?.deviceId === delta.device.id) {
          this.upsertCover(cover);
        }
      }

      for (const deletedId of delta.deletedMediaIds) {
        this.db.run('DELETE FROM media WHERE id = ? AND device_id = ?', [
          deletedId,
          delta.device.id,
        ]);
      }
    });
    return { importedCount };
  }

  // ---------------------------------------------------------------------------
  // Private query helpers
  // ---------------------------------------------------------------------------

  private buildWhereClause(
    queryText: string,
    filters: MediaSearchFilters,
  ): { whereSql: string; params: SqlParam[]; joinFts: boolean } {
    const clauses: string[] = [];
    const params: SqlParam[] = [];
    let joinFts = false;

    const ftsMatch = buildFtsMatchQuery(queryText);
    if (ftsMatch.length > 0) {
      joinFts = true;
      clauses.push('media_fts MATCH ?');
      params.push(ftsMatch);
    }

    if (!filters.includeTrashed) {
      clauses.push('m.is_trashed = 0');
    }
    if (filters.type && filters.type !== 'all') {
      clauses.push('m.type = ?');
      params.push(filters.type);
    }
    if (filters.deviceId && filters.deviceId !== 'all') {
      clauses.push('m.device_id = ?');
      params.push(filters.deviceId);
    }
    if (filters.syncState && filters.syncState !== 'all') {
      clauses.push('m.sync_state = ?');
      params.push(filters.syncState);
    }
    if (filters.folderPath) {
      clauses.push('m.folder_path = ?');
      params.push(filters.folderPath);
    }
    if (filters.dateFrom != null) {
      clauses.push('m.created_at >= ?');
      params.push(filters.dateFrom);
    }
    if (filters.dateTo != null) {
      clauses.push('m.created_at <= ?');
      params.push(filters.dateTo);
    }
    if (filters.minSize != null) {
      clauses.push('m.size >= ?');
      params.push(filters.minSize);
    }
    if (filters.maxSize != null) {
      clauses.push('m.size <= ?');
      params.push(filters.maxSize);
    }
    if (filters.minDurationMs != null) {
      clauses.push('m.duration_ms >= ?');
      params.push(filters.minDurationMs);
    }
    if (filters.maxDurationMs != null) {
      clauses.push('m.duration_ms <= ?');
      params.push(filters.maxDurationMs);
    }
    if (filters.minWidth != null) {
      clauses.push('m.width >= ?');
      params.push(filters.minWidth);
    }
    if (filters.timeOfDay && filters.timeOfDay !== 'all') {
      const hourExpr = "CAST(strftime('%H', m.created_at / 1000, 'unixepoch', 'localtime') AS INTEGER)";
      if (filters.timeOfDay === 'morning') {
        clauses.push(`${hourExpr} BETWEEN 5 AND 11`);
      } else if (filters.timeOfDay === 'afternoon') {
        clauses.push(`${hourExpr} BETWEEN 12 AND 16`);
      } else if (filters.timeOfDay === 'evening') {
        clauses.push(`${hourExpr} BETWEEN 17 AND 20`);
      } else if (filters.timeOfDay === 'night') {
        clauses.push(`(${hourExpr} >= 21 OR ${hourExpr} <= 4)`);
      }
    }

    return {
      whereSql: clauses.length > 0 ? `WHERE ${clauses.join(' AND ')}` : '',
      params,
      joinFts,
    };
  }

  private attachJoinColumns(row: Record<string, unknown>): MediaWithCover {
    const base = mapMediaRow(row);
    const cover: CoverRecord | null =
      row.c_id != null
        ? {
            id: String(row.c_id),
            mediaId: base.id,
            source: row.c_source as CoverRecord['source'],
            filePath: String(row.c_file_path),
            cloudId: row.c_cloud_id != null ? String(row.c_cloud_id) : null,
            aspect: (row.c_aspect as CoverRecord['aspect']) ?? '16:9',
            crop: parseJson<CropRect | null>(row.c_crop, null),
            frameTimeMs: Number(row.c_frame_time_ms ?? 0),
            updatedAt: Number(row.c_updated_at ?? 0),
          }
        : null;
    return {
      ...base,
      cover,
      deviceName: row.device_name != null ? String(row.device_name) : undefined,
      deviceType: row.device_type != null ? (row.device_type as DeviceRecord['type']) : undefined,
      deviceOnline: row.device_online != null ? Boolean(row.device_online) : undefined,
    };
  }
}
