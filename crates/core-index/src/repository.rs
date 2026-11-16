use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, types::Value, Connection, Row};
use thiserror::Error;

use crate::migrations::run_migrations;
use crate::models::{
    AppSettings, AspectPreset, AutoSyncRuleRecord, CoverRecord, CoverSource, CropRect,
    DeviceRecord, DeviceType, DownloadFormat, DownloadJobRecord, DownloadStatus, DuplicateGroup,
    FolderSummary, IndexDelta, MediaFilter, MediaPage, MediaRecord, MediaTags, MediaType,
    MediaWithCover, PinnedFolderRecord, RenameLogRecord, SavedSearchRecord, SortDir, SortField,
    SortSpec, SyncState, TimeOfDayFilter, TransferRecord, TransferStatus, TransportKind,
};

#[derive(Debug, Error)]
pub enum IndexError {
    #[error("SQLite error: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("serialization error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("invalid argument: {0}")]
    Invalid(String),
    #[error("internal lock poisoned")]
    LockPoisoned,
}

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub fn normalize_db_path(raw: &str) -> String {
    raw.replace('\\', "/")
}

fn exec_media_upsert_stmt(
    stmt: &mut rusqlite::Statement<'_>,
    effective_id: &str,
    item: &MediaRecord,
    norm_path: &str,
    norm_folder: &str,
    tags_json: &str,
) -> Result<usize, rusqlite::Error> {
    stmt.execute(params![
        effective_id,
        item.device_id,
        norm_path,
        norm_folder,
        item.file_name,
        item.media_type.as_str(),
        item.mime,
        item.size as i64,
        item.blake3,
        item.partial_hash,
        item.created_at,
        item.modified_at,
        item.duration_ms as i64,
        i64::from(item.width),
        i64::from(item.height),
        item.gps_lat,
        item.gps_lon,
        item.place,
        tags_json,
        item.alias,
        item.auto_alias,
        item.cover_id,
        item.sync_state.as_str(),
        i64::from(item.is_duplicate),
        item.duplicate_group_id,
        i64::from(item.is_trashed),
        item.trashed_path,
        item.updated_at,
    ])
}

/// Converts user search text into a safe FTS5 prefix expression (e.g. `"2026"* "beach"*`).
pub fn build_fts_match_query(raw_query: &str) -> String {
    let tokens: Vec<String> = raw_query
        .split(|c: char| !c.is_alphanumeric())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| format!("\"{}\"*", s.replace('"', "")))
        .collect();
    tokens.join(" ")
}

fn map_device_row(row: &Row<'_>) -> Result<DeviceRecord, rusqlite::Error> {
    let transports_json: String = row.get("transports")?;
    let transports: Vec<TransportKind> = serde_json::from_str(&transports_json).unwrap_or_default();
    let dtype_raw: String = row.get("type")?;
    Ok(DeviceRecord {
        id: row.get("id")?,
        name: row.get("name")?,
        device_type: DeviceType::parse(&dtype_raw),
        public_key: row.get("public_key")?,
        last_seen: row.get("last_seen")?,
        transports,
        host: row.get("host")?,
        port: row.get::<_, Option<i64>>("port")?.map(|p| p as u16),
        is_self: row.get::<_, i64>("is_self")? != 0,
        is_online: row.get::<_, i64>("is_online")? != 0,
        paired_at: row.get("paired_at")?,
    })
}

fn map_media_row(row: &Row<'_>) -> Result<MediaRecord, rusqlite::Error> {
    let type_raw: String = row.get("type")?;
    let sync_raw: String = row.get("sync_state")?;
    let tags_json: String = row.get("tags")?;
    let tags: MediaTags = serde_json::from_str(&tags_json).unwrap_or_default();
    Ok(MediaRecord {
        id: row.get("id")?,
        device_id: row.get("device_id")?,
        path: row.get("path")?,
        folder_path: row.get("folder_path")?,
        file_name: row.get("file_name")?,
        media_type: MediaType::parse(&type_raw),
        mime: row.get("mime")?,
        size: row.get::<_, i64>("size")? as u64,
        blake3: row.get("blake3")?,
        partial_hash: row.get("partial_hash")?,
        created_at: row.get("created_at")?,
        modified_at: row.get("modified_at")?,
        duration_ms: row.get::<_, i64>("duration_ms")? as u64,
        width: row.get::<_, i64>("width")? as u32,
        height: row.get::<_, i64>("height")? as u32,
        gps_lat: row.get("gps_lat")?,
        gps_lon: row.get("gps_lon")?,
        place: row.get("place")?,
        tags,
        alias: row.get("alias")?,
        auto_alias: row.get("auto_alias")?,
        cover_id: row.get("cover_id")?,
        sync_state: SyncState::parse(&sync_raw),
        is_duplicate: row.get::<_, i64>("is_duplicate")? != 0,
        duplicate_group_id: row.get("duplicate_group_id")?,
        is_trashed: row.get::<_, i64>("is_trashed")? != 0,
        trashed_path: row.get("trashed_path")?,
        updated_at: row.get("updated_at")?,
    })
}

fn map_cover_row(row: &Row<'_>) -> Result<CoverRecord, rusqlite::Error> {
    let source_raw: String = row.get("source")?;
    let aspect_raw: String = row.get("aspect")?;
    let crop_raw: Option<String> = row.get("crop")?;
    let crop = crop_raw.and_then(|s| serde_json::from_str::<CropRect>(&s).ok());
    Ok(CoverRecord {
        id: row.get("id")?,
        media_id: row.get("media_id")?,
        source: CoverSource::parse(&source_raw),
        file_path: row.get("file_path")?,
        cloud_id: row.get("cloud_id")?,
        aspect: AspectPreset::parse(&aspect_raw),
        crop,
        frame_time_ms: row.get::<_, i64>("frame_time_ms")? as u64,
        updated_at: row.get("updated_at")?,
    })
}

