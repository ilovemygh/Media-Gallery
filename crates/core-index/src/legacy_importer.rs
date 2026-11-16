use std::path::Path;

use rusqlite::{Connection, OpenFlags};

use crate::models::{
    AppSettings, AspectPreset, AutoSyncRuleRecord, CoverRecord, CoverSource, CropRect,
    DeviceRecord, DeviceType, LegacyImportReport, MediaRecord, MediaTags, MediaType,
    PinnedFolderRecord, RenameLogRecord, SavedSearchRecord, SyncState, TransportKind,
};
use crate::repository::{now_ms, IndexError, MediaIndex};

/// Idempotent importer reading v0.1 SQLite databases read-only into the v0.2 Rust index.
///
/// - Never writes to or alters legacy database files.
/// - Preserves existing aliases, fixed video `frame_time_ms` thumbnails, pinned folders,
///   rename logs, saved searches, auto-sync rules, and non-secret settings.
/// - Never migrates plaintext/ephemeral auth tokens (`auth_relogin_required = true`).
pub fn import_legacy_sqlite(
    target: &MediaIndex,
    legacy_db_path: &Path,
) -> Result<LegacyImportReport, IndexError> {
    if !legacy_db_path.exists() {
        return Err(IndexError::NotFound(format!(
            "legacy database not found: {}",
            legacy_db_path.display()
        )));
    }

    let legacy = Connection::open_with_flags(
        legacy_db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;

    let mut report = LegacyImportReport {
        auth_relogin_required: true,
        ..Default::default()
    };

    // 1. Import devices (strip legacy keys/tokens so peers re-verify safely if needed, keeping paired allow-list metadata)
    {
        let mut stmt = legacy.prepare(
            "SELECT id, name, type, public_key, last_seen, transports, host, port, is_self, is_online, paired_at FROM device",
        )?;
        let rows = stmt.query_map([], |row| {
            let transports_raw: String = row.get("transports")?;
            let legacy_transports: Vec<String> =
                serde_json::from_str(&transports_raw).unwrap_or_default();
            let transports = legacy_transports
                .into_iter()
                .filter_map(|t| match t.as_str() {
                    "wired" => Some(TransportKind::Wired),
                    "wifi_lan" => Some(TransportKind::WifiLan),
                    "wifi_direct" => Some(TransportKind::HolePunched),
                    "bluetooth" => Some(TransportKind::Bluetooth),
                    _ => None,
                })
                .collect();
            let dtype: String = row.get("type")?;
            Ok(DeviceRecord {
                id: row.get("id")?,
                name: row.get("name")?,
                device_type: DeviceType::parse(&dtype),
                public_key: row.get("public_key")?,
                last_seen: row.get("last_seen")?,
                transports,
                host: row.get("host")?,
                port: row.get::<_, Option<i64>>("port")?.map(|p| p as u16),
                is_self: row.get::<_, i64>("is_self")? != 0,
                is_online: row.get::<_, i64>("is_online")? != 0,
                paired_at: row.get("paired_at")?,
            })
        })?;
        for dev in rows {
            match dev {
                Ok(d) => {
                    target.upsert_device(&d)?;
                    report.imported_devices += 1;
                }
                Err(e) => report.errors.push(format!("device row error: {e}")),
            }
        }
    }

    // 2. Import media records (legacy `hash` stored SHA-256; keep partial_hash empty for BLAKE3 re-hash or carry forward)
    {
        let mut stmt = legacy.prepare("SELECT * FROM media")?;
        let mut rows = stmt.query([])?;
        let mut batch = Vec::new();
        while let Some(row) = rows.next()? {
            let id: String = row.get("id")?;
            let existing = target.get_media_by_id(&id)?;
            if existing.is_some() {
                report.skipped_items += 1;
                continue;
            }
            let type_raw: String = row.get("type")?;
            let sync_raw: String = row.get("sync_state")?;
            let tags_raw: String = row.get("tags")?;
            let tags: MediaTags = serde_json::from_str(&tags_raw).unwrap_or_default();

            batch.push(MediaRecord {
                id,
                device_id: row.get("device_id")?,
                path: row.get("path")?,
                folder_path: row.get("folder_path")?,
                file_name: row.get("file_name")?,
                media_type: MediaType::parse(&type_raw),
                mime: row.get("mime")?,
                size: row.get::<_, i64>("size")? as u64,
                blake3: String::new(),
                partial_hash: row.get::<_, Option<String>>("partial_hash")?.unwrap_or_default(),
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
                updated_at: row.get::<_, Option<i64>>("updated_at")?.unwrap_or_else(now_ms),
            });
        }
        report.imported_media = batch.len() as u64;
        target.upsert_media_batch(&batch)?;
    }

    // 3. Import covers (preserving fixed video frame_time_ms & custom cover paths)
    {
        let mut stmt = legacy.prepare("SELECT * FROM cover")?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let media_id: String = row.get("media_id")?;
            if target.get_cover_by_media_id(&media_id)?.is_some() {
                continue;
            }
            let src_raw: String = row.get("source")?;
            let asp_raw: String = row.get("aspect")?;
            let crop_raw: Option<String> = row.get("crop")?;
            let cover = CoverRecord {
                id: row.get("id")?,
                media_id,
                source: CoverSource::parse(&src_raw),
                file_path: row.get("file_path")?,
                cloud_id: row.get("cloud_id")?,
                aspect: AspectPreset::parse(&asp_raw),
                crop: crop_raw.and_then(|s| serde_json::from_str::<CropRect>(&s).ok()),
                frame_time_ms: row.get::<_, i64>("frame_time_ms")? as u64,
                updated_at: row.get("updated_at")?,
            };
            target.upsert_cover(&cover)?;
            report.imported_covers += 1;
        }
    }

    // 4. Import pinned folders
    {
        let mut stmt = legacy.prepare("SELECT * FROM pinned_folder")?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let pf = PinnedFolderRecord {
                id: row.get("id")?,
                device_id: row.get("device_id")?,
                path: row.get("path")?,
                label: row.get("label")?,
                icon: row.get("icon")?,
                position: row.get("position")?,
            };
            target.upsert_pinned_folder(&pf)?;
            report.imported_pinned_folders += 1;
        }
    }

    // 5. Import rename logs (preserves undo history!)
    {
        let mut stmt = legacy.prepare("SELECT * FROM rename_log")?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let log = RenameLogRecord {
                id: row.get("id")?,
                media_id: row.get("media_id")?,
                batch_id: row.get("batch_id")?,
                old_name: row.get("old_name")?,
                new_name: row.get("new_name")?,
                old_path: row.get("old_path")?,
                new_path: row.get("new_path")?,
                applied_at: row.get("applied_at")?,
                undone_at: row.get("undone_at")?,
            };
            target.insert_rename_log(&log)?;
            report.imported_rename_logs += 1;
        }
    }

    // 6. Import saved searches & auto-sync rules if present
    if table_exists(&legacy, "saved_search") {
        let mut stmt = legacy.prepare("SELECT * FROM saved_search")?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let id: String = row.get("id")?;
            let saved = SavedSearchRecord {
                id,
                name: row.get("name")?,
                query: row.get("query")?,
                filter: Default::default(),
                created_at: row.get("created_at")?,
            };
            target.save_search(&saved)?;
            report.imported_saved_searches += 1;
        }
    }

    if table_exists(&legacy, "auto_sync_rule") {
        let mut stmt = legacy.prepare("SELECT * FROM auto_sync_rule")?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let types_raw: String = row.get("media_types")?;
            let types: Vec<MediaType> = serde_json::from_str(&types_raw).unwrap_or_default();
            let rule = AutoSyncRuleRecord {
                id: row.get("id")?,
                name: row.get("name")?,
                source_device_id: row.get("source_device_id")?,
                target_device_id: row.get("target_device_id")?,
                folder_pattern: row.get("folder_pattern")?,
                media_types: types,
                enabled: row.get::<_, i64>("enabled")? != 0,
                created_at: row.get("created_at")?,
            };
            target.upsert_auto_sync_rule(&rule)?;
            report.imported_auto_sync_rules += 1;
        }
    }

    // 7. Import non-secret app_settings
    if let Ok(raw_settings) =
        legacy.query_row::<String, _, _>("SELECT value FROM settings WHERE key = 'app_settings'", [], |r| r.get(0))
    {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&raw_settings) {
            let mut settings = AppSettings::default();
            if let Some(w) = parsed.get("sidebarWidth").and_then(|v| v.as_u64()) {
                settings.sidebar_width = w as u32;
            }
            if let Some(c) = parsed.get("sidebarCollapsed").and_then(|v| v.as_bool()) {
                settings.sidebar_collapsed = c;
            }
            if let Some(t) = parsed.get("theme").and_then(|v| v.as_str()) {
                settings.theme = t.to_string();
            }
            if let Some(tpl) = parsed.get("aliasTemplate").and_then(|v| v.as_str()) {
                settings.alias_template = tpl.to_string();
            }
            if let Some(geo) = parsed.get("onlineGeocodingEnabled").and_then(|v| v.as_bool()) {
                settings.online_geocoding_enabled = geo;
            }
            target.save_app_settings(&settings)?;
        }
    }

    Ok(report)
}

fn table_exists(conn: &Connection, table: &str) -> bool {
    conn.query_row::<i64, _, _>(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
        [table],
        |r| r.get(0),
    )
    .unwrap_or(0)
        > 0
}
