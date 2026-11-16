use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use core_index::{
    import_legacy_sqlite, ingest_android_mediastore_batch, now_ms, scan_folders, AppSettings,
    AspectPreset, CoverRecord, CropRect, DeviceRecord, DeviceType, DownloadJobRecord,
    DuplicateGroup, FolderSummary, IndexError, LegacyImportReport, MediaFilter, MediaIndex,
    MediaPage, MediaRecord, MediaTags, MediaType, PinnedFolderRecord, RenameLogRecord,
    SavedSearchRecord, ScanProgress, ScanRequest, SortSpec, TransferRecord, TransportKind,
};
use core_media::{
    apply_bulk_rename, apply_real_rename, extract_metadata_and_alias, generate_auto_alias,
    move_media_to_folder, move_to_trash, pick_video_frame, preview_bulk_rename, remove_audio_cover,
    replace_audio_cover, reroll_video_thumbnail, restore_audio_cover, set_alias, set_custom_cover,
    undo_bulk_rename, undo_last_rename, AutoAliasInput, BulkRenamePreviewItem, MediaOpError,
    ThumbnailManager,
};
use core_net::{NetError, PairingManager, PairingSession};
use core_platform::{
    AndroidMediaStoreRow, DesktopFfmpegSidecar, DesktopFileWatcher, DesktopYtDlpBackend,
    EncryptedMemoryStore, ManagedTrashBackend, PlatformError, SecureStore, ThumbnailSidecar,
    TrashBackend, YtDlpBackend,
};
use core_sync::{
    GoogleCloudSyncClient, GoogleSession, InMemoryDriveBackend, IndexSyncManager, SyncError,
};
use core_ytdl::{YoutubeDownloadRequest, YoutubeDownloader, YtdlError};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::music::{
    build_unified_music_library, group_unified_tracks, MusicGroupBy, UnifiedMusicTrack,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApiErrorCode {
    InvalidInput,
    NotFound,
    AlreadyExists,
    IoFailure,
    DatabaseFailure,
    NetworkFailure,
    UnpairedPeer,
    PeerOffline,
    HashMismatch,
    ConfirmationRequired,
    Cancelled,
    Internal,
}

#[derive(Debug, Clone, Error, Serialize, Deserialize)]
#[error("[{code:?}] {message}")]
pub struct ApiError {
    pub code: ApiErrorCode,
    pub message: String,
}

impl From<IndexError> for ApiError {
    fn from(err: IndexError) -> Self {
        let code = match &err {
            IndexError::NotFound(_) => ApiErrorCode::NotFound,
            IndexError::Invalid(_) => ApiErrorCode::InvalidInput,
            IndexError::Io(_) => ApiErrorCode::IoFailure,
            IndexError::Sql(_) => ApiErrorCode::DatabaseFailure,
            _ => ApiErrorCode::Internal,
        };
        Self {
            code,
            message: err.to_string(),
        }
    }
}

impl From<MediaOpError> for ApiError {
    fn from(err: MediaOpError) -> Self {
        let code = match &err {
            MediaOpError::NotFound(_) => ApiErrorCode::NotFound,
            MediaOpError::Conflict(_) => ApiErrorCode::AlreadyExists,
            MediaOpError::InvalidName(_) => ApiErrorCode::InvalidInput,
            MediaOpError::ConfirmationRequired(_) => ApiErrorCode::ConfirmationRequired,
            MediaOpError::Io(_) => ApiErrorCode::IoFailure,
            MediaOpError::Index(e) => {
                return Self::from(match e {
                    IndexError::NotFound(m) => IndexError::NotFound(m.clone()),
                    _ => IndexError::Invalid(e.to_string()),
                })
            }
            MediaOpError::Platform(_) => ApiErrorCode::IoFailure,
        };
        Self {
            code,
            message: err.to_string(),
        }
    }
}

impl From<NetError> for ApiError {
    fn from(err: NetError) -> Self {
        let code = match &err {
            NetError::UnpairedPeer(_) => ApiErrorCode::UnpairedPeer,
            NetError::PeerOffline(_) => ApiErrorCode::PeerOffline,
            NetError::HashMismatch { .. } => ApiErrorCode::HashMismatch,
            NetError::Pairing(_) => ApiErrorCode::InvalidInput,
            NetError::Io(_) => ApiErrorCode::IoFailure,
            _ => ApiErrorCode::NetworkFailure,
        };
        Self {
            code,
            message: err.to_string(),
        }
    }
}

impl From<SyncError> for ApiError {
    fn from(err: SyncError) -> Self {
        let code = match &err {
            SyncError::NotFound(_) => ApiErrorCode::NotFound,
            SyncError::PeerOffline(_) => ApiErrorCode::PeerOffline,
            SyncError::AuthRequired => ApiErrorCode::ConfirmationRequired,
            SyncError::Io(_) => ApiErrorCode::IoFailure,
            _ => ApiErrorCode::Internal,
        };
        Self {
            code,
            message: err.to_string(),
        }
    }
}

impl From<YtdlError> for ApiError {
    fn from(err: YtdlError) -> Self {
        Self {
            code: ApiErrorCode::IoFailure,
            message: err.to_string(),
        }
    }
}

impl From<PlatformError> for ApiError {
    fn from(err: PlatformError) -> Self {
        Self {
            code: ApiErrorCode::IoFailure,
            message: err.to_string(),
        }
    }
}

/// Coarse event stream emitted to Flutter via `flutter_rust_bridge` `StreamSink`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", content = "payload")]
pub enum CoreEvent {
    ScanProgress(ScanProgress),
    TransferProgress(TransferRecord),
    DownloadProgress(DownloadJobRecord),
    DeviceUpdated(DeviceRecord),
    LegacyImportCompleted(LegacyImportReport),
    InfoBanner(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreConfig {
    pub db_path: Option<PathBuf>,
    pub cache_dir: PathBuf,
    pub self_device_id: String,
    pub self_device_name: String,
    pub self_device_type: DeviceType,
    pub max_thumb_cache_bytes: Option<u64>,
}

/// Coarse-grained FRB v2 facade owning SQLite, background workers, and event streams.
pub struct MediaGalleryCore {
    pub index: MediaIndex,
    pub cache_dir: PathBuf,
    pub self_device_id: String,
    pub self_device_name: String,
    pub max_thumb_cache_bytes: Option<u64>,
    sidecar: Arc<dyn ThumbnailSidecar>,
    trash: Arc<dyn TrashBackend>,
    ytdl_backend: Arc<dyn YtDlpBackend>,
    watcher: DesktopFileWatcher,
    pairing: Arc<PairingManager>,
    cloud_sync: GoogleCloudSyncClient,
    events_tx: broadcast::Sender<CoreEvent>,
    cancel_flags: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

impl MediaGalleryCore {
    pub fn init(config: CoreConfig) -> Result<Self, ApiError> {
        std::fs::create_dir_all(&config.cache_dir).map_err(|e| ApiError {
            code: ApiErrorCode::IoFailure,
            message: e.to_string(),
        })?;
        let index = match &config.db_path {
            Some(p) => MediaIndex::open(p)?,
            None => MediaIndex::open_in_memory()?,
        };

        let self_dev = DeviceRecord {
            id: config.self_device_id.clone(),
            name: config.self_device_name.clone(),
            device_type: config.self_device_type,
            public_key: format!("ed25519:{}", config.self_device_id),
            last_seen: now_ms(),
            transports: vec![
                TransportKind::Wired,
                TransportKind::WifiLan,
                TransportKind::HolePunched,
                TransportKind::Relay,
                TransportKind::Bluetooth,
            ],
            host: Some("127.0.0.1".to_string()),
            port: Some(9443),
            is_self: true,
            is_online: true,
            paired_at: Some(now_ms()),
        };
        index.upsert_device(&self_dev)?;

        let secure_store: Arc<dyn SecureStore> = Arc::new(EncryptedMemoryStore::default());
        let pairing = Arc::new(PairingManager::new(
            index.clone(),
            Arc::clone(&secure_store),
        ));
        let cloud_sync = GoogleCloudSyncClient::new(
            Arc::clone(&secure_store),
            index.clone(),
            Arc::new(InMemoryDriveBackend::default()),
        );
        let (events_tx, _) = broadcast::channel(256);

        Ok(Self {
            index,
            cache_dir: config.cache_dir,
            self_device_id: config.self_device_id,
            self_device_name: config.self_device_name,
            max_thumb_cache_bytes: config.max_thumb_cache_bytes,
            sidecar: Arc::new(DesktopFfmpegSidecar::default()),
            trash: Arc::new(ManagedTrashBackend::default()),
            ytdl_backend: Arc::new(DesktopYtDlpBackend::default()),
            watcher: DesktopFileWatcher::new(),
            pairing,
            cloud_sync,
            events_tx,
            cancel_flags: Mutex::new(HashMap::new()),
        })
    }

    pub fn with_sidecar(mut self, sidecar: Arc<dyn ThumbnailSidecar>) -> Self {
        self.sidecar = sidecar;
        self
    }

    pub fn with_ytdl_backend(mut self, backend: Arc<dyn YtDlpBackend>) -> Self {
        self.ytdl_backend = backend;
        self
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<CoreEvent> {
        self.events_tx.subscribe()
    }

    pub fn cancel_operation(&self, operation_id: &str) -> bool {
        if let Ok(map) = self.cancel_flags.lock() {
            if let Some(flag) = map.get(operation_id) {
                flag.store(true, Ordering::Relaxed);
                return true;
            }
        }
        false
    }

    /// Runs folder scan on tokio's blocking threadpool and streams `ScanProgress` events without blocking callers.
    pub async fn start_scan(
        self: &Arc<Self>,
        roots: Vec<PathBuf>,
        excluded_folders: Vec<PathBuf>,
    ) -> Result<String, ApiError> {
        let op_id = Uuid::new_v4().to_string();
        let cancel_flag = Arc::new(AtomicBool::new(false));
        if let Ok(mut map) = self.cancel_flags.lock() {
            map.insert(op_id.clone(), Arc::clone(&cancel_flag));
        }

        let core = Arc::clone(self);
        let op_clone = op_id.clone();
        let settings = self.index.get_app_settings()?;
        let template = settings.alias_template;

        let _watch_res = self.watcher.watch_roots(&roots, |_paths| {});

        tokio::task::spawn_blocking(move || {
            let req = ScanRequest {
                operation_id: op_clone.clone(),
                device_id: core.self_device_id.clone(),
                device_name: core.self_device_name.clone(),
                roots,
                excluded_folders,
                compute_hashes: true,
                batch_size: 200,
                cancel: Some(&cancel_flag),
            };
            let _ = scan_folders(
                &core.index,
                &req,
                &|path, mt, fs_created, mod_at, dev_name| {
                    extract_metadata_and_alias(
                        path,
                        mt,
                        fs_created,
                        mod_at,
                        dev_name,
                        Some(&template),
                        Some(core.sidecar.as_ref()),
                    )
                },
                |prog| {
                    let _ = core.events_tx.send(CoreEvent::ScanProgress(prog.clone()));
                },
            );
            if let Ok(mut map) = core.cancel_flags.lock() {
                map.remove(&op_clone);
            }
        });

        Ok(op_id)
    }

    pub fn list_media(
        &self,
        filter: &MediaFilter,
        sort: SortSpec,
        cursor: Option<&str>,
        limit: u32,
    ) -> Result<MediaPage, ApiError> {
        Ok(self.index.list_media(filter, sort, cursor, limit)?)
    }

    pub fn list_folders(
        &self,
        device_id: Option<&str>,
        media_type: Option<MediaType>,
    ) -> Result<Vec<FolderSummary>, ApiError> {
        Ok(self.index.list_folders(device_id, media_type)?)
    }

    pub fn detect_duplicates(
        &self,
        device_id: Option<&str>,
    ) -> Result<Vec<DuplicateGroup>, ApiError> {
        Ok(self.index.detect_and_flag_duplicates(device_id)?)
    }

    pub fn ingest_android_mediastore_batch(
        &self,
        device_id: &str,
        device_name: &str,
        rows: &[AndroidMediaStoreRow],
    ) -> Result<u32, ApiError> {
        self.index.upsert_device(&DeviceRecord {
            id: device_id.to_string(),
            name: device_name.to_string(),
            device_type: DeviceType::Phone,
            public_key: format!("android:{device_id}"),
            last_seen: now_ms(),
            transports: vec![TransportKind::WifiLan, TransportKind::Wired],
            host: None,
            port: None,
            is_self: device_id == self.self_device_id,
            is_online: true,
            paired_at: Some(now_ms()),
        })?;
        let template = self.index.get_app_settings()?.alias_template;
        let count = ingest_android_mediastore_batch(
            &self.index,
            device_id,
            device_name,
            rows,
            &|r, mt, dev| {
                let tags = MediaTags {
                    title: r.title.clone(),
                    artist: r.artist.clone(),
                    album: r.album.clone(),
                    ..Default::default()
                };
                let ts = r.date_added_sec.max(r.date_modified_sec) * 1000;
                generate_auto_alias(&AutoAliasInput {
                    file_name: &r.display_name,
                    media_type: mt,
                    created_at: ts,
                    modified_at: r.date_modified_sec * 1000,
                    gps_lat: r.latitude,
                    gps_lon: r.longitude,
                    place: None,
                    tags: &tags,
                    device_name: dev,
                    template: Some(&template),
                    use_utc_date: true,
                })
            },
        )?;
        Ok(count)
    }

    pub fn set_alias(&self, media_id: &str, alias: &str) -> Result<MediaRecord, ApiError> {
        Ok(set_alias(&self.index, media_id, alias)?)
    }

    pub fn apply_real_rename(
        &self,
        media_id: &str,
        new_name: &str,
    ) -> Result<MediaRecord, ApiError> {
        let (rec, _) = apply_real_rename(&self.index, media_id, new_name, None)?;
        Ok(rec)
    }

    pub fn undo_last_rename(
        &self,
        media_id: Option<&str>,
    ) -> Result<Option<RenameLogRecord>, ApiError> {
        Ok(undo_last_rename(&self.index, media_id)?)
    }

    pub fn preview_bulk_rename(
        &self,
        media_ids: &[String],
        template: &str,
    ) -> Result<Vec<BulkRenamePreviewItem>, ApiError> {
        Ok(preview_bulk_rename(&self.index, media_ids, template)?)
    }

    pub fn apply_bulk_rename(
        &self,
        media_ids: &[String],
        template: &str,
    ) -> Result<(String, Vec<RenameLogRecord>), ApiError> {
        Ok(apply_bulk_rename(&self.index, media_ids, template)?)
    }

    pub fn undo_bulk_rename(&self, batch_id: &str) -> Result<u32, ApiError> {
        Ok(undo_bulk_rename(&self.index, batch_id)?)
    }

    pub fn move_media_to_folder(
        &self,
        media_id: &str,
        target_folder: &Path,
    ) -> Result<MediaRecord, ApiError> {
        Ok(move_media_to_folder(&self.index, media_id, target_folder)?)
    }

    pub fn move_to_trash(&self, media_id: &str) -> Result<String, ApiError> {
        Ok(move_to_trash(&self.index, self.trash.as_ref(), media_id)?)
    }

    pub fn ensure_thumbnail(&self, media_id: &str) -> Result<CoverRecord, ApiError> {
        let item = self
            .index
            .get_media_by_id(media_id)?
            .ok_or_else(|| ApiError {
                code: ApiErrorCode::NotFound,
                message: format!("media not found: {media_id}"),
            })?;
        let mgr = ThumbnailManager {
            index: &self.index,
            sidecar: self.sidecar.as_ref(),
            cache_dir: &self.cache_dir,
            max_cache_bytes: self.max_thumb_cache_bytes,
        };
        Ok(mgr.ensure_thumbnail(&item.media, None)?)
    }

    pub fn reroll_video_thumbnail(&self, media_id: &str) -> Result<CoverRecord, ApiError> {
        Ok(reroll_video_thumbnail(
            &self.index,
            self.sidecar.as_ref(),
            &self.cache_dir,
            media_id,
            None,
        )?)
    }

    pub fn pick_video_frame(
        &self,
        media_id: &str,
        frame_time_ms: u64,
    ) -> Result<CoverRecord, ApiError> {
        Ok(pick_video_frame(
            &self.index,
            self.sidecar.as_ref(),
            &self.cache_dir,
            media_id,
            frame_time_ms,
        )?)
    }

    pub fn set_custom_cover(
        &self,
        media_id: &str,
        image_path: &Path,
        aspect: AspectPreset,
        crop: Option<CropRect>,
        confirmed_embedded_write: bool,
    ) -> Result<CoverRecord, ApiError> {
        let item = self
            .index
            .get_media_by_id(media_id)?
            .ok_or_else(|| ApiError {
                code: ApiErrorCode::NotFound,
                message: format!("media not found: {media_id}"),
            })?;
        if item.media.media_type == MediaType::Audio {
            Ok(replace_audio_cover(
                &self.index,
                media_id,
                image_path,
                &self.cache_dir,
                aspect,
                crop,
                confirmed_embedded_write,
            )?)
        } else {
            Ok(set_custom_cover(
                &self.index,
                media_id,
                image_path,
                &self.cache_dir,
                aspect,
                crop,
                (item.media.width.max(1920), item.media.height.max(1080)),
            )?)
        }
    }

    pub fn remove_audio_cover(&self, media_id: &str, confirmed: bool) -> Result<(), ApiError> {
        Ok(remove_audio_cover(&self.index, media_id, confirmed)?)
    }

    pub fn restore_audio_cover(&self, media_id: &str) -> Result<Option<CoverRecord>, ApiError> {
        Ok(restore_audio_cover(&self.index, media_id)?)
    }

    pub fn create_pairing_session(&self) -> Result<PairingSession, ApiError> {
        let self_dev = self
            .index
            .get_device(&self.self_device_id)?
            .ok_or_else(|| ApiError {
                code: ApiErrorCode::NotFound,
                message: "self device missing".into(),
            })?;
        Ok(self.pairing.create_session(&self_dev, "{}", 300_000)?)
    }

    pub fn pair_with_pin(&self, pin: &str, remote: DeviceRecord) -> Result<DeviceRecord, ApiError> {
        let dev = self.pairing.pair_with_pin(pin, remote)?;
        let _ = self.events_tx.send(CoreEvent::DeviceUpdated(dev.clone()));
        Ok(dev)
    }

    pub fn list_devices(&self) -> Result<Vec<DeviceRecord>, ApiError> {
        Ok(self.index.list_devices()?)
    }

    pub fn forget_device(&self, device_id: &str) -> Result<(), ApiError> {
        Ok(self.pairing.forget_device(device_id)?)
    }

    pub fn assert_remote_download_allowed(&self, media_id: &str) -> Result<DeviceRecord, ApiError> {
        let mgr = IndexSyncManager::new(&self.index);
        let (_, owner) = mgr.assert_download_allowed(media_id)?;
        Ok(owner)
    }

    pub fn set_google_session(
        &self,
        email: Option<String>,
        access_token: Option<String>,
    ) -> Result<(), ApiError> {
        match (email, access_token) {
            (Some(mail), Some(tok)) => {
                self.cloud_sync.save_login_session(&GoogleSession {
                    email: mail,
                    access_token: tok,
                    expires_at: now_ms() + 3_600_000,
                })?;
            }
            _ => {
                self.cloud_sync.logout_and_wipe(&self.self_device_id)?;
            }
        }
        Ok(())
    }

    pub fn get_google_email(&self) -> Result<Option<String>, ApiError> {
        Ok(self.cloud_sync.get_session()?.map(|s| s.email))
    }

    pub fn list_unified_music(
        &self,
        group_by: MusicGroupBy,
    ) -> Result<Vec<(String, Vec<UnifiedMusicTrack>)>, ApiError> {
        let audio_page = self.index.list_media(
            &MediaFilter {
                media_type: Some(MediaType::Audio),
                ..Default::default()
            },
            SortSpec::default(),
            None,
            10_000,
        )?;
        let unified = build_unified_music_library(&audio_page.items);
        Ok(group_unified_tracks(&unified, group_by))
    }

    pub fn enqueue_youtube_download(
        &self,
        req: &YoutubeDownloadRequest,
    ) -> Result<DownloadJobRecord, ApiError> {
        let downloader = YoutubeDownloader::new(
            self.index.clone(),
            Arc::clone(&self.ytdl_backend),
            Arc::clone(&self.sidecar),
            &self.cache_dir,
        );
        let (job, _) = downloader.enqueue_and_download(req, |j| {
            let _ = self.events_tx.send(CoreEvent::DownloadProgress(j.clone()));
        })?;
        Ok(job)
    }

    pub fn update_yt_dlp(&self) -> Result<String, ApiError> {
        let downloader = YoutubeDownloader::new(
            self.index.clone(),
            Arc::clone(&self.ytdl_backend),
            Arc::clone(&self.sidecar),
            &self.cache_dir,
        );
        Ok(downloader.update_yt_dlp()?)
    }

    pub fn list_pinned_folders(&self) -> Result<Vec<PinnedFolderRecord>, ApiError> {
        Ok(self.index.list_pinned_folders()?)
    }

    pub fn upsert_pinned_folder(&self, pf: &PinnedFolderRecord) -> Result<(), ApiError> {
        Ok(self.index.upsert_pinned_folder(pf)?)
    }

    pub fn save_search(&self, saved: &SavedSearchRecord) -> Result<(), ApiError> {
        Ok(self.index.save_search(saved)?)
    }

    pub fn list_saved_searches(&self) -> Result<Vec<SavedSearchRecord>, ApiError> {
        Ok(self.index.list_saved_searches()?)
    }

    pub fn get_settings(&self) -> Result<AppSettings, ApiError> {
        Ok(self.index.get_app_settings()?)
    }

    pub fn save_settings(&self, settings: &AppSettings) -> Result<(), ApiError> {
        Ok(self.index.save_app_settings(settings)?)
    }

    pub fn import_legacy_database(
        &self,
        legacy_db_path: &Path,
    ) -> Result<LegacyImportReport, ApiError> {
        let report = import_legacy_sqlite(&self.index, legacy_db_path)?;
        let _ = self
            .events_tx
            .send(CoreEvent::LegacyImportCompleted(report.clone()));
        Ok(report)
    }

    pub fn shutdown(&self) {
        self.watcher.stop();
    }
}
