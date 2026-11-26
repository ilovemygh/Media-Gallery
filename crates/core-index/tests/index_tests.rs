use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use core_index::{
    compute_file_blake3, compute_partial_blake3, import_legacy_sqlite,
    ingest_android_mediastore_batch, scan_folders, AppSettings, AspectPreset, CoverRecord,
    CoverSource, DeviceRecord, DeviceType, MediaFilter, MediaIndex, MediaRecord, MediaTags,
    MediaType, ProbedMediaMetadata, ScanRequest, SortSpec, SyncState, TransportKind,
};
use core_platform::AndroidMediaStoreRow;

fn temp_dir(prefix: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("{prefix}-{nanos}"));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn fresh_settings_default_to_light_theme() {
    assert_eq!(AppSettings::default().theme, "light");
}

fn setup_index_with_self() -> MediaIndex {
    let idx = MediaIndex::open_in_memory().unwrap();
    idx.upsert_device(&DeviceRecord {
        id: "pc-1".to_string(),
        name: "Workstation".to_string(),
        device_type: DeviceType::Pc,
        public_key: "pk-pc-1".to_string(),
        last_seen: 1_700_000_000_000,
        transports: vec![TransportKind::WifiLan],
        host: Some("127.0.0.1".to_string()),
        port: Some(9443),
        is_self: true,
        is_online: true,
        paired_at: None,
    })
    .unwrap();
    idx
}

