use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use anyhow::{ensure, Context, Result};
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

fn create_temp_root(prefix: &str) -> Result<PathBuf> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("read system clock while creating parity temp directory")?
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("{prefix}-{nanos}"));
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("create parity root directory at {}", dir.display()))?;
    Ok(dir)
}

fn default_self_device(id: &str, name: &str, last_seen: i64) -> DeviceRecord {
    DeviceRecord {
        id: id.into(),
        name: name.into(),
        device_type: DeviceType::Pc,
        public_key: format!("pk-{id}"),
        last_seen,
        transports: vec![TransportKind::WifiLan],
        host: Some("127.0.0.1".into()),
        port: Some(9443),
        is_self: true,
        is_online: true,
        paired_at: None,
    }
}

fn run_parity_and_benchmarks(root: &Path) -> Result<ParityBenchmarkReport> {
    let parity_dir = root.join("parity_fixtures");
    let thumb_dir = root.join("thumb_parity");
    let bench_1k_dir = root.join("bench_1k");
    let cache_dir = root.join("thumbs");
    for dir in [&parity_dir, &thumb_dir, &bench_1k_dir, &cache_dir] {
        std::fs::create_dir_all(dir)
            .with_context(|| format!("create fixture directory {}", dir.display()))?;
    }

    // 1. Parity fixture checks (16 supported extensions + duplicates)
    let exts = [
        "a.mp4", "b.mkv", "c.webm", "d.mov", "e.avi", "f.mp3", "g.m4a", "h.flac", "i.wav", "j.ogg",
        "k.png", "l.jpg", "m.jpeg", "n.gif", "o.webp", "p.heic",
    ];
    for name in exts {
        let p = parity_dir.join(name);
        std::fs::write(&p, format!("parity-payload-{name}"))
            .with_context(|| format!("write parity fixture {}", p.display()))?;
    }
    let dup_bytes = vec![77u8; 4096];
    for dup_name in ["dup_a.jpg", "dup_b.jpg"] {
        let p = parity_dir.join(dup_name);
        std::fs::write(&p, &dup_bytes)
            .with_context(|| format!("write duplicate fixture {}", p.display()))?;
    }

    let scan_idx = MediaIndex::open_in_memory().context("open scan parity SQLite index")?;
    scan_idx
        .upsert_device(&default_self_device(
            "pc-1",
            "Workstation",
            1_773_489_600_000,
        ))
        .context("upsert pc-1 device for scan parity")?;

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
        &scan_idx,
        &scan_req,
        &|p, mt, c, m, dev| {
            extract_metadata_and_alias(p, mt, c, m, dev, None, Some(sidecar.as_ref()))
        },
        |_| {},
    )
    .with_context(|| format!("scan parity fixtures in {}", parity_dir.display()))?;
    ensure!(
        res.added_files == 18,
        "expected 18 scanned parity files in {}, got {}",
        parity_dir.display(),
        res.added_files
    );
    let dup_groups = scan_idx
        .detect_and_flag_duplicates(Some("pc-1"))
        .context("detect duplicate fixture group on pc-1")?;
    ensure!(
        dup_groups.len() == 1,
        "expected 1 duplicate group on pc-1, got {}",
        dup_groups.len()
    );

    // 2. Auto-alias parity checks
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
    ensure!(
        place_bali.as_deref() == Some("Kuta Beach Bali"),
        "expected Kuta Beach Bali geocoding match, got {place_bali:?}"
    );
    ensure!(
        auto_alias_bali_photo == "2026-03-14 Kuta Beach Bali",
        "unexpected Bali auto-alias: {auto_alias_bali_photo}"
    );

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
    ensure!(
        auto_alias_camel_video == "2026-03-14 sunset Walk",
        "unexpected video auto-alias: {auto_alias_camel_video}"
    );
    ensure!(
        clean_file_name_for_alias("VID_0042_sunsetWalk-09.mp4") == "sunset Walk",
        "unexpected clean_file_name_for_alias result"
    );

    // 3. Thumbnail first-third parity (isolated DB & fixture path, independent of scan_folders)
    let thumb_idx = MediaIndex::open_in_memory().context("open thumbnail parity SQLite index")?;
    thumb_idx
        .upsert_device(&default_self_device("pc-thumb", "Workstation", march_2026))
        .context("upsert pc-thumb device for thumbnail parity")?;

    let sample_video_file = thumb_dir.join("sample_video.mp4");
    std::fs::write(&sample_video_file, b"sample-video-parity-payload").with_context(|| {
        format!(
            "write thumbnail parity fixture {}",
            sample_video_file.display()
        )
    })?;
    let sample_video_path = sample_video_file.to_string_lossy().replace('\\', "/");
    let sample_video = MediaRecord {
        id: "vid-parity".into(),
        device_id: "pc-thumb".into(),
        path: sample_video_path.clone(),
        folder_path: thumb_dir.to_string_lossy().replace('\\', "/"),
        file_name: "sample_video.mp4".into(),
        media_type: MediaType::Video,
        mime: "video/mp4".into(),
        size: 27,
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
    thumb_idx
        .upsert_media(&sample_video)
        .with_context(|| format!("upsert synthetic sample video at {sample_video_path}"))?;
    let thumb_mgr = ThumbnailManager {
        index: &thumb_idx,
        sidecar: sidecar.as_ref(),
        cache_dir: &cache_dir,
        max_cache_bytes: None,
    };
    let t_thumb0 = Instant::now();
    let cover = thumb_mgr
        .ensure_thumbnail(&sample_video, Some(0.5))
        .with_context(|| {
            format!(
                "generate first-third thumbnail for {} into {}",
                sample_video_file.display(),
                cache_dir.display()
            )
        })?;
    let first_thumb_ms = t_thumb0.elapsed().as_secs_f64() * 1000.0;
    ensure!(
        cover.frame_time_ms <= 30_000,
        "frame_time_ms {} exceeded first third (30_000 ms) for {}",
        cover.frame_time_ms,
        sample_video_file.display()
    );

    // 4. BLAKE3 transfer check
    let src_xfer = root.join("src.bin");
    let xfer_bytes = vec![55u8; 64 * 1024];
    std::fs::write(&src_xfer, &xfer_bytes)
        .with_context(|| format!("write transfer fixture {}", src_xfer.display()))?;
    let expected_b3 = compute_file_blake3(&src_xfer)
        .with_context(|| format!("compute BLAKE3 digest for {}", src_xfer.display()))?;
    let rx_dir = root.join("rx");
    let rx = TransferReceiver::new(rx_dir.clone());
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
            data: xfer_bytes,
        })
        .with_context(|| format!("verify BLAKE3 transfer chunk into {}", rx_dir.display()))?;
    ensure!(
        final_rx.is_some(),
        "expected TransferReceiver completion path for tx-parity"
    );

    // 5. Legacy read-only SQLite import check (isolated DB)
    let leg_db = root.join("legacy.db");
    {
        let conn = rusqlite::Connection::open(&leg_db)
            .with_context(|| format!("create legacy fixture SQLite db {}", leg_db.display()))?;
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
        .with_context(|| format!("seed legacy fixture tables in {}", leg_db.display()))?;
    }
    let import_idx = MediaIndex::open_in_memory().context("open isolated legacy import index")?;
    let import_report = import_legacy_sqlite(&import_idx, &leg_db)
        .with_context(|| format!("import legacy SQLite db from {}", leg_db.display()))?;
    ensure!(
        import_report.imported_media == 1,
        "expected 1 imported legacy media row from {}, got {}",
        leg_db.display(),
        import_report.imported_media
    );

    // 6. Benchmark: 1,000 real fixture files filesystem scan (matches legacy benchmark)
    for i in 0..1000 {
        let name = if i == 500 {
            "IMG_2031_beach.jpg".to_string()
        } else {
            format!("IMG_{i:04}.jpg")
        };
        let p = bench_1k_dir.join(name);
        std::fs::write(&p, b"stub-bytes")
            .with_context(|| format!("write 1k benchmark file {}", p.display()))?;
    }
    let bench_idx = MediaIndex::open_in_memory().context("open 1k scan benchmark SQLite index")?;
    bench_idx
        .upsert_device(&default_self_device("pc-1", "Workstation", march_2026))
        .context("upsert pc-1 device for 1k scan benchmark")?;
    let t_scan0 = Instant::now();
    scan_folders(
        &bench_idx,
        &ScanRequest {
            operation_id: "bench-1k".into(),
            device_id: "pc-1".into(),
            device_name: "Workstation".into(),
            roots: vec![bench_1k_dir.clone()],
            excluded_folders: Vec::new(),
            compute_hashes: true,
            batch_size: 200,
            cancel: None,
        },
        &|_p, _mt, _c, _m, _d| ProbedMediaMetadata::default(),
        |_| {},
    )
    .with_context(|| format!("run 1k file scan benchmark in {}", bench_1k_dir.display()))?;
    let scan_1k_files_ms = t_scan0.elapsed().as_secs_f64() * 1000.0;

    // 7. Benchmark: 10,000 and 100,000 mixed media batch SQLite + FTS5 indexing (PRD Sec 15 & 18)
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

    let idx_10k = MediaIndex::open_in_memory().context("open 10k batch benchmark SQLite index")?;
    idx_10k
        .upsert_device(&default_self_device("pc-1", "PC", march_2026))
        .context("upsert pc-1 device for 10k batch benchmark")?;
    let batch_10k = make_batch(10_000, "b10k");
    let t_10k = Instant::now();
    idx_10k
        .upsert_media_batch(&batch_10k)
        .context("upsert 10k media benchmark batch")?;
    let insert_10k_ms = t_10k.elapsed().as_secs_f64() * 1000.0;

    let idx_100k =
        MediaIndex::open_in_memory().context("open 100k batch benchmark SQLite index")?;
    idx_100k
        .upsert_device(&default_self_device("pc-1", "PC", march_2026))
        .context("upsert pc-1 device for 100k batch benchmark")?;
    let batch_100k = make_batch(100_000, "b100k");
    let t_100k = Instant::now();
    idx_100k
        .upsert_media_batch(&batch_100k)
        .context("upsert 100k media benchmark batch")?;
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
        .context("execute FTS5 '2026 beach' query against 100k media index")?;
    let fts_query_100k_ms = t_q.elapsed().as_secs_f64() * 1000.0;
    ensure!(
        hits.total_count == 1,
        "expected 1 FTS5 hit for '2026 beach' across 100k records, got {}",
        hits.total_count
    );

    Ok(ParityBenchmarkReport {
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
    })
}

fn main() -> Result<()> {
    let root = create_temp_root("mg-parity-bench")?;
    let result = run_parity_and_benchmarks(&root);
    let _ = std::fs::remove_dir_all(&root);
    let report = result?;
    let json =
        serde_json::to_string_pretty(&report).context("serialize ParityBenchmarkReport to JSON")?;
    println!("{json}");
    Ok(())
}
