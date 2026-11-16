use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use core_index::{
    compute_file_blake3, compute_partial_blake3, import_legacy_sqlite,
    ingest_android_mediastore_batch, scan_folders, DeviceRecord, DeviceType, MediaFilter,
    MediaIndex, MediaType, ProbedMediaMetadata, ScanRequest, SortSpec, TransportKind,
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
        "a.mp4", "b.mkv", "c.webm", "d.mov", "e.avi",
        "f.mp3", "g.m4a", "h.flac", "i.wav", "j.ogg",
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

    let inserted = ingest_android_mediastore_batch(
        &idx,
        "phone-1",
        "Pixel 9",
        &rows,
        &|r, _mt, _dev| {
            if r.display_name.contains("_2.") {
                ("2026-03-14 Kuta Beach Bali".to_string(), Some("Kuta Beach Bali".to_string()))
            } else {
                ("2026-03-14 Home".to_string(), Some("Home".to_string()))
            }
        },
    )
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