#[test]
fn scans_all_16_formats_skips_excluded_and_flags_duplicates_with_blake3() {
    let root = temp_dir("mg-idx-scan");
    let lib = root.join("library");
    let excluded = lib.join("excluded_cache");
    std::fs::create_dir_all(&lib).unwrap();
    std::fs::create_dir_all(&excluded).unwrap();

    let supported = [
        "a.mp4", "b.mkv", "c.webm", "d.mov", "e.avi", "f.mp3", "g.m4a", "h.flac", "i.wav", "j.ogg",
        "k.png", "l.jpg", "m.jpeg", "n.gif", "o.webp", "p.heic",
    ];
    for name in supported {
        std::fs::write(lib.join(name), format!("unique-content-{name}")).unwrap();
    }
    std::fs::write(lib.join("notes.txt"), "not media").unwrap();
    std::fs::write(excluded.join("skip.jpg"), "excluded").unwrap();

    // Add duplicate payload
    let dup_bytes = vec![42u8; 4096];
    std::fs::write(lib.join("dup1.jpg"), &dup_bytes).unwrap();
    std::fs::write(lib.join("dup2.jpg"), &dup_bytes).unwrap();

    let idx = setup_index_with_self();
    let cancel = AtomicBool::new(false);
    let req = ScanRequest {
        operation_id: "op-1".to_string(),
        device_id: "pc-1".to_string(),
        device_name: "Workstation".to_string(),
        roots: vec![lib.clone()],
        excluded_folders: vec![excluded],
        compute_hashes: true,
        batch_size: 100,
        cancel: Some(&cancel),
    };

    let progress = scan_folders(
        &idx,
        &req,
        &|path, _mt, _c, _m, _dev| ProbedMediaMetadata {
            auto_alias: Some(path.file_name().unwrap().to_string_lossy().to_string()),
            ..Default::default()
        },
        |_| {},
    )
    .unwrap();

    assert_eq!(progress.added_files, 18);
    assert_eq!(progress.skipped_files, 0);

    // Incremental rescan skips unchanged files
    let second = scan_folders(
        &idx,
        &req,
        &|_, _, _, _, _| ProbedMediaMetadata::default(),
        |_| {},
    )
    .unwrap();
    assert_eq!(second.skipped_files, 18);
    assert_eq!(second.added_files, 0);

    // Duplicate detection flags dup1 + dup2 without deleting
    let groups = idx.detect_and_flag_duplicates(Some("pc-1")).unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].items.len(), 2);

    // Verify BLAKE3 helper consistency
    let h1 = compute_file_blake3(&lib.join("dup1.jpg")).unwrap();
    let h2 = compute_file_blake3(&lib.join("dup2.jpg")).unwrap();
    assert_eq!(h1, h2);
    assert_eq!(
        compute_partial_blake3(&lib.join("dup1.jpg")).unwrap(),
        compute_partial_blake3(&lib.join("dup2.jpg")).unwrap()
    );

    // Cancellation stops scan immediately
    cancel.store(true, Ordering::Relaxed);
    let cancelled = scan_folders(
        &idx,
        &req,
        &|_, _, _, _, _| ProbedMediaMetadata::default(),
        |_| {},
    )
    .unwrap();
    assert!(!cancelled.in_progress);

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn cursor_paging_fts5_and_mediastore_batch_ingestion_work() {
    let idx = setup_index_with_self();
    idx.upsert_device(&DeviceRecord {
        id: "phone-1".to_string(),
        name: "Pixel 9".to_string(),
        device_type: DeviceType::Phone,
        public_key: "pk-phone".to_string(),
        last_seen: 1_700_000_000_000,
        transports: vec![TransportKind::WifiLan],
        host: None,
        port: None,
        is_self: false,
        is_online: true,
        paired_at: Some(1_700_000_000_000),
    })
    .unwrap();

    let rows: Vec<AndroidMediaStoreRow> = (0..5)
        .map(|i| AndroidMediaStoreRow {
            uri: format!("content://media/external/images/media/{i}"),
            display_name: format!("IMG_20260314_{i}.jpg"),
            relative_path: "DCIM/Camera/".to_string(),
            mime_type: "image/jpeg".to_string(),
            size: 100_000 + i as u64,
            date_added_sec: 1_773_489_600 + i as i64,
            date_modified_sec: 1_773_489_600 + i as i64,
            date_taken_ms: None,
            duration_ms: 0,
            width: 4000,
            height: 3000,
            latitude: Some(-8.7185),
            longitude: Some(115.1686),
            artist: None,
            album: None,
            title: None,
        })
        .collect();

    let inserted =
        ingest_android_mediastore_batch(&idx, "phone-1", "Pixel 9", &rows, &|r, _mt, _dev| {
            if r.display_name.contains("_2.") {
                (
                    "2026-03-14 Kuta Beach Bali".to_string(),
                    Some("Kuta Beach Bali".to_string()),
                )
            } else {
                ("2026-03-14 Home".to_string(), Some("Home".to_string()))
            }
        })
        .unwrap();
    assert_eq!(inserted, 5);

    // Paged query (2 per page)
    let page1 = idx
        .list_media(&MediaFilter::default(), SortSpec::default(), None, 2)
        .unwrap();
    assert_eq!(page1.total_count, 5);
    assert_eq!(page1.items.len(), 2);
    assert_eq!(page1.next_cursor.as_deref(), Some("2"));

    let page2 = idx
        .list_media(
            &MediaFilter::default(),
            SortSpec::default(),
            page1.next_cursor.as_deref(),
            2,
        )
        .unwrap();
    assert_eq!(page2.items.len(), 2);
    assert_eq!(page2.next_cursor.as_deref(), Some("4"));

    // FTS5 search "2026 beach"
    let beach_hits = idx
        .list_media(
            &MediaFilter {
                query: "2026 beach".to_string(),
                media_type: Some(MediaType::Image),
                ..Default::default()
            },
            SortSpec::default(),
            None,
            20,
        )
        .unwrap();
    assert_eq!(beach_hits.total_count, 1);
    assert_eq!(beach_hits.items[0].media.file_name, "IMG_20260314_2.jpg");
}

