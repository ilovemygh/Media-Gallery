use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use core_api::{CoreConfig, CoreEvent, MediaGalleryCore};
use core_index::{DeviceType, MediaFilter, SortSpec};
use core_net::{CtrlMessage, IrohQuicNode};
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

/// M0 Spike (a): iroh endpoints connect via key-based QUIC dialing across nodes.
#[tokio::test]
async fn spike_a_iroh_endpoints_connect_and_exchange_control_messages() {
    let server = IrohQuicNode::bind(false, None, Arc::new(|_| Ok(())), Arc::new(|_| Ok(())))
        .await
        .unwrap();
    let client = IrohQuicNode::bind(false, None, Arc::new(|_| Ok(())), Arc::new(|_| Ok(())))
        .await
        .unwrap();

    let reply = client
        .send_ctrl(
            server.endpoint.addr(),
            &CtrlMessage::Ping {
                from_device: "spike-client".into(),
            },
        )
        .await
        .unwrap();
    assert!(reply.ok);
    assert_eq!(reply.message, "pong:spike-client");

    client.shutdown().await;
    server.shutdown().await;
}

/// M0 Spike (b), (c), (d): FRB-style event streaming + cursor paging, thumbnail sidecar by path,
/// and Kotlin MediaStore row batch ingestion into Rust SQLite + FTS5.
#[tokio::test]
async fn spike_bcd_frb_streams_sidecar_thumbnail_and_mediastore_ingest() {
    let dir = temp_dir("mg-m0-spikes");
    let lib = dir.join("videos");
    let cache = dir.join("cache");
    std::fs::create_dir_all(&lib).unwrap();
    std::fs::write(lib.join("holiday_clip.mp4"), b"fake-mp4-stream").unwrap();

    let core = Arc::new(
        MediaGalleryCore::init(CoreConfig {
            db_path: None,
            cache_dir: cache.clone(),
            self_device_id: "pc-spike".into(),
            self_device_name: "Spike PC".into(),
            self_device_type: DeviceType::Pc,
            max_thumb_cache_bytes: Some(50 * 1024 * 1024),
        })
        .unwrap(),
    );

    // Spike (b): subscribe to broadcast stream and start background scan
    let mut rx = core.subscribe_events();
    let op_id = core.start_scan(vec![lib], vec![]).await.unwrap();
    assert!(!op_id.is_empty());

    let mut saw_completed_scan = false;
    for _ in 0..20 {
        if let Ok(CoreEvent::ScanProgress(prog)) =
            tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
                .await
                .unwrap()
        {
            if !prog.in_progress {
                saw_completed_scan = true;
                break;
            }
        }
    }
    assert!(saw_completed_scan);

    // Paged query returns item
    let page = core
        .list_media(&MediaFilter::default(), SortSpec::default(), None, 10)
        .unwrap();
    assert_eq!(page.total_count, 1);

    // Spike (c): Sidecar writes thumbnail file to cache and returns file path (not raw bytes)
    let vid_id = &page.items[0].media.id;
    let cover = core.ensure_thumbnail(vid_id).unwrap();
    assert!(std::path::Path::new(&cover.file_path).exists());

    // Spike (d): Kotlin MediaStore rows ingest into Rust index and are FTS5-searchable
    let rows = vec![AndroidMediaStoreRow {
        uri: "content://media/external/images/media/9001".into(),
        display_name: "IMG_2031.jpg".into(),
        relative_path: "DCIM/Camera/".into(),
        mime_type: "image/jpeg".into(),
        size: 3_500_000,
        date_added_sec: 1_773_489_600,
        date_modified_sec: 1_773_489_600,
        date_taken_ms: None,
        duration_ms: 0,
        width: 4032,
        height: 3024,
        latitude: Some(-8.7185),
        longitude: Some(115.1686),
        artist: None,
        album: None,
        title: None,
    }];
    let ingested = core
        .ingest_android_mediastore_batch("phone-spike", "Pixel 9", &rows)
        .unwrap();
    assert_eq!(ingested, 1);

    let hits = core
        .list_media(
            &MediaFilter {
                query: "2026 beach".into(),
                ..Default::default()
            },
            SortSpec::default(),
            None,
            10,
        )
        .unwrap();
    assert_eq!(hits.total_count, 1);
    assert_eq!(hits.items[0].media.file_name, "IMG_2031.jpg");

    core.shutdown();
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn frb_live_core_scan_thumbnails_stats_and_trash_cycle_work() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let dir = std::env::temp_dir().join("mg-frb-live-test");
        let _ = std::fs::remove_dir_all(&dir);
        let media_dir = dir.join("Photos");
        std::fs::create_dir_all(&media_dir).unwrap();
        let p1 = media_dir.join("IMG_001.jpg");
        let p2 = media_dir.join("IMG_002_copy.jpg");
        let img_bytes = b"fake-jpeg-data-for-duplicate-check";
        std::fs::write(&p1, img_bytes).unwrap();
        std::fs::write(&p2, img_bytes).unwrap();

        let db_path = dir.join("gallery.db");
        let cache_dir = dir.join("cache");
        core_api::api::frb_init_core(
            Some(db_path.to_string_lossy().to_string()),
            cache_dir.to_string_lossy().to_string(),
            "test-pc".to_string(),
            "Test PC".to_string(),
            false,
        )
        .unwrap();

        let scanned = core_api::api::frb_scan_folders(
            vec![media_dir.to_string_lossy().to_string()],
            Vec::new(),
        )
        .await
        .unwrap();
        assert_eq!(scanned, 2);

        let stats = core_api::api::frb_get_storage_stats().await.unwrap();
        assert_eq!(stats.total_files, 2);
        assert_eq!(stats.image_count, 2);
        assert_eq!(stats.duplicate_groups, 1);

        let page = core_api::api::frb_list_media(
            String::new(),
            None,
            None,
            None,
            None,
            None,
            false,
            false,
            "date".to_string(),
            "desc".to_string(),
            None,
            50,
        )
        .await
        .unwrap();
        assert_eq!(page.total_count, 2);

        let ids: Vec<String> = page.items.iter().map(|i| i.id.clone()).collect();
        let thumbs = core_api::api::frb_ensure_thumbnails_batch(ids.clone())
            .await
            .unwrap();
        assert_eq!(thumbs.len(), 2);
        for t in &thumbs {
            assert!(std::path::Path::new(&t.file_path).exists());
        }

        let target_id = &ids[0];
        core_api::api::frb_move_to_trash(target_id.clone())
            .await
            .unwrap();
        let trashed_page = core_api::api::frb_list_media(
            String::new(),
            None,
            None,
            None,
            None,
            None,
            false,
            true,
            "date".to_string(),
            "desc".to_string(),
            None,
            50,
        )
        .await
        .unwrap();
        assert_eq!(trashed_page.total_count, 1);

        core_api::api::frb_restore_from_trash(target_id.clone())
            .await
            .unwrap();
        let stats_restored = core_api::api::frb_get_storage_stats().await.unwrap();
        assert_eq!(stats_restored.trashed_count, 0);
        assert_eq!(stats_restored.total_files, 2);

        core_api::api::frb_move_to_trash(target_id.clone())
            .await
            .unwrap();
        let emptied = core_api::api::frb_empty_trash(None).await.unwrap();
        assert_eq!(emptied, 1);
        let stats_final = core_api::api::frb_get_storage_stats().await.unwrap();
        assert_eq!(stats_final.trashed_count, 0);
        assert_eq!(stats_final.total_files, 1);

        // Create a duplicate again, test frb_list_duplicate_groups + frb_clean_duplicates
        let p3 = media_dir.join("IMG_003_dup.jpg");
        std::fs::write(&p3, img_bytes).unwrap();
        core_api::api::frb_scan_folders(vec![media_dir.to_string_lossy().to_string()], Vec::new())
            .await
            .unwrap();
        let groups = core_api::api::frb_list_duplicate_groups().await.unwrap();
        assert_eq!(groups.len(), 1);
        let cleaned = core_api::api::frb_clean_duplicates(true).await.unwrap();
        assert_eq!(cleaned.cleaned_count, 1);
        assert!(cleaned.freed_bytes > 0);

        // Saved searches & unified music
        core_api::api::frb_save_search(
            "All Images".to_string(),
            "".to_string(),
            Some("image".to_string()),
        )
        .await
        .unwrap();
        let searches = core_api::api::frb_list_saved_searches().await.unwrap();
        assert_eq!(searches.len(), 1);
        let music_groups = core_api::api::frb_list_unified_music("artist".to_string())
            .await
            .unwrap();
        assert!(music_groups.is_empty());

        // Pairing + Auto-Sync rules
        let sess = core_api::api::frb_create_pairing_session().await.unwrap();
        assert_eq!(sess.pin.len(), 6);
        let paired = core_api::api::frb_pair_with_device(sess.pin, Some("My Phone".to_string()))
            .await
            .unwrap();
        assert_eq!(paired.name, "My Phone");

        core_api::api::frb_upsert_auto_sync_rule(
            "rule-cam".to_string(),
            "Backup Camera".to_string(),
            paired.id.clone(),
            "DCIM/Camera".to_string(),
            true,
        )
        .await
        .unwrap();
        let rules = core_api::api::frb_list_auto_sync_rules().await.unwrap();
        assert_eq!(rules.len(), 1);

        let _ = std::fs::remove_dir_all(&dir);
    });
}
