use std::time::{SystemTime, UNIX_EPOCH};

use core_index::{
    DeviceRecord, DeviceType, MediaFilter, MediaIndex, MediaRecord, MediaTags, MediaType,
    SortSpec, SyncState, TransportKind,
};
use core_media::{
    apply_bulk_rename, apply_real_rename, clean_file_name_for_alias, compute_crop_for_aspect,
    generate_auto_alias, lookup_place_offline, move_media_to_folder, move_to_trash,
    preview_bulk_rename, remove_audio_cover, replace_audio_cover, restore_audio_cover,
    select_random_first_third_frame_ms, set_alias, undo_bulk_rename, undo_last_rename,
    AutoAliasInput, MediaOpError, ThumbnailManager,
};
use core_platform::{DesktopFfmpegSidecar, ManagedTrashBackend};

fn temp_dir(prefix: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("{prefix}-{nanos}"));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn setup_index(db_path: Option<&std::path::Path>) -> MediaIndex {
    let idx = match db_path {
        Some(p) => MediaIndex::open(p).unwrap(),
        None => MediaIndex::open_in_memory().unwrap(),
    };
    idx.upsert_device(&DeviceRecord {
        id: "pc-1".to_string(),
        name: "Workstation".to_string(),
        device_type: DeviceType::Pc,
        public_key: "pk".to_string(),
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
fn auto_alias_and_offline_reverse_geocoding_match_prd_rules() {
    assert_eq!(clean_file_name_for_alias("IMG_2031.jpg"), "");
    assert_eq!(
        clean_file_name_for_alias("VID_0042_sunsetWalk-09.mp4"),
        "sunset Walk"
    );
    assert_eq!(
        clean_file_name_for_alias("Screenshot_myHolidayTrip_2026.png"),
        "my Holiday Trip"
    );

    // Offline GPS lookup resolves coastal beach landmark + GeoNames cities
    assert_eq!(
        lookup_place_offline(Some(-8.7185), Some(115.1686)).as_deref(),
        Some("Kuta Beach Bali")
    );
    assert!(lookup_place_offline(Some(35.6762), Some(139.6503)).is_some());

    let march_2026 = 1_773_489_600_000_i64; // 2026-03-14T12:00:00Z
    let tags = MediaTags::default();
    let (alias, place) = generate_auto_alias(&AutoAliasInput {
        file_name: "IMG_2031.jpg",
        media_type: MediaType::Image,
        created_at: march_2026,
        modified_at: march_2026,
        gps_lat: Some(-8.7185),
        gps_lon: Some(115.1686),
        place: None,
        tags: &tags,
        device_name: "Workstation",
        template: None,
        use_utc_date: true,
    });
    assert_eq!(place.as_deref(), Some("Kuta Beach Bali"));
    assert_eq!(alias, "2026-03-14 Kuta Beach Bali");

    let audio_tags = MediaTags {
        artist: Some("Nova Drift".into()),
        title: Some("Midnight Echoes".into()),
        ..Default::default()
    };
    let (audio_alias, _) = generate_auto_alias(&AutoAliasInput {
        file_name: "track_09.mp3",
        media_type: MediaType::Audio,
        created_at: march_2026,
        modified_at: march_2026,
        gps_lat: None,
        gps_lon: None,
        place: None,
        tags: &audio_tags,
        device_name: "Workstation",
        template: None,
        use_utc_date: true,
    });
    assert_eq!(audio_alias, "Nova Drift - Midnight Echoes");
}

#[test]
fn alias_disk_rename_undo_bulk_rename_folder_move_and_safe_trash_work() {
    let dir = temp_dir("mg-media-ren");
    let lib = dir.join("photos");
    std::fs::create_dir_all(&lib).unwrap();
    let f1 = lib.join("IMG_01.jpg");
    let f2 = lib.join("IMG_02.jpg");
    std::fs::write(&f1, b"photo1").unwrap();
    std::fs::write(&f2, b"photo2").unwrap();

    let idx = setup_index(None);
    for (id, file, name) in [("m1", &f1, "IMG_01.jpg"), ("m2", &f2, "IMG_02.jpg")] {
        idx.upsert_media(&MediaRecord {
            id: id.into(),
            device_id: "pc-1".into(),
            path: file.to_string_lossy().replace('\\', "/"),
            folder_path: lib.to_string_lossy().replace('\\', "/"),
            file_name: name.into(),
            media_type: MediaType::Image,
            mime: "image/jpeg".into(),
            size: 6,
            blake3: String::new(),
            partial_hash: String::new(),
            created_at: 1_773_489_600_000,
            modified_at: 1_773_489_600_000,
            duration_ms: 0,
            width: 100,
            height: 100,
            gps_lat: None,
            gps_lon: None,
            place: Some("Bali".into()),
            tags: MediaTags::default(),
            alias: "2026-03-14 Bali".into(),
            auto_alias: "2026-03-14 Bali".into(),
            cover_id: None,
            sync_state: SyncState::Local,
            is_duplicate: false,
            duplicate_group_id: None,
            is_trashed: false,
            trashed_path: None,
            updated_at: 1_773_489_600_000,
        })
        .unwrap();
    }

    // Non-destructive alias
    let aliased = set_alias(&idx, "m1", "Sunset Shot").unwrap();
    assert_eq!(aliased.alias, "Sunset Shot");
    assert!(f1.exists());

    // Real disk rename + undo
    let (renamed, _) = apply_real_rename(&idx, "m1", "Sunset Shot", None).unwrap();
    assert_eq!(renamed.file_name, "Sunset Shot.jpg");
    assert!(lib.join("Sunset Shot.jpg").exists());
    assert!(!f1.exists());

    let undone = undo_last_rename(&idx, Some("m1")).unwrap();
    assert!(undone.is_some());
    assert!(f1.exists());

    // Bulk rename + batch undo
    let ids = vec!["m1".to_string(), "m2".to_string()];
    let previews = preview_bulk_rename(&idx, &ids, "{date} Trip {index}").unwrap();
    assert_eq!(previews.len(), 2);
    assert!(previews[0].new_name.contains("Trip 001.jpg"));

    let (batch_id, logs) = apply_bulk_rename(&idx, &ids, "{date} Trip {index}").unwrap();
    assert_eq!(logs.len(), 2);
    assert!(lib.join(&logs[0].new_name).exists());

    let reverted = undo_bulk_rename(&idx, &batch_id).unwrap();
    assert_eq!(reverted, 2);
    assert!(f1.exists() && f2.exists());

    // Move to folder (spring-loaded drop) + safe trash
    let archive = dir.join("Archive");
    let moved = move_media_to_folder(&idx, "m1", &archive).unwrap();
    assert!(archive.join("IMG_01.jpg").exists());
    assert_eq!(moved.folder_path, archive.to_string_lossy().replace('\\', "/"));

    let trash = ManagedTrashBackend::default();
    let trashed = move_to_trash(&idx, &trash, "m1").unwrap();
    assert!(std::path::Path::new(&trashed).exists());
    assert!(!archive.join("IMG_01.jpg").exists());

    let visible = idx
        .list_media(&MediaFilter::default(), SortSpec::default(), None, 50)
        .unwrap();
    assert_eq!(visible.total_count, 1);

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn video_first_third_thumbnail_persists_across_restarts_and_cover_crop_works() {
    assert_eq!(select_random_first_third_frame_ms(0, None), 0);
    for _ in 0..100 {
        let t = select_random_first_third_frame_ms(90_000, None);
        assert!(t <= 30_000);
    }

    let dir = temp_dir("mg-media-thumb");
    let db_path = dir.join("gallery.db");
    let cache_dir = dir.join("thumbs");
    let vid_path = dir.join("clip.mp4");
    let audio_path = dir.join("song.mp3");
    let custom_cover = dir.join("custom.jpg");
    std::fs::write(&vid_path, b"fake-mp4").unwrap();
    std::fs::write(&audio_path, b"fake-mp3").unwrap();
    std::fs::write(&custom_cover, b"fake-jpg").unwrap();

    let video = MediaRecord {
        id: "vid-1".into(),
        device_id: "pc-1".into(),
        path: vid_path.to_string_lossy().replace('\\', "/"),
        folder_path: dir.to_string_lossy().replace('\\', "/"),
        file_name: "clip.mp4".into(),
        media_type: MediaType::Video,
        mime: "video/mp4".into(),
        size: 8,
        blake3: String::new(),
        partial_hash: String::new(),
        created_at: 1_773_489_600_000,
        modified_at: 1_773_489_600_000,
        duration_ms: 120_000,
        width: 1920,
        height: 1080,
        gps_lat: None,
        gps_lon: None,
        place: None,
        tags: MediaTags::default(),
        alias: "Clip".into(),
        auto_alias: "Clip".into(),
        cover_id: None,
        sync_state: SyncState::Local,
        is_duplicate: false,
        duplicate_group_id: None,
        is_trashed: false,
        trashed_path: None,
        updated_at: 1_773_489_600_000,
    };

    let sidecar = DesktopFfmpegSidecar::default();
    let first_cover = {
        let idx1 = setup_index(Some(&db_path));
        idx1.upsert_media(&video).unwrap();
        let mgr1 = ThumbnailManager {
            index: &idx1,
            sidecar: &sidecar,
            cache_dir: &cache_dir,
            max_cache_bytes: None,
        };
        mgr1.ensure_thumbnail(&video, Some(0.5)).unwrap()
    };
    assert!(first_cover.frame_time_ms <= 40_000);
    assert!(std::path::Path::new(&first_cover.file_path).exists());

    // Restart with a different RNG sample — cached frame remains fixed (PRD I1)
    let idx2 = setup_index(Some(&db_path));
    let mgr2 = ThumbnailManager {
        index: &idx2,
        sidecar: &sidecar,
        cache_dir: &cache_dir,
        max_cache_bytes: None,
    };
    let reloaded = mgr2.ensure_thumbnail(&video, Some(0.95)).unwrap();
    assert_eq!(reloaded.frame_time_ms, first_cover.frame_time_ms);
    assert_eq!(reloaded.file_path, first_cover.file_path);

    // Aspect presets (4:3, 9:16, 16:9, 1:1, custom)
    assert_eq!(
        compute_crop_for_aspect(1600, 1200, core_index::AspectPreset::Ratio1x1, None).width,
        1200
    );
    assert_eq!(
        compute_crop_for_aspect(1920, 1080, core_index::AspectPreset::Ratio9x16, None).width,
        608
    );

    // Audio cover requires confirmation (PRD I3)
    let audio = MediaRecord {
        id: "aud-1".into(),
        media_type: MediaType::Audio,
        path: audio_path.to_string_lossy().replace('\\', "/"),
        file_name: "song.mp3".into(),
        mime: "audio/mpeg".into(),
        ..video
    };
    idx2.upsert_media(&audio).unwrap();
    let initial_audio_cover = mgr2.ensure_thumbnail(&audio, None).unwrap();

    let err = replace_audio_cover(
        &idx2,
        "aud-1",
        &custom_cover,
        &cache_dir,
        core_index::AspectPreset::Ratio16x9,
        None,
        false,
    )
    .unwrap_err();
    assert!(matches!(err, MediaOpError::ConfirmationRequired(_)));

    let replaced = replace_audio_cover(
        &idx2,
        "aud-1",
        &custom_cover,
        &cache_dir,
        core_index::AspectPreset::Ratio16x9,
        None,
        true,
    )
    .unwrap();
    assert_eq!(replaced.aspect, core_index::AspectPreset::Ratio16x9);

    remove_audio_cover(&idx2, "aud-1", true).unwrap();
    let restored = restore_audio_cover(&idx2, "aud-1").unwrap().unwrap();
    assert_eq!(restored.file_path, initial_audio_cover.file_path);

    let _ = std::fs::remove_dir_all(dir);
}