#[test]
fn android_media_store_uses_capture_date_when_available() {
    let idx = setup_index_with_self();
    idx.upsert_device(&DeviceRecord {
        id: "phone-1".into(),
        name: "Pixel 9".into(),
        device_type: DeviceType::Phone,
        public_key: "pk-phone".into(),
        last_seen: 1_700_000_000_000,
        transports: vec![TransportKind::WifiLan],
        host: None,
        port: None,
        is_self: false,
        is_online: true,
        paired_at: Some(1_700_000_000_000),
    })
    .unwrap();
    let captured_at_ms = 1_600_000_000_000_i64;
    let row: AndroidMediaStoreRow = serde_json::from_value(serde_json::json!({
        "uri": "content://media/external/images/media/42",
        "display_name": "IMG_20201001.jpg",
        "relative_path": "DCIM/Camera/",
        "mime_type": "image/jpeg",
        "size": 2048,
        "date_added_sec": 1_770_000_000_i64,
        "date_modified_sec": 1_770_000_001_i64,
        "date_taken_ms": captured_at_ms,
        "duration_ms": 0,
        "width": 1920,
        "height": 1080
    }))
    .unwrap();

    ingest_android_mediastore_batch(&idx, "phone-1", "Pixel 9", &[row], &|_, _, _| {
        ("2020-09-13 Home IMG_20201001.jpg".into(), None)
    })
    .unwrap();

    let media = idx
        .get_media_by_path("phone-1", "content://media/external/images/media/42")
        .unwrap()
        .unwrap();
    assert_eq!(media.created_at, captured_at_ms);
}

#[test]
fn android_media_store_falls_back_when_capture_date_is_unavailable() {
    let row: AndroidMediaStoreRow = serde_json::from_value(serde_json::json!({
        "uri": "content://media/external/images/media/43",
        "display_name": "IMG_20201001.jpg",
        "relative_path": "DCIM/Camera/",
        "mime_type": "image/jpeg",
        "size": 2048,
        "date_added_sec": 1_700_000_000_i64,
        "date_modified_sec": 1_700_000_001_i64,
        "date_taken_ms": 0,
        "duration_ms": 0,
        "width": 1920,
        "height": 1080
    }))
    .unwrap();

    assert_eq!(row.created_at_ms(), 1_700_000_001_000);

    let legacy_row: AndroidMediaStoreRow = serde_json::from_value(serde_json::json!({
        "uri": "content://media/external/images/media/44",
        "display_name": "IMG_20201002.jpg",
        "relative_path": "DCIM/Camera/",
        "mime_type": "image/jpeg",
        "size": 2048,
        "date_added_sec": 1_700_000_000_i64,
        "date_modified_sec": 1_700_000_001_i64,
        "duration_ms": 0,
        "width": 1920,
        "height": 1080
    }))
    .unwrap();

    assert_eq!(legacy_row.created_at_ms(), 1_700_000_001_000);
}