fn map_media_with_cover_row(row: &Row<'_>) -> Result<MediaWithCover, rusqlite::Error> {
    let media = map_media_row(row)?;
    let cover_id: Option<String> = row.get("c_id")?;
    let cover = if let Some(cid) = cover_id {
        let src: String = row.get("c_source")?;
        let asp: String = row.get("c_aspect")?;
        let crop_raw: Option<String> = row.get("c_crop")?;
        Some(CoverRecord {
            id: cid,
            media_id: media.id.clone(),
            source: CoverSource::parse(&src),
            file_path: row.get("c_file_path")?,
            cloud_id: row.get("c_cloud_id")?,
            aspect: AspectPreset::parse(&asp),
            crop: crop_raw.and_then(|s| serde_json::from_str(&s).ok()),
            frame_time_ms: row.get::<_, i64>("c_frame_time_ms")? as u64,
            updated_at: row.get("c_updated_at")?,
        })
    } else {
        None
    };

    Ok(MediaWithCover {
        media,
        cover,
        device_name: row.get("device_name")?,
        device_online: row.get::<_, Option<i64>>("device_online")?.unwrap_or(0) != 0,
    })
}

/// Thread-safe Rust SQLite + FTS5 media index.
#[derive(Clone)]
pub struct MediaIndex {
    conn: Arc<Mutex<Connection>>,
}

