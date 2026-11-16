use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use core_index::{
    AspectPreset, AutoSyncRuleRecord, CoverRecord, CoverSource, DeviceRecord, DeviceType,
    MediaIndex, MediaRecord, MediaTags, MediaType, SyncState, TransportKind,
};
use core_platform::EncryptedMemoryStore;
use core_sync::{
    GoogleCloudSyncClient, GoogleSession, InMemoryDriveBackend, IndexSyncManager, SyncError,
};

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
fn serverless_delta_sync_offline_guard_auto_sync_and_h2_logout_wipe() {
    let dir = temp_dir("mg-sync-test");
    let thumb_path = dir.join("phone_thumb.png");
    std::fs::write(&thumb_path, b"PNG_DATA_SMALL").unwrap();

    // Phone exports metadata + thumbnail delta
    let phone_idx = MediaIndex::open_in_memory().unwrap();
    phone_idx
        .upsert_device(&DeviceRecord {
            id: "phone-1".into(),
            name: "Pixel 9".into(),
            device_type: DeviceType::Phone,
            public_key: "pk-phone".into(),
            last_seen: 1_700_000_000_000,
            transports: vec![TransportKind::WifiLan, TransportKind::Relay],
            host: Some("192.168.1.44".into()),
            port: Some(9443),
            is_self: true,
            is_online: true,
            paired_at: None,
        })
        .unwrap();

    phone_idx
        .upsert_media(&MediaRecord {
            id: "pm-1".into(),
            device_id: "phone-1".into(),
            path: "/storage/emulated/0/DCIM/Camera/VID_01.mp4".into(),
            folder_path: "/storage/emulated/0/DCIM/Camera".into(),
            file_name: "VID_01.mp4".into(),
            media_type: MediaType::Video,
            mime: "video/mp4".into(),
            size: 50_000_000,
            blake3: "b3-pm1".into(),
            partial_hash: "pb3-pm1".into(),
            created_at: 1_773_489_600_000,
            modified_at: 1_773_489_600_000,
            duration_ms: 45_000,
            width: 1920,
            height: 1080,
            gps_lat: None,
            gps_lon: None,
            place: None,
            tags: MediaTags::default(),
            alias: "Phone Clip".into(),
            auto_alias: "Phone Clip".into(),
            cover_id: Some("cov-pm1".into()),
            sync_state: SyncState::Local,
            is_duplicate: false,
            duplicate_group_id: None,
            is_trashed: false,
            trashed_path: None,
            updated_at: 1_773_489_600_000,
        })
        .unwrap();

    phone_idx
        .upsert_cover(&CoverRecord {
            id: "cov-pm1".into(),
            media_id: "pm-1".into(),
            source: CoverSource::Generated,
            file_path: thumb_path.to_string_lossy().to_string(),
            cloud_id: None,
            aspect: AspectPreset::Ratio16x9,
            crop: None,
            frame_time_ms: 9_000,
            updated_at: 1_773_489_600_000,
        })
        .unwrap();

    let phone_sync = IndexSyncManager::new(&phone_idx);
    let delta = phone_sync.export_local_delta("phone-1", 0, true).unwrap();
    assert!(delta
        .thumbnail_blobs_base64
        .get("pm-1")
        .unwrap()
        .starts_with("data:image/png;base64,"));

    // PC imports delta without Google login (PRD E5)
    let pc_idx = MediaIndex::open_in_memory().unwrap();
    pc_idx
        .upsert_device(&DeviceRecord {
            id: "pc-1".into(),
            name: "PC".into(),
            device_type: DeviceType::Pc,
            public_key: "pk-pc".into(),
            last_seen: 1_700_000_000_000,
            transports: vec![TransportKind::WifiLan],
            host: Some("127.0.0.1".into()),
            port: Some(9443),
            is_self: true,
            is_online: true,
            paired_at: None,
        })
        .unwrap();

    let pc_sync = IndexSyncManager::new(&pc_idx);
    let imported = pc_sync
        .import_remote_delta(delta.clone(), "pc-1", &dir.join("remote_thumbs"))
        .unwrap();
    assert_eq!(imported, 1);

    let remote = pc_idx.get_media_by_id("pm-1").unwrap().unwrap();
    assert_eq!(remote.media.sync_state, SyncState::Remote);
    assert!(std::path::Path::new(&remote.cover.unwrap().file_path).exists());
    assert!(pc_sync.build_stream_preview_ticket("pm-1").is_ok());

    // Auto-sync rule match (PRD E3)
    pc_idx
        .upsert_auto_sync_rule(&AutoSyncRuleRecord {
            id: "rule-cam".into(),
            name: "Sync Camera".into(),
            source_device_id: "phone-1".into(),
            target_device_id: "pc-1".into(),
            folder_pattern: "DCIM/Camera".into(),
            media_types: vec![MediaType::Video, MediaType::Image],
            enabled: true,
            created_at: 1_773_489_600_000,
        })
        .unwrap();
    assert_eq!(
        pc_sync.find_pending_auto_sync_items("pc-1").unwrap().len(),
        1
    );

    // Owner goes offline -> browsable thumbnail persists, Download blocked cleanly (PRD E6)
    pc_idx.update_device_online("phone-1", false).unwrap();
    let offline_err = pc_sync.assert_download_allowed("pm-1").unwrap_err();
    assert!(matches!(offline_err, SyncError::PeerOffline(_)));

    // Google Cloud Drive appDataFolder push + H2 logout wipe
    let store = Arc::new(EncryptedMemoryStore::default());
    let drive = Arc::new(InMemoryDriveBackend::default());
    let cloud = GoogleCloudSyncClient::new(store, pc_idx.clone(), drive);
    cloud
        .save_login_session(&GoogleSession {
            email: "user@example.com".into(),
            access_token: "tok-xyz".into(),
            expires_at: 9_999_999_999_999,
        })
        .unwrap();
    cloud.push_index_delta(&delta).unwrap();
    assert_eq!(cloud.pull_peer_deltas("pc-1").unwrap().len(), 1);

    cloud.logout_and_wipe("pc-1").unwrap();
    assert!(cloud.get_session().unwrap().is_none());
    assert!(pc_idx.get_media_by_id("pm-1").unwrap().is_none());

    let _ = std::fs::remove_dir_all(dir);
}