#[test]
fn imports_legacy_v01_sqlite_idempotently_and_requires_relogin() {
    let dir = temp_dir("mg-legacy-import");
    let legacy_db = dir.join("legacy.db");
    {
        let conn = rusqlite::Connection::open(&legacy_db).unwrap();
        conn.execute_batch(
            "CREATE TABLE device (id TEXT PRIMARY KEY, name TEXT, type TEXT, public_key TEXT, last_seen INTEGER, transports TEXT, host TEXT, port INTEGER, is_self INTEGER, is_online INTEGER, paired_at INTEGER);
             CREATE TABLE media (id TEXT PRIMARY KEY, device_id TEXT, path TEXT, folder_path TEXT, file_name TEXT, type TEXT, mime TEXT, size INTEGER, hash TEXT, partial_hash TEXT, created_at INTEGER, modified_at INTEGER, duration_ms INTEGER, width INTEGER, height INTEGER, gps_lat REAL, gps_lon REAL, place TEXT, tags TEXT, alias TEXT, auto_alias TEXT, cover_id TEXT, sync_state TEXT, is_duplicate INTEGER, duplicate_group_id TEXT, is_trashed INTEGER, trashed_path TEXT, updated_at INTEGER);
             CREATE TABLE cover (id TEXT PRIMARY KEY, media_id TEXT UNIQUE, source TEXT, file_path TEXT, cloud_id TEXT, aspect TEXT, crop TEXT, frame_time_ms INTEGER, updated_at INTEGER);
             CREATE TABLE pinned_folder (id TEXT PRIMARY KEY, device_id TEXT, path TEXT, label TEXT, icon TEXT, position INTEGER);
             CREATE TABLE rename_log (id TEXT PRIMARY KEY, media_id TEXT, batch_id TEXT, old_name TEXT, new_name TEXT, old_path TEXT, new_path TEXT, applied_at INTEGER, undone_at INTEGER);
             CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT);
             INSERT INTO device VALUES ('pc-leg', 'Old PC', 'pc', 'pk', 1000, '[\"wifi_lan\"]', '127.0.0.1', 9443, 1, 1, 1000);
             INSERT INTO media VALUES ('m-leg', 'pc-leg', 'C:/vid.mp4', 'C:/', 'vid.mp4', 'video', 'video/mp4', 999, 'sha256old', 'pold', 1773489600000, 1773489600000, 90000, 1920, 1080, NULL, NULL, 'Bali', '{}', 'Legacy Clip', 'Legacy Clip', 'c-leg', 'Local', 0, NULL, 0, NULL, 1000);
             INSERT INTO cover VALUES ('c-leg', 'm-leg', 'generated', 'C:/thumbs/m-leg.png', NULL, '16:9', NULL, 14500, 1000);
             INSERT INTO pinned_folder VALUES ('pin-1', 'pc-leg', 'C:/Camera', 'Camera', 'folder', 0);
             INSERT INTO rename_log VALUES ('rl-1', 'm-leg', NULL, 'VID_01.mp4', 'vid.mp4', 'C:/VID_01.mp4', 'C:/vid.mp4', 1000, NULL);
             INSERT INTO settings VALUES ('app_settings', '{\"sidebarWidth\":280,\"theme\":\"light\"}');",
        )
        .unwrap();
    }

    let target = MediaIndex::open_in_memory().unwrap();
    let r1 = import_legacy_sqlite(&target, &legacy_db).unwrap();
    assert_eq!(r1.imported_devices, 1);
    assert_eq!(r1.imported_media, 1);
    assert_eq!(r1.imported_covers, 1);
    assert_eq!(r1.imported_pinned_folders, 1);
    assert_eq!(r1.imported_rename_logs, 1);
    assert!(r1.auth_relogin_required);

    let migrated = target.get_media_by_id("m-leg").unwrap().unwrap();
    assert_eq!(migrated.media.alias, "Legacy Clip");
    assert_eq!(migrated.cover.unwrap().frame_time_ms, 14_500);
    assert_eq!(target.get_app_settings().unwrap().sidebar_width, 280);

    // Second run is idempotent
    let r2 = import_legacy_sqlite(&target, &legacy_db).unwrap();
    assert_eq!(r2.imported_media, 0);
    assert_eq!(r2.skipped_items, 1);

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn upsert_media_idempotency_after_scan_and_multi_device_isolation() {
    let root = temp_dir("mg-upsert-idempotent");
    let lib = root.join("videos");
    std::fs::create_dir_all(&lib).unwrap();
    let scanned_file = lib.join("a.mp4");
    std::fs::write(&scanned_file, b"initial-video-bytes").unwrap();
    let scanned_path = scanned_file.to_string_lossy().replace('\\', "/");
    let scanned_folder = lib.to_string_lossy().replace('\\', "/");

    let idx = setup_index_with_self();
    idx.upsert_device(&DeviceRecord {
        id: "pc-2".to_string(),
        name: "Backup PC".to_string(),
        device_type: DeviceType::Pc,
        public_key: "pk-pc-2".to_string(),
        last_seen: 1_700_000_000_000,
        transports: vec![TransportKind::WifiLan],
        host: None,
        port: None,
        is_self: false,
        is_online: true,
        paired_at: None,
    })
    .unwrap();

    // (a) Calling upsert_media twice with same (device_id, path) leaves 1 row,
    // preserves original id / user alias / cover_id, and updates mutable columns.
    let manual_path = format!("{scanned_folder}/manual.mp4");
    let rec1 = MediaRecord {
        id: "manual-id-1".into(),
        device_id: "pc-1".into(),
        path: manual_path.clone(),
        folder_path: scanned_folder.clone(),
        file_name: "manual.mp4".into(),
        media_type: MediaType::Video,
        mime: "video/mp4".into(),
        size: 100,
        blake3: "hash-v1".into(),
        partial_hash: "phash-v1".into(),
        created_at: 1_773_489_600_000,
        modified_at: 1_773_489_600_000,
        duration_ms: 0,
        width: 0,
        height: 0,
        gps_lat: None,
        gps_lon: None,
        place: None,
        tags: MediaTags::default(),
        alias: "Custom User Alias".into(),
        auto_alias: "2026-03-14 manual".into(),
        cover_id: None,
        sync_state: SyncState::Local,
        is_duplicate: false,
        duplicate_group_id: None,
        is_trashed: false,
        trashed_path: None,
        updated_at: 1_773_489_600_000,
    };
    idx.upsert_media(&rec1).unwrap();
    idx.upsert_cover(&CoverRecord {
        id: "cov-manual-1".into(),
        media_id: "manual-id-1".into(),
        source: CoverSource::Custom,
        file_path: "C:/thumbs/cov-manual-1.png".into(),
        cloud_id: None,
        aspect: AspectPreset::Ratio16x9,
        crop: None,
        frame_time_ms: 4_000,
        updated_at: 1_773_489_600_000,
    })
    .unwrap();

    let rec2 = MediaRecord {
        id: "manual-id-different".into(),
        size: 2048,
        blake3: "hash-v2".into(),
        partial_hash: "phash-v2".into(),
        modified_at: 1_773_489_700_000,
        duration_ms: 90_000,
        width: 1920,
        height: 1080,
        place: Some("Bali".into()),
        tags: MediaTags {
            title: Some("Updated Clip".into()),
            ..Default::default()
        },
        alias: "2026-03-14 auto regenerated".into(),
        auto_alias: "2026-03-14 auto regenerated".into(),
        cover_id: None,
        sync_state: SyncState::Synced,
        updated_at: 1_773_489_700_000,
        ..rec1.clone()
    };
    idx.upsert_media(&rec2).unwrap();

    let all_manual = idx
        .list_media(&MediaFilter::default(), SortSpec::default(), None, 50)
        .unwrap();
    assert_eq!(all_manual.total_count, 1);
    let stored = idx
        .get_media_by_path("pc-1", &manual_path)
        .unwrap()
        .unwrap();
    assert_eq!(stored.id, "manual-id-1");
    assert_eq!(stored.size, 2048);
    assert_eq!(stored.blake3, "hash-v2");
    assert_eq!(stored.duration_ms, 90_000);
    assert_eq!(stored.width, 1920);
    assert_eq!(stored.height, 1080);
    assert_eq!(stored.alias, "Custom User Alias");
    assert_eq!(stored.cover_id.as_deref(), Some("cov-manual-1"));
    assert_eq!(stored.sync_state, SyncState::Synced);

    // (b) Scan folder then call upsert_media on a scanned file with a different struct id
    scan_folders(
        &idx,
        &ScanRequest {
            operation_id: "op-upsert-parity".into(),
            device_id: "pc-1".into(),
            device_name: "Workstation".into(),
            roots: vec![lib],
            excluded_folders: Vec::new(),
            compute_hashes: true,
            batch_size: 50,
            cancel: None,
        },
        &|_, _, _, _, _| ProbedMediaMetadata::default(),
        |_| {},
    )
    .unwrap();

    let scanned_row = idx
        .get_media_by_path("pc-1", &scanned_path)
        .unwrap()
        .unwrap();
    idx.upsert_media(&MediaRecord {
        id: "vid-parity-override".into(),
        device_id: "pc-1".into(),
        path: scanned_path.clone(),
        folder_path: scanned_folder.clone(),
        file_name: "a.mp4".into(),
        media_type: MediaType::Video,
        mime: "video/mp4".into(),
        size: 99_999,
        blake3: "updated-b3".into(),
        partial_hash: String::new(),
        created_at: scanned_row.created_at,
        modified_at: scanned_row.modified_at + 1000,
        duration_ms: 45_000,
        width: 1280,
        height: 720,
        gps_lat: None,
        gps_lon: None,
        place: Some("Tokyo".into()),
        tags: MediaTags::default(),
        alias: "a.mp4".into(),
        auto_alias: "a.mp4".into(),
        cover_id: None,
        sync_state: SyncState::Local,
        is_duplicate: false,
        duplicate_group_id: None,
        is_trashed: false,
        trashed_path: None,
        updated_at: scanned_row.updated_at + 1000,
    })
    .unwrap();

    let after_scan_upsert = idx
        .get_media_by_path("pc-1", &scanned_path)
        .unwrap()
        .unwrap();
    assert_eq!(after_scan_upsert.id, scanned_row.id);
    assert_eq!(after_scan_upsert.size, 99_999);
    assert_eq!(after_scan_upsert.duration_ms, 45_000);

    // (c) Same path on two different device_ids creates two distinct rows
    idx.upsert_media(&MediaRecord {
        device_id: "pc-2".into(),
        ..rec1
    })
    .unwrap();

    let across_devices = idx
        .list_media(
            &MediaFilter {
                folder_path: Some(scanned_folder),
                ..Default::default()
            },
            SortSpec::default(),
            None,
            50,
        )
        .unwrap();
    assert_eq!(across_devices.total_count, 3);
    assert!(idx
        .get_media_by_path("pc-1", &manual_path)
        .unwrap()
        .is_some());
    assert!(idx
        .get_media_by_path("pc-2", &manual_path)
        .unwrap()
        .is_some());

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn recent_date_and_size_sorting_orders_correctly() {
    let idx = setup_index_with_self();
    let make = |id: &str, size: u64, created_at: i64, modified_at: i64| MediaRecord {
        id: id.into(),
        device_id: "pc-1".into(),
        path: format!("C:/Media/{id}.mp4"),
        folder_path: "C:/Media".into(),
        file_name: format!("{id}.mp4"),
        media_type: MediaType::Video,
        mime: "video/mp4".into(),
        size,
        blake3: id.into(),
        partial_hash: id.into(),
        created_at,
        modified_at,
        duration_ms: 1000,
        width: 1920,
        height: 1080,
        gps_lat: None,
        gps_lon: None,
        place: None,
        tags: MediaTags::default(),
        alias: id.into(),
        auto_alias: id.into(),
        cover_id: None,
        sync_state: SyncState::Local,
        is_duplicate: false,
        duplicate_group_id: None,
        is_trashed: false,
        trashed_path: None,
        updated_at: modified_at,
    };

    idx.upsert_media_batch(&[
        make("old", 300, 1_000_000, 1_000_000),
        make("redownloaded", 100, 1_100_000, 5_000_000),
        make("mid", 200, 2_000_000, 2_000_000),
    ])
    .unwrap();

    let recent_desc = idx
        .list_media(
            &MediaFilter::default(),
            SortSpec {
                field: core_index::SortField::Date,
                dir: core_index::SortDir::Desc,
            },
            None,
            10,
        )
        .unwrap();
    let recent_ids: Vec<&str> = recent_desc
        .items
        .iter()
        .map(|i| i.media.id.as_str())
        .collect();
    assert_eq!(recent_ids, vec!["redownloaded", "mid", "old"]);

    let size_desc = idx
        .list_media(
            &MediaFilter::default(),
            SortSpec {
                field: core_index::SortField::Size,
                dir: core_index::SortDir::Desc,
            },
            None,
            10,
        )
        .unwrap();
    let size_ids: Vec<&str> = size_desc
        .items
        .iter()
        .map(|i| i.media.id.as_str())
        .collect();
    assert_eq!(size_ids, vec!["old", "mid", "redownloaded"]);
}