impl MediaIndex {
    pub fn open_in_memory() -> Result<Self, IndexError> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        run_migrations(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn open(db_path: &Path) -> Result<Self, IndexError> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(db_path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        run_migrations(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, IndexError> {
        self.conn.lock().map_err(|_| IndexError::LockPoisoned)
    }

    // -------------------------------------------------------------------------
    // Devices
    // -------------------------------------------------------------------------

    pub fn upsert_device(&self, device: &DeviceRecord) -> Result<(), IndexError> {
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO device (id, name, type, public_key, last_seen, transports, host, port, is_self, is_online, paired_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
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
               paired_at = COALESCE(excluded.paired_at, device.paired_at)",
            params![
                device.id,
                device.name,
                device.device_type.as_str(),
                device.public_key,
                device.last_seen,
                serde_json::to_string(&device.transports)?,
                device.host,
                device.port.map(i64::from),
                i64::from(device.is_self),
                i64::from(device.is_online),
                device.paired_at,
            ],
        )?;
        Ok(())
    }

    pub fn get_device(&self, id: &str) -> Result<Option<DeviceRecord>, IndexError> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare("SELECT * FROM device WHERE id = ?1")?;
        let mut rows = stmt.query([id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(map_device_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn list_devices(&self) -> Result<Vec<DeviceRecord>, IndexError> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT * FROM device ORDER BY is_self DESC, is_online DESC, last_seen DESC",
        )?;
        let rows = stmt.query_map([], map_device_row)?;
        let mut out = Vec::new();
        for item in rows {
            out.push(item?);
        }
        Ok(out)
    }

    pub fn update_device_online(&self, id: &str, is_online: bool) -> Result<(), IndexError> {
        let conn = self.lock()?;
        conn.execute(
            "UPDATE device SET is_online = ?1, last_seen = ?2 WHERE id = ?3",
            params![i64::from(is_online), now_ms(), id],
        )?;
        Ok(())
    }

    pub fn forget_device(&self, id: &str) -> Result<(), IndexError> {
        let Some(dev) = self.get_device(id)? else {
            return Ok(());
        };
        if dev.is_self {
            return Ok(());
        }
        let mut conn = self.lock()?;
        let tx = conn.transaction()?;
        tx.execute("DELETE FROM media WHERE device_id = ?1", [id])?;
        tx.execute("DELETE FROM device WHERE id = ?1 AND is_self = 0", [id])?;
        tx.commit()?;
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Media & Cursor-Paged FTS5 Search
    // -------------------------------------------------------------------------

    pub fn upsert_media(&self, item: &MediaRecord) -> Result<(), IndexError> {
        self.upsert_media_batch(std::slice::from_ref(item))
    }

    pub fn upsert_media_batch(&self, items: &[MediaRecord]) -> Result<(), IndexError> {
        if items.is_empty() {
            return Ok(());
        }
        let mut conn = self.lock()?;
        let tx = conn.transaction()?;
        {
            const UPSERT_SQL: &str = "INSERT INTO media (
                   id, device_id, path, folder_path, file_name, type, mime, size, blake3, partial_hash,
                   created_at, modified_at, duration_ms, width, height, gps_lat, gps_lon, place,
                   tags, alias, auto_alias, cover_id, sync_state, is_duplicate, duplicate_group_id,
                   is_trashed, trashed_path, updated_at
                 ) VALUES (
                   ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
                   ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18,
                   ?19, ?20, ?21, ?22, ?23, ?24, ?25,
                   ?26, ?27, ?28
                 )
                 ON CONFLICT(device_id, path) DO UPDATE SET
                   folder_path = excluded.folder_path,
                   file_name = excluded.file_name,
                   type = excluded.type,
                   mime = excluded.mime,
                   size = excluded.size,
                   blake3 = CASE WHEN excluded.blake3 != '' THEN excluded.blake3 ELSE media.blake3 END,
                   partial_hash = CASE WHEN excluded.partial_hash != '' THEN excluded.partial_hash ELSE media.partial_hash END,
                   created_at = excluded.created_at,
                   modified_at = excluded.modified_at,
                   duration_ms = CASE WHEN excluded.duration_ms > 0 THEN excluded.duration_ms ELSE media.duration_ms END,
                   width = CASE WHEN excluded.width > 0 THEN excluded.width ELSE media.width END,
                   height = CASE WHEN excluded.height > 0 THEN excluded.height ELSE media.height END,
                   gps_lat = COALESCE(excluded.gps_lat, media.gps_lat),
                   gps_lon = COALESCE(excluded.gps_lon, media.gps_lon),
                   place = COALESCE(excluded.place, media.place),
                   tags = CASE WHEN excluded.tags != '{}' THEN excluded.tags ELSE media.tags END,
                   alias = CASE
                     WHEN excluded.alias != '' AND excluded.alias != excluded.auto_alias THEN excluded.alias
                     WHEN media.alias != '' AND media.alias != media.auto_alias THEN media.alias
                     WHEN excluded.alias != '' THEN excluded.alias
                     ELSE media.alias
                   END,
                   auto_alias = CASE WHEN excluded.auto_alias != '' THEN excluded.auto_alias ELSE media.auto_alias END,
                   cover_id = COALESCE(excluded.cover_id, media.cover_id),
                   sync_state = excluded.sync_state,
                   is_duplicate = excluded.is_duplicate,
                   duplicate_group_id = excluded.duplicate_group_id,
                   is_trashed = excluded.is_trashed,
                   trashed_path = excluded.trashed_path,
                   updated_at = excluded.updated_at";

            const UPDATE_BY_ID_SQL: &str = "UPDATE media SET
                   device_id = ?2,
                   path = ?3,
                   folder_path = ?4,
                   file_name = ?5,
                   type = ?6,
                   mime = ?7,
                   size = ?8,
                   blake3 = CASE WHEN ?9 != '' THEN ?9 ELSE blake3 END,
                   partial_hash = CASE WHEN ?10 != '' THEN ?10 ELSE partial_hash END,
                   created_at = ?11,
                   modified_at = ?12,
                   duration_ms = CASE WHEN ?13 > 0 THEN ?13 ELSE duration_ms END,
                   width = CASE WHEN ?14 > 0 THEN ?14 ELSE width END,
                   height = CASE WHEN ?15 > 0 THEN ?15 ELSE height END,
                   gps_lat = COALESCE(?16, gps_lat),
                   gps_lon = COALESCE(?17, gps_lon),
                   place = COALESCE(?18, place),
                   tags = CASE WHEN ?19 != '{}' THEN ?19 ELSE tags END,
                   alias = CASE
                     WHEN ?20 != '' AND ?20 != ?21 THEN ?20
                     WHEN alias != '' AND alias != auto_alias THEN alias
                     WHEN ?20 != '' THEN ?20
                     ELSE alias
                   END,
                   auto_alias = CASE WHEN ?21 != '' THEN ?21 ELSE auto_alias END,
                   cover_id = COALESCE(?22, cover_id),
                   sync_state = ?23,
                   is_duplicate = ?24,
                   duplicate_group_id = ?25,
                   is_trashed = ?26,
                   trashed_path = ?27,
                   updated_at = ?28
                 WHERE id = ?1";

            let mut stmt = tx.prepare_cached(UPSERT_SQL)?;
            let mut pk_check = tx.prepare_cached("SELECT device_id FROM media WHERE id = ?1")?;
            let mut path_check =
                tx.prepare_cached("SELECT id FROM media WHERE device_id = ?1 AND path = ?2")?;
            let mut update_by_id = tx.prepare_cached(UPDATE_BY_ID_SQL)?;

            for item in items {
                let norm_path = normalize_db_path(&item.path);
                let norm_folder = normalize_db_path(&item.folder_path);
                let tags_json = serde_json::to_string(&item.tags)?;

                let exec_res = exec_media_upsert_stmt(
                    &mut stmt,
                    &item.id,
                    item,
                    &norm_path,
                    &norm_folder,
                    &tags_json,
                );

                if let Err(err) = exec_res {
                    let is_constraint = matches!(
                        err,
                        rusqlite::Error::SqliteFailure(ref f, _)
                            if f.code == rusqlite::ErrorCode::ConstraintViolation
                    );
                    if !is_constraint {
                        return Err(err.into());
                    }

                    // Primary key (`id`) collided with a different (device_id, path) row.
                    let existing_dev: Option<String> =
                        pk_check.query_row([&item.id], |r| r.get(0)).ok();
                    let existing_path_id: Option<String> = path_check
                        .query_row(params![item.device_id, norm_path], |r| r.get(0))
                        .ok();

                    match (existing_dev.as_deref(), existing_path_id) {
                        (_, Some(target_id)) => {
                            exec_media_upsert_stmt(
                                &mut stmt,
                                &target_id,
                                item,
                                &norm_path,
                                &norm_folder,
                                &tags_json,
                            )?;
                        }
                        (Some(dev), None) if dev == item.device_id => {
                            // Same item on same device moved/renamed to a new path.
                            exec_media_upsert_stmt(
                                &mut update_by_id,
                                &item.id,
                                item,
                                &norm_path,
                                &norm_folder,
                                &tags_json,
                            )?;
                        }
                        (Some(_), None) | (None, None) => {
                            // Same `id` reused across another device_id: allocate deterministic per-(device, path) id.
                            let derived_id =
                                crate::scanner::stable_media_id(&item.device_id, &norm_path);
                            exec_media_upsert_stmt(
                                &mut stmt,
                                &derived_id,
                                item,
                                &norm_path,
                                &norm_folder,
                                &tags_json,
                            )?;
                        }
                    }
                }
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn get_media_by_id(&self, id: &str) -> Result<Option<MediaWithCover>, IndexError> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT m.*,
                    d.name AS device_name, d.is_online AS device_online,
                    c.id AS c_id, c.source AS c_source, c.file_path AS c_file_path, c.cloud_id AS c_cloud_id,
                    c.aspect AS c_aspect, c.crop AS c_crop, c.frame_time_ms AS c_frame_time_ms, c.updated_at AS c_updated_at
             FROM media m
             LEFT JOIN device d ON d.id = m.device_id
             LEFT JOIN cover c ON c.media_id = m.id
             WHERE m.id = ?1",
        )?;
        let mut rows = stmt.query([id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(map_media_with_cover_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn get_media_by_path(
        &self,
        device_id: &str,
        file_path: &str,
    ) -> Result<Option<MediaRecord>, IndexError> {
        let norm_path = normalize_db_path(file_path);
        let conn = self.lock()?;
        let mut stmt = conn.prepare("SELECT * FROM media WHERE device_id = ?1 AND path = ?2")?;
        let mut rows = stmt.query(params![device_id, norm_path])?;
        if let Some(row) = rows.next()? {
            Ok(Some(map_media_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn list_media(
        &self,
        filter: &MediaFilter,
        sort: SortSpec,
        cursor: Option<&str>,
        limit: u32,
    ) -> Result<MediaPage, IndexError> {
        let offset: u64 = cursor.and_then(|c| c.parse::<u64>().ok()).unwrap_or(0);
        let safe_limit = u64::from(limit.clamp(1, 10_000));

        let (where_sql, bind_params, join_fts) = build_where_clause(filter);
        let fts_join = if join_fts {
            "INNER JOIN media_fts f ON f.media_id = m.id"
        } else {
            ""
        };

        let dir = match sort.dir {
            SortDir::Asc => "ASC",
            SortDir::Desc => "DESC",
        };
        let order_expr = match sort.field {
            SortField::Name => format!(
                "LOWER(COALESCE(NULLIF(m.alias, ''), m.file_name)) {dir}, LOWER(m.file_name) {dir}, m.id ASC"
            ),
            SortField::Date => format!("m.created_at {dir}, m.id ASC"),
            SortField::Size => format!("m.size {dir}, m.id ASC"),
            SortField::Duration => format!("m.duration_ms {dir}, m.id ASC"),
            SortField::Type => format!("m.type {dir}, m.id ASC"),
        };

        let conn = self.lock()?;

        let count_sql = format!("SELECT COUNT(*) FROM media m {fts_join} {where_sql}");
        let total_count: u64 = conn.query_row(
            &count_sql,
            rusqlite::params_from_iter(bind_params.iter()),
            |r| Ok(r.get::<_, i64>(0)? as u64),
        )?;

        let mut query_params = bind_params;
        query_params.push(Value::Integer(safe_limit as i64));
        query_params.push(Value::Integer(offset as i64));

        let list_sql = format!(
            "SELECT m.*,
                    d.name AS device_name, d.is_online AS device_online,
                    c.id AS c_id, c.source AS c_source, c.file_path AS c_file_path, c.cloud_id AS c_cloud_id,
                    c.aspect AS c_aspect, c.crop AS c_crop, c.frame_time_ms AS c_frame_time_ms, c.updated_at AS c_updated_at
             FROM media m
             {fts_join}
             LEFT JOIN device d ON d.id = m.device_id
             LEFT JOIN cover c ON c.media_id = m.id
             {where_sql}
             ORDER BY {order_expr}
             LIMIT ? OFFSET ?"
        );

        let mut stmt = conn.prepare(&list_sql)?;
        let rows = stmt.query_map(
            rusqlite::params_from_iter(query_params.iter()),
            map_media_with_cover_row,
        )?;
        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }

        let next_offset = offset + (items.len() as u64);
        let next_cursor = if next_offset < total_count && !items.is_empty() {
            Some(next_offset.to_string())
        } else {
            None
        };

        Ok(MediaPage {
            items,
            next_cursor,
            total_count,
        })
    }

    pub fn list_folders(
        &self,
        device_id: Option<&str>,
        media_type: Option<MediaType>,
    ) -> Result<Vec<FolderSummary>, IndexError> {
        let mut clauses = vec!["m.is_trashed = 0".to_string()];
        let mut vals: Vec<Value> = Vec::new();

        if let Some(dev) = device_id {
            clauses.push("m.device_id = ?".to_string());
            vals.push(Value::Text(dev.to_string()));
        }
        if let Some(mt) = media_type {
            clauses.push("m.type = ?".to_string());
            vals.push(Value::Text(mt.as_str().to_string()));
        }

        let sql = format!(
            "SELECT m.folder_path, m.device_id, COUNT(*) AS cnt
             FROM media m
             WHERE {}
             GROUP BY m.device_id, m.folder_path
             ORDER BY cnt DESC, m.folder_path ASC",
            clauses.join(" AND ")
        );

        let folder_rows: Vec<(String, String, u64)> = {
            let conn = self.lock()?;
            let mut stmt = conn.prepare(&sql)?;
            let mapped = stmt.query_map(rusqlite::params_from_iter(vals.iter()), |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)? as u64,
                ))
            })?;
            let mut list = Vec::new();
            for item in mapped {
                list.push(item?);
            }
            list
        };

        let mut summaries = Vec::with_capacity(folder_rows.len());
        for (folder_path, dev_id, item_count) in folder_rows {
            let page = self.list_media(
                &MediaFilter {
                    device_id: Some(dev_id.clone()),
                    folder_path: Some(folder_path.clone()),
                    media_type,
                    ..Default::default()
                },
                SortSpec::default(),
                None,
                4,
            )?;
            let preview_covers = page
                .items
                .into_iter()
                .filter_map(|item| item.cover.map(|c| c.file_path))
                .filter(|p| !p.is_empty())
                .collect();
            let name = folder_path
                .replace('\\', "/")
                .split('/')
                .rfind(|s| !s.is_empty())
                .unwrap_or(&folder_path)
                .to_string();

            summaries.push(FolderSummary {
                path: folder_path,
                name,
                device_id: dev_id,
                item_count,
                preview_covers,
            });
        }
        Ok(summaries)
    }

    pub fn detect_and_flag_duplicates(
        &self,
        device_id: Option<&str>,
    ) -> Result<Vec<DuplicateGroup>, IndexError> {
        let page = self.list_media(
            &MediaFilter {
                device_id: device_id.map(str::to_string),
                ..Default::default()
            },
            SortSpec::default(),
            None,
            10_000,
        )?;

        let mut buckets: HashMap<(u64, String), Vec<MediaWithCover>> = HashMap::new();
        for item in page.items {
            if item.media.partial_hash.is_empty() {
                continue;
            }
            buckets
                .entry((item.media.size, item.media.partial_hash.clone()))
                .or_default()
                .push(item);
        }

        let mut groups = Vec::new();
        let mut updates: Vec<(String, bool, Option<String>)> = Vec::new();

        for ((size, phash), items) in buckets {
            if items.len() > 1 {
                let prefix = &phash[..phash.len().min(16)];
                let group_id = format!("dup-{size}-{prefix}");
                for item in &items {
                    updates.push((item.media.id.clone(), true, Some(group_id.clone())));
                }
                groups.push(DuplicateGroup {
                    group_id,
                    size,
                    partial_hash: phash,
                    items,
                });
            } else if items[0].media.is_duplicate {
                updates.push((items[0].media.id.clone(), false, None));
            }
        }

        if !updates.is_empty() {
            let mut conn = self.lock()?;
            let tx = conn.transaction()?;
            let ts = now_ms();
            for (id, is_dup, gid) in updates {
                tx.execute(
                    "UPDATE media SET is_duplicate = ?1, duplicate_group_id = ?2, updated_at = ?3 WHERE id = ?4",
                    params![i64::from(is_dup), gid, ts, id],
                )?;
            }
            tx.commit()?;
        }

        Ok(groups)
    }

    pub fn update_alias(&self, media_id: &str, alias: &str) -> Result<(), IndexError> {
        let conn = self.lock()?;
        conn.execute(
            "UPDATE media SET alias = ?1, updated_at = ?2 WHERE id = ?3",
            params![alias, now_ms(), media_id],
        )?;
        Ok(())
    }

    pub fn update_path_and_name(
        &self,
        media_id: &str,
        new_path: &str,
        new_file_name: &str,
        new_folder_path: &str,
    ) -> Result<(), IndexError> {
        let norm_path = normalize_db_path(new_path);
        let norm_folder = normalize_db_path(new_folder_path);
        let conn = self.lock()?;
        conn.execute(
            "UPDATE media SET path = ?1, file_name = ?2, folder_path = ?3, updated_at = ?4 WHERE id = ?5",
            params![norm_path, new_file_name, norm_folder, now_ms(), media_id],
        )?;
        Ok(())
    }

    pub fn update_sync_state(&self, media_id: &str, state: SyncState) -> Result<(), IndexError> {
        let conn = self.lock()?;
        conn.execute(
            "UPDATE media SET sync_state = ?1, updated_at = ?2 WHERE id = ?3",
            params![state.as_str(), now_ms(), media_id],
        )?;
        Ok(())
    }

    pub fn mark_media_trashed(&self, media_id: &str, trashed_path: &str) -> Result<(), IndexError> {
        let conn = self.lock()?;
        conn.execute(
            "UPDATE media SET is_trashed = 1, trashed_path = ?1, updated_at = ?2 WHERE id = ?3",
            params![trashed_path, now_ms(), media_id],
        )?;
        Ok(())
    }

    pub fn delete_media_by_device_path(
        &self,
        device_id: &str,
        file_path: &str,
    ) -> Result<(), IndexError> {
        let norm_path = normalize_db_path(file_path);
        let conn = self.lock()?;
        conn.execute(
            "DELETE FROM media WHERE device_id = ?1 AND path = ?2",
            params![device_id, norm_path],
        )?;
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Covers
    // -------------------------------------------------------------------------

    pub fn upsert_cover(&self, cover: &CoverRecord) -> Result<(), IndexError> {
        let mut conn = self.lock()?;
        let tx = conn.transaction()?;
        tx.execute(
            "INSERT INTO cover (id, media_id, source, file_path, cloud_id, aspect, crop, frame_time_ms, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(media_id) DO UPDATE SET
               source = excluded.source,
               file_path = excluded.file_path,
               cloud_id = COALESCE(excluded.cloud_id, cover.cloud_id),
               aspect = excluded.aspect,
               crop = excluded.crop,
               frame_time_ms = excluded.frame_time_ms,
               updated_at = excluded.updated_at",
            params![
                cover.id,
                cover.media_id,
                cover.source.as_str(),
                cover.file_path,
                cover.cloud_id,
                cover.aspect.as_str(),
                cover.crop.as_ref().and_then(|c| serde_json::to_string(c).ok()),
                cover.frame_time_ms as i64,
                cover.updated_at,
            ],
        )?;
        tx.execute(
            "UPDATE media SET cover_id = ?1, updated_at = ?2 WHERE id = ?3",
            params![cover.id, cover.updated_at, cover.media_id],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn get_cover_by_media_id(&self, media_id: &str) -> Result<Option<CoverRecord>, IndexError> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare("SELECT * FROM cover WHERE media_id = ?1")?;
        let mut rows = stmt.query([media_id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(map_cover_row(row)?))
        } else {
            Ok(None)
        }
    }

    // -------------------------------------------------------------------------
    // Transfers
    // -------------------------------------------------------------------------

    pub fn upsert_transfer(&self, tr: &TransferRecord) -> Result<(), IndexError> {
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO transfer (
               id, media_id, file_name, from_device, to_device, status, bytes_done, bytes_total,
               last_chunk_index, chunk_size, path_kind, speed_bps, expected_blake3, error, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
             ON CONFLICT(id) DO UPDATE SET
               status = excluded.status,
               bytes_done = excluded.bytes_done,
               bytes_total = excluded.bytes_total,
               last_chunk_index = excluded.last_chunk_index,
               chunk_size = excluded.chunk_size,
               path_kind = excluded.path_kind,
               speed_bps = excluded.speed_bps,
               expected_blake3 = excluded.expected_blake3,
               error = excluded.error,
               updated_at = excluded.updated_at",
            params![
                tr.id,
                tr.media_id,
                tr.file_name,
                tr.from_device,
                tr.to_device,
                tr.status.as_str(),
                tr.bytes_done as i64,
                tr.bytes_total as i64,
                tr.last_chunk_index as i64,
                tr.chunk_size as i64,
                tr.path_kind,
                tr.speed_bps as i64,
                tr.expected_blake3,
                tr.error,
                tr.updated_at,
            ],
        )?;
        Ok(())
    }

    pub fn get_transfer(&self, id: &str) -> Result<Option<TransferRecord>, IndexError> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare("SELECT * FROM transfer WHERE id = ?1")?;
        let mut rows = stmt.query([id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(map_transfer_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn list_transfers(&self) -> Result<Vec<TransferRecord>, IndexError> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare("SELECT * FROM transfer ORDER BY updated_at DESC")?;
        let rows = stmt.query_map([], map_transfer_row)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    // -------------------------------------------------------------------------
    // Pinned Folders
    // -------------------------------------------------------------------------

    pub fn list_pinned_folders(&self) -> Result<Vec<PinnedFolderRecord>, IndexError> {
        let conn = self.lock()?;
        let mut stmt =
            conn.prepare("SELECT * FROM pinned_folder ORDER BY position ASC, label ASC")?;
        let rows = stmt.query_map([], |row| {
            Ok(PinnedFolderRecord {
                id: row.get("id")?,
                device_id: row.get("device_id")?,
                path: row.get("path")?,
                label: row.get("label")?,
                icon: row.get("icon")?,
                position: row.get("position")?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn upsert_pinned_folder(&self, pf: &PinnedFolderRecord) -> Result<(), IndexError> {
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO pinned_folder (id, device_id, path, label, icon, position)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET
               device_id = excluded.device_id,
               path = excluded.path,
               label = excluded.label,
               icon = excluded.icon,
               position = excluded.position",
            params![pf.id, pf.device_id, pf.path, pf.label, pf.icon, pf.position],
        )?;
        Ok(())
    }

    pub fn remove_pinned_folder(&self, id: &str) -> Result<(), IndexError> {
        let conn = self.lock()?;
        conn.execute("DELETE FROM pinned_folder WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn reorder_pinned_folders(&self, ordered_ids: &[String]) -> Result<(), IndexError> {
        let mut conn = self.lock()?;
        let tx = conn.transaction()?;
        for (idx, id) in ordered_ids.iter().enumerate() {
            tx.execute(
                "UPDATE pinned_folder SET position = ?1 WHERE id = ?2",
                params![idx as i64, id],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Rename Log
    // -------------------------------------------------------------------------

    pub fn insert_rename_log(&self, entry: &RenameLogRecord) -> Result<(), IndexError> {
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO rename_log (id, media_id, batch_id, old_name, new_name, old_path, new_path, applied_at, undone_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(id) DO NOTHING",
            params![
                entry.id,
                entry.media_id,
                entry.batch_id,
                entry.old_name,
                entry.new_name,
                entry.old_path,
                entry.new_path,
                entry.applied_at,
                entry.undone_at,
            ],
        )?;
        Ok(())
    }

    pub fn get_latest_undoable_rename(
        &self,
        media_id: Option<&str>,
    ) -> Result<Option<RenameLogRecord>, IndexError> {
        let conn = self.lock()?;
        let sql = if media_id.is_some() {
            "SELECT * FROM rename_log WHERE media_id = ?1 AND undone_at IS NULL ORDER BY applied_at DESC LIMIT 1"
        } else {
            "SELECT * FROM rename_log WHERE undone_at IS NULL ORDER BY applied_at DESC LIMIT 1"
        };
        let mut stmt = conn.prepare(sql)?;
        let mut rows = if let Some(mid) = media_id {
            stmt.query([mid])?
        } else {
            stmt.query([])?
        };
        if let Some(row) = rows.next()? {
            Ok(Some(map_rename_log_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn get_rename_batch(&self, batch_id: &str) -> Result<Vec<RenameLogRecord>, IndexError> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT * FROM rename_log WHERE batch_id = ?1 AND undone_at IS NULL ORDER BY applied_at DESC",
        )?;
        let rows = stmt.query_map([batch_id], map_rename_log_row)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn mark_rename_undone(&self, id: &str) -> Result<(), IndexError> {
        let conn = self.lock()?;
        conn.execute(
            "UPDATE rename_log SET undone_at = ?1 WHERE id = ?2",
            params![now_ms(), id],
        )?;
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Download Jobs
    // -------------------------------------------------------------------------

    pub fn upsert_download_job(&self, job: &DownloadJobRecord) -> Result<(), IndexError> {
        let conn = self.lock()?;
        let fmt_str = match job.format {
            DownloadFormat::Mp4 => "mp4",
            DownloadFormat::Mp3 => "mp3",
        };
        let status_str = match job.status {
            DownloadStatus::Queued => "queued",
            DownloadStatus::Downloading => "downloading",
            DownloadStatus::Processing => "processing",
            DownloadStatus::Completed => "completed",
            DownloadStatus::Error => "error",
        };
        conn.execute(
            "INSERT INTO download_job (
               id, url, title, format, quality, destination_device_id, destination_folder,
               status, progress, output_media_id, error, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
             ON CONFLICT(id) DO UPDATE SET
               title = COALESCE(excluded.title, download_job.title),
               status = excluded.status,
               progress = excluded.progress,
               output_media_id = COALESCE(excluded.output_media_id, download_job.output_media_id),
               error = excluded.error,
               updated_at = excluded.updated_at",
            params![
                job.id,
                job.url,
                job.title,
                fmt_str,
                job.quality,
                job.destination_device_id,
                job.destination_folder,
                status_str,
                f64::from(job.progress),
                job.output_media_id,
                job.error,
                job.created_at,
                job.updated_at,
            ],
        )?;
        Ok(())
    }

    pub fn list_download_jobs(&self) -> Result<Vec<DownloadJobRecord>, IndexError> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare("SELECT * FROM download_job ORDER BY created_at DESC")?;
        let rows = stmt.query_map([], map_download_job_row)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    // -------------------------------------------------------------------------
    // Settings, Saved Searches & Auto-Sync Rules
    // -------------------------------------------------------------------------

    pub fn get_setting(&self, key: &str) -> Result<Option<String>, IndexError> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?1")?;
        let mut rows = stmt.query([key])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), IndexError> {
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn get_app_settings(&self) -> Result<AppSettings, IndexError> {
        let Some(raw) = self.get_setting("app_settings")? else {
            return Ok(AppSettings::default());
        };
        Ok(serde_json::from_str(&raw).unwrap_or_default())
    }

    pub fn save_app_settings(&self, settings: &AppSettings) -> Result<(), IndexError> {
        let json = serde_json::to_string(settings)?;
        self.set_setting("app_settings", &json)
    }

    pub fn save_search(&self, saved: &SavedSearchRecord) -> Result<(), IndexError> {
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO saved_search (id, name, query, filters, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET name = excluded.name, query = excluded.query, filters = excluded.filters",
            params![
                saved.id,
                saved.name,
                saved.query,
                serde_json::to_string(&saved.filter)?,
                saved.created_at,
            ],
        )?;
        Ok(())
    }

    pub fn list_saved_searches(&self) -> Result<Vec<SavedSearchRecord>, IndexError> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare("SELECT * FROM saved_search ORDER BY created_at DESC")?;
        let rows = stmt.query_map([], |row| {
            let f_json: String = row.get("filters")?;
            Ok(SavedSearchRecord {
                id: row.get("id")?,
                name: row.get("name")?,
                query: row.get("query")?,
                filter: serde_json::from_str(&f_json).unwrap_or_default(),
                created_at: row.get("created_at")?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn upsert_auto_sync_rule(&self, rule: &AutoSyncRuleRecord) -> Result<(), IndexError> {
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO auto_sync_rule (id, name, source_device_id, target_device_id, folder_pattern, media_types, enabled, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(id) DO UPDATE SET
               name = excluded.name,
               source_device_id = excluded.source_device_id,
               target_device_id = excluded.target_device_id,
               folder_pattern = excluded.folder_pattern,
               media_types = excluded.media_types,
               enabled = excluded.enabled",
            params![
                rule.id,
                rule.name,
                rule.source_device_id,
                rule.target_device_id,
                rule.folder_pattern,
                serde_json::to_string(&rule.media_types)?,
                i64::from(rule.enabled),
                rule.created_at,
            ],
        )?;
        Ok(())
    }

    pub fn list_auto_sync_rules(&self) -> Result<Vec<AutoSyncRuleRecord>, IndexError> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare("SELECT * FROM auto_sync_rule ORDER BY created_at DESC")?;
        let rows = stmt.query_map([], |row| {
            let types_json: String = row.get("media_types")?;
            Ok(AutoSyncRuleRecord {
                id: row.get("id")?,
                name: row.get("name")?,
                source_device_id: row.get("source_device_id")?,
                target_device_id: row.get("target_device_id")?,
                folder_pattern: row.get("folder_pattern")?,
                media_types: serde_json::from_str(&types_json).unwrap_or_default(),
                enabled: row.get::<_, i64>("enabled")? != 0,
                created_at: row.get("created_at")?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    // -------------------------------------------------------------------------
    // Authoritative Delta Export & Import
    // -------------------------------------------------------------------------

    pub fn export_delta_since(
        &self,
        device_id: &str,
        since_timestamp: i64,
    ) -> Result<IndexDelta, IndexError> {
        let device = self
            .get_device(device_id)?
            .ok_or_else(|| IndexError::NotFound(format!("unknown device {device_id}")))?;

        let conn = self.lock()?;
        let mut m_stmt = conn.prepare(
            "SELECT * FROM media WHERE device_id = ?1 AND is_trashed = 0 AND updated_at > ?2",
        )?;
        let upserted_media = m_stmt
            .query_map(params![device_id, since_timestamp], map_media_row)?
            .collect::<Result<Vec<_>, _>>()?;

        let mut d_stmt = conn.prepare(
            "SELECT id FROM media WHERE device_id = ?1 AND is_trashed = 1 AND updated_at > ?2",
        )?;
        let deleted_media_ids = d_stmt
            .query_map(params![device_id, since_timestamp], |r| r.get(0))?
            .collect::<Result<Vec<String>, _>>()?;

        let mut c_stmt = conn.prepare(
            "SELECT c.* FROM cover c
             INNER JOIN media m ON m.id = c.media_id
             WHERE m.device_id = ?1 AND c.updated_at > ?2",
        )?;
        let upserted_covers = c_stmt
            .query_map(params![device_id, since_timestamp], map_cover_row)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(IndexDelta {
            device,
            generated_at: now_ms(),
            since_timestamp,
            upserted_media,
            upserted_covers,
            deleted_media_ids,
            thumbnail_blobs_base64: HashMap::new(),
        })
    }

    pub fn apply_remote_delta(
        &self,
        delta: &IndexDelta,
        self_device_id: &str,
    ) -> Result<u64, IndexError> {
        if delta.device.id == self_device_id {
            return Ok(0);
        }

        let mut dev = delta.device.clone();
        dev.is_self = false;
        self.upsert_device(&dev)?;

        let mut authoritative_items = Vec::new();
        for mut item in delta.upserted_media.iter().cloned() {
            if item.device_id != delta.device.id {
                continue;
            }
            let existing = if let Some(found) = self.get_media_by_id(&item.id)? {
                Some(found.media)
            } else {
                self.get_media_by_path(&item.device_id, &item.path)?
            };
            item.sync_state = match existing {
                Some(ref ex) if ex.sync_state == SyncState::Synced => SyncState::Synced,
                _ => SyncState::Remote,
            };
            authoritative_items.push(item);
        }

        let imported_count = authoritative_items.len() as u64;
        self.upsert_media_batch(&authoritative_items)?;

        for cover in &delta.upserted_covers {
            if let Some(m) = self.get_media_by_id(&cover.media_id)? {
                if m.media.device_id == delta.device.id {
                    self.upsert_cover(cover)?;
                }
            }
        }

        let conn = self.lock()?;
        for deleted_id in &delta.deleted_media_ids {
            conn.execute(
                "DELETE FROM media WHERE id = ?1 AND device_id = ?2",
                params![deleted_id, delta.device.id],
            )?;
        }

        Ok(imported_count)
    }
}

fn map_transfer_row(row: &Row<'_>) -> Result<TransferRecord, rusqlite::Error> {
    let status_raw: String = row.get("status")?;
    Ok(TransferRecord {
        id: row.get("id")?,
        media_id: row.get("media_id")?,
        file_name: row.get("file_name")?,
        from_device: row.get("from_device")?,
        to_device: row.get("to_device")?,
        status: TransferStatus::parse(&status_raw),
        bytes_done: row.get::<_, i64>("bytes_done")? as u64,
        bytes_total: row.get::<_, i64>("bytes_total")? as u64,
        last_chunk_index: row.get::<_, i64>("last_chunk_index")? as u64,
        chunk_size: row.get::<_, i64>("chunk_size")? as u64,
        path_kind: row.get("path_kind")?,
        speed_bps: row.get::<_, i64>("speed_bps")? as u64,
        expected_blake3: row.get("expected_blake3")?,
        error: row.get("error")?,
        updated_at: row.get("updated_at")?,
    })
}

fn map_rename_log_row(row: &Row<'_>) -> Result<RenameLogRecord, rusqlite::Error> {
    Ok(RenameLogRecord {
        id: row.get("id")?,
        media_id: row.get("media_id")?,
        batch_id: row.get("batch_id")?,
        old_name: row.get("old_name")?,
        new_name: row.get("new_name")?,
        old_path: row.get("old_path")?,
        new_path: row.get("new_path")?,
        applied_at: row.get("applied_at")?,
        undone_at: row.get("undone_at")?,
    })
}

fn map_download_job_row(row: &Row<'_>) -> Result<DownloadJobRecord, rusqlite::Error> {
    let fmt_raw: String = row.get("format")?;
    let status_raw: String = row.get("status")?;
    let format = if fmt_raw == "mp3" {
        DownloadFormat::Mp3
    } else {
        DownloadFormat::Mp4
    };
    let status = match status_raw.as_str() {
        "downloading" => DownloadStatus::Downloading,
        "processing" => DownloadStatus::Processing,
        "completed" => DownloadStatus::Completed,
        "error" => DownloadStatus::Error,
        _ => DownloadStatus::Queued,
    };
    Ok(DownloadJobRecord {
        id: row.get("id")?,
        url: row.get("url")?,
        title: row.get("title")?,
        format,
        quality: row.get("quality")?,
        destination_device_id: row.get("destination_device_id")?,
        destination_folder: row.get("destination_folder")?,
        status,
        progress: row.get::<_, f64>("progress")? as f32,
        output_media_id: row.get("output_media_id")?,
        error: row.get("error")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

fn build_where_clause(filter: &MediaFilter) -> (String, Vec<Value>, bool) {
    let mut clauses = Vec::new();
    let mut params = Vec::new();
    let mut join_fts = false;

    let fts_match = build_fts_match_query(&filter.query);
    if !fts_match.is_empty() {
        join_fts = true;
        clauses.push("media_fts MATCH ?".to_string());
        params.push(Value::Text(fts_match));
    }

    if !filter.include_trashed {
        clauses.push("m.is_trashed = 0".to_string());
    }
    if let Some(mt) = filter.media_type {
        clauses.push("m.type = ?".to_string());
        params.push(Value::Text(mt.as_str().to_string()));
    }
    if let Some(ref dev) = filter.device_id {
        if dev != "all" {
            clauses.push("m.device_id = ?".to_string());
            params.push(Value::Text(dev.clone()));
        }
    }
    if let Some(sync) = filter.sync_state {
        clauses.push("m.sync_state = ?".to_string());
        params.push(Value::Text(sync.as_str().to_string()));
    }
    if let Some(ref folder) = filter.folder_path {
        clauses.push("m.folder_path = ?".to_string());
        params.push(Value::Text(folder.clone()));
    }
    if let Some(d_from) = filter.date_from {
        clauses.push("m.created_at >= ?".to_string());
        params.push(Value::Integer(d_from));
    }
    if let Some(d_to) = filter.date_to {
        clauses.push("m.created_at <= ?".to_string());
        params.push(Value::Integer(d_to));
    }
    if let Some(min_sz) = filter.min_size {
        clauses.push("m.size >= ?".to_string());
        params.push(Value::Integer(min_sz as i64));
    }
    if let Some(max_sz) = filter.max_size {
        clauses.push("m.size <= ?".to_string());
        params.push(Value::Integer(max_sz as i64));
    }
    if let Some(min_dur) = filter.min_duration_ms {
        clauses.push("m.duration_ms >= ?".to_string());
        params.push(Value::Integer(min_dur as i64));
    }
    if let Some(min_w) = filter.min_width {
        clauses.push("m.width >= ?".to_string());
        params.push(Value::Integer(i64::from(min_w)));
    }
    if let Some(tod) = filter.time_of_day {
        let hour_expr =
            "CAST(strftime('%H', m.created_at / 1000, 'unixepoch', 'localtime') AS INTEGER)";
        match tod {
            TimeOfDayFilter::All => {}
            TimeOfDayFilter::Morning => clauses.push(format!("{hour_expr} BETWEEN 5 AND 11")),
            TimeOfDayFilter::Afternoon => clauses.push(format!("{hour_expr} BETWEEN 12 AND 16")),
            TimeOfDayFilter::Evening => clauses.push(format!("{hour_expr} BETWEEN 17 AND 20")),
            TimeOfDayFilter::Night => {
                clauses.push(format!("({hour_expr} >= 21 OR {hour_expr} <= 4)"))
            }
        }
    }

    let where_sql = if clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", clauses.join(" AND "))
    };
    (where_sql, params, join_fts)
}
