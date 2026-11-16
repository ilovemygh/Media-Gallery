use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use core_index::{
    compute_file_blake3, import_legacy_sqlite, scan_folders, DeviceRecord, DeviceType, MediaFilter,
    MediaIndex, MediaRecord, MediaTags, MediaType, ProbedMediaMetadata, ScanRequest, SortSpec,
    SyncState, TransportKind,
};
use core_media::{
    clean_file_name_for_alias, extract_metadata_and_alias, generate_auto_alias, AutoAliasInput,
    ThumbnailManager,
};
use core_net::{FileChunkFrame, TransferReceiver};
use core_platform::DesktopFfmpegSidecar;
use serde::Serialize;

#[derive(Serialize)]
struct ParityBenchmarkReport {
    parity_checks_passed: bool,
    supported_extensions_count: usize,
    duplicate_groups_found: usize,
    auto_alias_bali_photo: String,
    auto_alias_camel_video: String,
    fts_beach_hits: u64,
    first_third_frame_ms: u64,
    blake3_transfer_verified: bool,
    legacy_import_media_count: u64,
    scan_1k_files_ms: f64,
    insert_10k_ms: f64,
    insert_100k_ms: f64,
    fts_query_100k_ms: f64,
    first_thumb_ms: f64,
}

fn temp_dir(prefix: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("{prefix}-{nanos}"));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn main() {
    let root = temp_dir("mg-parity-bench");
    let parity_dir = root.join("parity_fixtures");
    let bench_1k_dir = root.join("bench_1k");
    let cache_dir = root.join("thumbs");
    std::fs::create_dir_all(&parity_dir).unwrap();
    std::fs::create_dir_all(&bench_1k_dir).unwrap();

    // 1. Parity fixture checks (16 supported extensions + duplicates)
    let exts = [
        "a.mp4", "b.mkv", "c.webm", "d.mov", "e.avi", "f.mp3", "g.m4a", "h.flac", "i.wav", "j.ogg",
        "k.png", "l.jpg", "m.jpeg", "n.gif", "o.webp", "p.heic",
    ];
    for name in exts {
        std::fs::write(parity_dir.join(name), format!("parity-payload-{name}")).unwrap();
    }
    let dup_bytes = vec![77u8; 4096];
    std::fs::write(parity_dir.join("dup_a.jpg"), &dup_bytes).unwrap();
    std::fs::write(parity_dir.join("dup_b.jpg"), &dup_bytes).unwrap();

    let idx = MediaIndex::open_in_memory().unwrap();
    idx.upsert_device(&DeviceRecord {
        id: "pc-1".into(),
        name: "Workstation".into(),
        device_type: DeviceType::Pc,
        public_key: "pk-1".into(),
        last_seen: 1_773_489_600_000,
        transports: vec![TransportKind::WifiLan],
        host: Some("127.0.0.1".into()),
        port: Some(9443),
        is_self: true,
        is_online: true,
        paired_at: None,
    })
    .unwrap();

    let sidecar = Arc::new(DesktopFfmpegSidecar::default());
    let scan_req = ScanRequest {
        operation_id: "parity-scan".into(),
        device_id: "pc-1".into(),
        device_name: "Workstation".into(),
        roots: vec![parity_dir.clone()],
        excluded_folders: Vec::new(),
        compute_hashes: true,
        batch_size: 200,
        cancel: None,
    };
    let res = scan_folders(
        &idx,
        &scan_req,
        &|p, mt, c, m, dev| {
            extract_metadata_and_alias(p, mt, c, m, dev, None, Some(sidecar.as_ref()))
        },
        |_| {},
    )
    .unwrap();
    assert_eq!(res.added_files, 18);
    let dup_groups = idx.detect_and_flag_duplicates(Some("pc-1")).unwrap();
    assert_eq!(dup_groups.len(), 1);

    // Auto-alias parity checks
    let march_2026 = 1_773_489_600_000_i64;
    let empty_tags = MediaTags::default();
    let (auto_alias_bali_photo, place_bali) = generate_auto_alias(&AutoAliasInput {
        file_name: "IMG_2031.jpg",
        media_type: MediaType::Image,
        created_at: march_2026,
        modified_at: march_2026,
        gps_lat: Some(-8.7185),
        gps_lon: Some(115.1686),
        place: None,
        tags: &empty_tags,
        device_name: "Workstation",
        template: None,
        use_utc_date: true,
    });
    assert_eq!(place_bali.as_deref(), Some("Kuta Beach Bali"));
    assert_eq!(auto_alias_bali_photo, "2026-03-14 Kuta Beach Bali");

    let (auto_alias_camel_video, _) = generate_auto_alias(&AutoAliasInput {
        file_name: "VID_0042_sunsetWalk-09.mp4",
        media_type: MediaType::Video,
        created_at: march_2026,
        modified_at: march_2026,
        gps_lat: None,
        gps_lon: None,
        place: None,
        tags: &empty_tags,
        device_name: "Workstation",
        template: None,
        use_utc_date: true,
    });
    assert_eq!(auto_alias_camel_video, "2026-03-14 sunset Walk");
    assert_eq!(
        clean_file_name_for_alias("VID_0042_sunsetWalk-09.mp4"),
        "sunset Walk"
    );

    // Thumbnail first-third parity
    let sample_video = MediaRecord {
        id: "vid-parity".into(),
        device_id: "pc-1".into(),
        path: parity_dir.join("a.mp4").to_string_lossy().to_string(),
        folder_path: parity_dir.to_string_lossy().to_string(),
        file_name: "a.mp4".into(),
        media_type: MediaType::Video,
        mime: "video/mp4".into(),
        size: 32,
        blake3: String::new(),
        partial_hash: String::new(),
        created_at: march_2026,
        modified_at: march_2026,
        duration_ms: 90_000,
        width: 1920,
        height: 1080,
        gps_lat: None,
        gps_lon: None,
        place: Some("Kuta Beach Bali".into()),
        tags: MediaTags::default(),
        alias: auto_alias_bali_photo.clone(),
        auto_alias: auto_alias_bali_photo.clone(),
        cover_id: None,
        sync_state: SyncState::Local,
        is_duplicate: false,
        duplicate_group_id: None,
        is_trashed: false,
        trashed_path: None,
        updated_at: march_2026,
    };
    idx.upsert_media(&sample_video).unwrap();
    let thumb_mgr = ThumbnailManager {
        index: &idx,
        sidecar: sidecar.as_ref(),
        cache_dir: &cache_dir,
        max_cache_bytes: None,
    };
    let t_thumb0 = Instant::now();
    let cover = thumb_mgr
        .ensure_thumbnail(&sample_video, Some(0.5))
        .unwrap();
    let first_thumb_ms = t_thumb0.elapsed().as_secs_f64() * 1000.0;
    assert!(cover.frame_time_ms <= 30_000);

    // BLAKE3 transfer check
    let src_xfer = root.join("src.bin");
    let xfer_bytes = vec![55u8; 64 * 1024];
    std::fs::write(&src_xfer, &xfer_bytes).unwrap();
    let expected_b3 = compute_file_blake3(&src_xfer).unwrap();
    let rx = TransferReceiver::new(root.join("rx"));
    let final_rx = rx
        .handle_chunk(&FileChunkFrame {
            transfer_id: "tx-parity".into(),
            media_id: "m-x".into(),
            file_name: "src.bin".into(),
            chunk_index: 0,
            total_chunks: 1,
            offset: 0,
            bytes_total: xfer_bytes.len() as u64,
            expected_blake3: expected_b3,
            data: xfer_bytes.clone(),
        })
        .unwrap();
    assert!(final_rx.is_some());

    // Legacy read-only SQLite import check
    let leg_db = root.join("legacy.db");
    {
        let conn = rusqlite::Connection::open(&leg_db).unwrap();
        conn.execute_batch(
            "CREATE TABLE device (id TEXT PRIMARY KEY, name TEXT, type TEXT, public_key TEXT, last_seen INTEGER, transports TEXT, host TEXT, port INTEGER, is_self INTEGER, is_online INTEGER, paired_at INTEGER);
             CREATE TABLE media (id TEXT PRIMARY KEY, device_id TEXT, path TEXT, folder_path TEXT, file_name TEXT, type TEXT, mime TEXT, size INTEGER, hash TEXT, partial_hash TEXT, created_at INTEGER, modified_at INTEGER, duration_ms INTEGER, width INTEGER, height INTEGER, gps_lat REAL, gps_lon REAL, place TEXT, tags TEXT, alias TEXT, auto_alias TEXT, cover_id TEXT, sync_state TEXT, is_duplicate INTEGER, duplicate_group_id TEXT, is_trashed INTEGER, trashed_path TEXT, updated_at INTEGER);
             CREATE TABLE cover (id TEXT PRIMARY KEY, media_id TEXT UNIQUE, source TEXT, file_path TEXT, cloud_id TEXT, aspect TEXT, crop TEXT, frame_time_ms INTEGER, updated_at INTEGER);
             CREATE TABLE pinned_folder (id TEXT PRIMARY KEY, device_id TEXT, path TEXT, label TEXT, icon TEXT, position INTEGER);
             CREATE TABLE rename_log (id TEXT PRIMARY KEY, media_id TEXT, batch_id TEXT, old_name TEXT, new_name TEXT, old_path TEXT, new_path TEXT, applied_at INTEGER, undone_at INTEGER);
             CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT);
             INSERT INTO device VALUES ('leg-pc', 'Legacy', 'pc', 'pk', 100, '[\"wifi_lan\"]', NULL, NULL, 1, 1, 100);
             INSERT INTO media VALUES ('leg-m1', 'leg-pc', 'C:/x.mp4', 'C:/', 'x.mp4', 'video', 'video/mp4', 100, 'sha', 'p', 1773489600000, 1773489600000, 30000, 1920, 1080, NULL, NULL, NULL, '{}', 'Legacy Video', 'Legacy Video', NULL, 'Local', 0, NULL, 0, NULL, 100);",
        )
        .unwrap();
    }
    let import_report = import_legacy_sqlite(&idx, &leg_db).unwrap();
    assert_eq!(import_report.imported_media, 1);

    // 2. Benchmark: 1,000 real fixture files filesystem scan (matches legacy benchmark)
    for i in 0..1000 {
        let name = if i == 500 {
            "IMG_2031_beach.jpg".to_string()
        } else {
            format!("IMG_{i:04}.jpg")
        };
        std::fs::write(bench_1k_dir.join(name), b"stub-bytes").unwrap();
    }
    let bench_idx = MediaIndex::open_in_memory().unwrap();
    bench_idx
        .upsert_device(&DeviceRecord {
            id: "pc-1".into(),
            name: "Workstation".into(),
            device_type: DeviceType::Pc,
            public_key: "pk".into(),
            last_seen: march_2026,
            transports: vec![TransportKind::WifiLan],
            host: None,
            port: None,
            is_self: true,
            is_online: true,
            paired_at: None,
        })
        .unwrap();
    let t_scan0 = Instant::now();
    scan_folders(
        &bench_idx,
        &ScanRequest {
            operation_id: "bench-1k".into(),
            device_id: "pc-1".into(),
            device_name: "Workstation".into(),
            roots: vec![bench_1k_dir],
            excluded_folders: Vec::new(),
            compute_hashes: true,
            batch_size: 200,
            cancel: None,
        },
        &|_p, _mt, _c, _m, _d| ProbedMediaMetadata::default(),
        |_| {},
    )
    .unwrap();
    let scan_1k_files_ms = t_scan0.elapsed().as_secs_f64() * 1000.0;

    // 3. Benchmark: 10,000 and 100,000 mixed media batch SQLite + FTS5 indexing (PRD Sec 15 & 18)
    let make_batch = |count: usize, prefix: &str| -> Vec<MediaRecord> {
        (0..count)
            .map(|i| {
                let is_target = i == count / 2;
                let place = if is_target { "Kuta Beach Bali" } else { "Home" };
                MediaRecord {
                    id: format!("{prefix}-{i}"),
                    device_id: "pc-1".into(),
                    path: format!("C:/Media/{}/{i}.mp4", i / 1000),
                    folder_path: format!("C:/Media/{}", i / 1000),
                    file_name: format!("VID_{i:06}.mp4"),
                    media_type: MediaType::Video,
                    mime: "video/mp4".into(),
                    size: 1024 + i as u64,
                    blake3: format!("b3-{prefix}-{i}"),
                    partial_hash: format!("pb3-{prefix}-{i}"),
                    created_at: march_2026 + (i as i64) * 1000,
                    modified_at: march_2026 + (i as i64) * 1000,
                    duration_ms: 60_000,
                    width: 1920,
                    height: 1080,
                    gps_lat: None,
                    gps_lon: None,
                    place: Some(place.into()),
                    tags: MediaTags::default(),
                    alias: if is_target {
                        "2026-03-14 Kuta Beach Bali".into()
                    } else {
                        format!("2026-03-14 Home {i}")
                    },
                    auto_alias: format!("2026-03-14 Home {i}"),
                    cover_id: None,
                    sync_state: SyncState::Local,
                    is_duplicate: false,
                    duplicate_group_id: None,
                    is_trashed: false,
                    trashed_path: None,
                    updated_at: march_2026,
                }
            })
            .collect()
    };

    let idx_10k = MediaIndex::open_in_memory().unwrap();
    idx_10k
        .upsert_device(&DeviceRecord {
            id: "pc-1".into(),
            name: "PC".into(),
            device_type: DeviceType::Pc,
            public_key: "pk".into(),
            last_seen: march_2026,
            transports: vec![TransportKind::WifiLan],
            host: None,
            port: None,
            is_self: true,
            is_online: true,
            paired_at: None,
        })
        .unwrap();
    let batch_10k = make_batch(10_000, "b10k");
    let t_10k = Instant::now();
    idx_10k.upsert_media_batch(&batch_10k).unwrap();
    let insert_10k_ms = t_10k.elapsed().as_secs_f64() * 1000.0;

    let idx_100k = MediaIndex::open_in_memory().unwrap();
    idx_100k
        .upsert_device(&DeviceRecord {
            id: "pc-1".into(),
            name: "PC".into(),
            device_type: DeviceType::Pc,
            public_key: "pk".into(),
            last_seen: march_2026,
            transports: vec![TransportKind::WifiLan],
            host: None,
            port: None,
            is_self: true,
            is_online: true,
            paired_at: None,
        })
        .unwrap();
    let batch_100k = make_batch(100_000, "b100k");
    let t_100k = Instant::now();
    idx_100k.upsert_media_batch(&batch_100k).unwrap();
    let insert_100k_ms = t_100k.elapsed().as_secs_f64() * 1000.0;

    let t_q = Instant::now();
    let hits = idx_100k
        .list_media(
            &MediaFilter {
                query: "2026 beach".into(),
                ..Default::default()
            },
            SortSpec::default(),
            None,
            50,
        )
        .unwrap();
    let fts_query_100k_ms = t_q.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(hits.total_count, 1);

    let report = ParityBenchmarkReport {
        parity_checks_passed: true,
        supported_extensions_count: 16,
        duplicate_groups_found: dup_groups.len(),
        auto_alias_bali_photo,
        auto_alias_camel_video,
        fts_beach_hits: hits.total_count,
        first_third_frame_ms: cover.frame_time_ms,
        blake3_transfer_verified: true,
        legacy_import_media_count: import_report.imported_media,
        scan_1k_files_ms: (scan_1k_files_ms * 100.0).round() / 100.0,
        insert_10k_ms: (insert_10k_ms * 100.0).round() / 100.0,
        insert_100k_ms: (insert_100k_ms * 100.0).round() / 100.0,
        fts_query_100k_ms: (fts_query_100k_ms * 100.0).round() / 100.0,
        first_thumb_ms: (first_thumb_ms * 100.0).round() / 100.0,
    };

    println!("{}", serde_json::to_string_pretty(&report).unwrap());
    let _ = std::fs::remove_dir_all(root);
}
