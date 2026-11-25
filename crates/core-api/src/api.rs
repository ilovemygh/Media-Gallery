use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use core_index::{
    import_legacy_sqlite, ingest_android_mediastore_batch, now_ms, scan_folders, AppSettings,
    AspectPreset, AutoSyncRuleRecord, CoverRecord, CropRect, DeviceRecord, DeviceType,
    DownloadFormat, DownloadJobRecord, DuplicateGroup, FolderSummary, IndexError,
    LegacyImportReport, MediaFilter, MediaIndex, MediaPage, MediaRecord, MediaTags, MediaType,
    MediaWithCover, PinnedFolderRecord, RenameLogRecord, SavedSearchRecord, ScanProgress,
    ScanRequest, SortDir, SortField, SortSpec, StorageStats, SyncState, TransferRecord,
    TransferStatus, TransportKind,
};
use core_media::{
    apply_bulk_rename, apply_real_rename, empty_trash, extract_metadata_and_alias,
    generate_auto_alias, move_media_to_folder, move_to_trash, pick_video_frame,
    preview_bulk_rename, remove_audio_cover, replace_audio_cover, reroll_video_thumbnail,
    restore_audio_cover, restore_from_trash, set_alias, set_custom_cover, undo_bulk_rename,
    undo_last_rename, AutoAliasInput, BulkRenamePreviewItem, MediaOpError, ThumbnailManager,
};
use core_net::{
    EndpointAddr, IrohQuicNode, IrohQuicTransport, NetError, PairingManager, PairingSession,
    QrPairingData, TransferEngine, TransferReceiver,
};
use core_platform::{
    resolve_sidecar_bin, AndroidMediaStoreRow, DesktopFfmpegSidecar, DesktopFileWatcher,
    DesktopYtDlpBackend, EncryptedMemoryStore, ManagedTrashBackend, PlatformError, SecureStore,
    ThumbnailSidecar, TrashBackend, YtDlpBackend,
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
#[flutter_rust_bridge::frb(ignore)]
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

#[flutter_rust_bridge::frb(ignore)]
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
#[flutter_rust_bridge::frb(ignore)]
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
    iroh_node: tokio::sync::Mutex<Option<(Arc<IrohQuicNode>, Arc<IrohQuicTransport>)>>,
    events_tx: broadcast::Sender<CoreEvent>,
    cancel_flags: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

#[flutter_rust_bridge::frb(ignore)]
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
            iroh_node: tokio::sync::Mutex::new(None),
            events_tx,
            cancel_flags: Mutex::new(HashMap::new()),
        })
    }

    /// Lazily binds the real `iroh` QUIC P2P endpoint and background receive loops.
    pub async fn ensure_iroh_node(
        &self,
    ) -> Result<(Arc<IrohQuicNode>, Arc<IrohQuicTransport>), ApiError> {
        let mut guard = self.iroh_node.lock().await;
        if let Some((ref node, ref transport)) = *guard {
            return Ok((Arc::clone(node), Arc::clone(transport)));
        }

        let custom_relay = self
            .index
            .get_app_settings()
            .ok()
            .and_then(|s| s.custom_relay_url);
        let recv_dir = self.cache_dir.join("received");
        let _ = std::fs::create_dir_all(&recv_dir);
        let receiver = Arc::new(TransferReceiver::new(&recv_dir));
        let idx_for_delta = self.index.clone();
        let cache_for_delta = self.cache_dir.clone();

        let node = IrohQuicNode::bind(
            true,
            custom_relay.as_deref(),
            Arc::new(move |frame| receiver.handle_chunk(&frame).map(|_| ())),
            Arc::new(move |delta| {
                let source_id = delta.device.id.clone();
                let mgr = IndexSyncManager::new(&idx_for_delta);
                mgr.import_remote_delta(delta, &source_id, &cache_for_delta)
                    .map(|_| ())
                    .map_err(|e| NetError::Pairing(e.to_string()))
            }),
        )
        .await?;
        let node = Arc::new(node);
        let transport = Arc::new(IrohQuicTransport::new(
            TransportKind::WifiLan,
            Arc::clone(&node),
        ));
        *guard = Some((Arc::clone(&node), Arc::clone(&transport)));
        Ok((node, transport))
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
                let ts = r.created_at_ms();
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
        let mut effective_req = req.clone();
        if effective_req.ffmpeg_bin.is_none() {
            let resolved_ffmpeg = resolve_sidecar_bin("ffmpeg");
            if resolved_ffmpeg.is_file() {
                effective_req.ffmpeg_bin = Some(resolved_ffmpeg.to_string_lossy().to_string());
            }
        }
        let downloader = YoutubeDownloader::new(
            self.index.clone(),
            Arc::clone(&self.ytdl_backend),
            Arc::clone(&self.sidecar),
            &self.cache_dir,
        );
        let (job, _) = downloader.enqueue_and_download(&effective_req, |j| {
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

    pub fn remove_pinned_folder(&self, id: &str) -> Result<(), ApiError> {
        Ok(self.index.remove_pinned_folder(id)?)
    }

    pub fn reorder_pinned_folders(&self, ordered_ids: &[String]) -> Result<(), ApiError> {
        Ok(self.index.reorder_pinned_folders(ordered_ids)?)
    }

    pub fn list_download_jobs(&self) -> Result<Vec<DownloadJobRecord>, ApiError> {
        Ok(self.index.list_download_jobs()?)
    }

    pub fn get_storage_stats(&self) -> Result<StorageStats, ApiError> {
        let _ = self.index.detect_and_flag_duplicates(None);
        Ok(self.index.get_storage_stats()?)
    }

    pub fn restore_from_trash(&self, media_id: &str) -> Result<MediaRecord, ApiError> {
        Ok(restore_from_trash(&self.index, media_id)?)
    }

    pub fn empty_trash(&self, media_id: Option<&str>) -> Result<u64, ApiError> {
        Ok(empty_trash(&self.index, media_id)?)
    }

    pub fn update_cover_aspect(
        &self,
        media_id: &str,
        aspect: AspectPreset,
    ) -> Result<CoverRecord, ApiError> {
        let mut cover = self.ensure_thumbnail(media_id)?;
        cover.aspect = aspect;
        cover.updated_at = now_ms();
        self.index.upsert_cover(&cover)?;
        Ok(cover)
    }

    /// Generates missing thumbnails for a batch of media IDs using bounded worker threads.
    pub fn ensure_thumbnails_batch(&self, media_ids: &[String]) -> Vec<(String, String)> {
        if media_ids.is_empty() {
            return Vec::new();
        }
        let max_workers = 4usize.min(media_ids.len());
        let chunk_size = media_ids.len().div_ceil(max_workers);
        let results = Mutex::new(Vec::with_capacity(media_ids.len()));

        std::thread::scope(|s| {
            for chunk in media_ids.chunks(chunk_size) {
                let results_ref = &results;
                s.spawn(move || {
                    for id in chunk {
                        if let Ok(cov) = self.ensure_thumbnail(id) {
                            if !cov.file_path.is_empty() {
                                if let Ok(mut guard) = results_ref.lock() {
                                    guard.push((id.clone(), cov.file_path));
                                }
                            }
                        }
                    }
                });
            }
        });

        results.into_inner().unwrap_or_default()
    }

    pub fn scan_now(
        &self,
        roots: Vec<PathBuf>,
        mut excluded_folders: Vec<PathBuf>,
    ) -> Result<u64, ApiError> {
        let template = self.index.get_app_settings()?.alias_template;
        let prefs = core_platform::load_extended_prefs(&self.cache_dir);
        if !prefs.scan_videos {
            excluded_folders.push(PathBuf::from("__mg_disable_type__:video"));
        }
        if !prefs.scan_images {
            excluded_folders.push(PathBuf::from("__mg_disable_type__:image"));
        }
        if !prefs.scan_audio {
            excluded_folders.push(PathBuf::from("__mg_disable_type__:audio"));
        }
        if !prefs.skip_system_folders {
            excluded_folders.push(PathBuf::from("__mg_allow_system_dirs__"));
        }
        if !prefs.enabled_extensions.is_empty() {
            let enabled: std::collections::HashSet<String> = prefs
                .enabled_extensions
                .iter()
                .map(|e| e.trim_start_matches('.').to_ascii_lowercase())
                .collect();
            for ext in [
                "mp4", "mkv", "webm", "mov", "avi", "mp3", "m4a", "flac", "wav", "ogg", "png",
                "jpg", "jpeg", "gif", "webp", "heic",
            ] {
                if !enabled.contains(ext) {
                    excluded_folders.push(PathBuf::from(format!("__mg_disable_ext__:{ext}")));
                }
            }
        }
        let _watch_res = self.watcher.watch_roots(&roots, |_paths| {});
        let req = ScanRequest {
            operation_id: Uuid::new_v4().to_string(),
            device_id: self.self_device_id.clone(),
            device_name: self.self_device_name.clone(),
            roots,
            excluded_folders,
            compute_hashes: true,
            batch_size: 128,
            cancel: None,
        };
        let mut last_total = 0u64;
        scan_folders(
            &self.index,
            &req,
            &|path, mt, fs_created, mod_at, dev_name| {
                extract_metadata_and_alias(
                    path,
                    mt,
                    fs_created,
                    mod_at,
                    dev_name,
                    Some(&template),
                    Some(self.sidecar.as_ref()),
                )
            },
            |prog| {
                last_total = prog.scanned_files;
                let _ = self.events_tx.send(CoreEvent::ScanProgress(prog.clone()));
            },
        )?;
        let _ = self.index.detect_and_flag_duplicates(None);
        Ok(last_total)
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

/// Summary returned across FFI when importing a read-only v0.1 SQLite database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrbLegacyImportReport {
    pub imported_devices: u64,
    pub imported_media: u64,
    pub imported_covers: u64,
    pub imported_pinned_folders: u64,
    pub imported_rename_logs: u64,
    pub skipped_items: u64,
    pub auth_relogin_required: bool,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrbCoverItem {
    pub id: String,
    pub media_id: String,
    pub file_path: String,
    pub aspect: String,
    pub frame_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrbMediaItem {
    pub id: String,
    pub device_id: String,
    pub device_name: String,
    pub device_online: bool,
    pub path: String,
    pub folder_path: String,
    pub file_name: String,
    pub media_type: String,
    pub size: u64,
    pub created_at: i64,
    pub duration_ms: u64,
    pub width: u32,
    pub height: u32,
    pub alias: String,
    pub auto_alias: String,
    pub sync_state: String,
    pub place: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub cover: Option<FrbCoverItem>,
    pub is_duplicate: bool,
    pub is_trashed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrbMediaPage {
    pub items: Vec<FrbMediaItem>,
    pub total_count: u64,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrbFolderSummary {
    pub path: String,
    pub name: String,
    pub device_id: String,
    pub item_count: u64,
    pub preview_covers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrbDeviceItem {
    pub id: String,
    pub name: String,
    pub device_type: String,
    pub transports: Vec<String>,
    pub is_self: bool,
    pub is_online: bool,
    pub paired_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrbPinnedFolder {
    pub id: String,
    pub device_id: String,
    pub path: String,
    pub label: String,
    pub icon: String,
    pub position: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrbAppSettings {
    pub sidebar_width: u32,
    pub sidebar_collapsed: bool,
    pub dark_mode: bool,
    pub manual_transport_override: Option<String>,
    pub alias_template: String,
    pub scan_folders: Vec<String>,
    pub excluded_folders: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrbDuplicateGroup {
    pub group_id: String,
    pub size: u64,
    pub reclaimable_bytes: u64,
    pub items: Vec<FrbMediaItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrbCleanupReport {
    pub cleaned_count: u64,
    pub freed_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrbUnifiedTrack {
    pub canonical_key: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: u64,
    pub primary: FrbMediaItem,
    pub source_devices: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrbMusicGroup {
    pub group_name: String,
    pub tracks: Vec<FrbUnifiedTrack>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrbTransferItem {
    pub id: String,
    pub media_id: String,
    pub file_name: String,
    pub from_device: String,
    pub to_device: String,
    pub status: String,
    pub bytes_done: u64,
    pub bytes_total: u64,
    pub path_kind: String,
    pub speed_bps: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrbAutoSyncRule {
    pub id: String,
    pub name: String,
    pub source_device_id: String,
    pub target_device_id: String,
    pub folder_pattern: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrbSavedSearch {
    pub id: String,
    pub name: String,
    pub query: String,
    pub media_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrbStorageStats {
    pub total_files: u64,
    pub total_bytes: u64,
    pub video_count: u64,
    pub video_bytes: u64,
    pub image_count: u64,
    pub image_bytes: u64,
    pub audio_count: u64,
    pub audio_bytes: u64,
    pub duplicate_groups: u64,
    pub duplicate_extra_bytes: u64,
    pub trashed_count: u64,
    pub trashed_bytes: u64,
    pub large_files_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrbBulkRenamePreview {
    pub media_id: String,
    pub old_file_name: String,
    pub new_file_name: String,
    pub conflict: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrbBulkRenameResult {
    pub batch_id: String,
    pub renamed_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrbThumbnailEntry {
    pub media_id: String,
    pub file_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrbDownloadJob {
    pub id: String,
    pub url: String,
    pub title: Option<String>,
    pub format: String,
    pub quality: String,
    pub destination_folder: String,
    pub status: String,
    pub progress: f64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrbPairingSession {
    pub pin: String,
    pub qr_payload: String,
    pub expires_at: i64,
}

static CORE_INSTANCE: OnceLock<Mutex<Option<Arc<MediaGalleryCore>>>> = OnceLock::new();

fn core_slot() -> &'static Mutex<Option<Arc<MediaGalleryCore>>> {
    CORE_INSTANCE.get_or_init(|| Mutex::new(None))
}

fn get_core() -> Result<Arc<MediaGalleryCore>, ApiError> {
    let guard = core_slot().lock().map_err(|_| ApiError {
        code: ApiErrorCode::Internal,
        message: "core lock poisoned".to_string(),
    })?;
    guard.clone().ok_or_else(|| ApiError {
        code: ApiErrorCode::InvalidInput,
        message: "MediaGalleryCore not initialized; call frb_init_core first".to_string(),
    })
}

fn aspect_to_frb(a: AspectPreset) -> String {
    match a {
        AspectPreset::Ratio4x3 => "ratio4x3".to_string(),
        AspectPreset::Ratio9x16 => "ratio9x16".to_string(),
        AspectPreset::Ratio16x9 => "ratio16x9".to_string(),
        AspectPreset::Ratio1x1 => "ratio1x1".to_string(),
        AspectPreset::Custom => "custom".to_string(),
    }
}

fn parse_aspect_frb(raw: &str) -> AspectPreset {
    match raw {
        "ratio4x3" | "4:3" => AspectPreset::Ratio4x3,
        "ratio9x16" | "9:16" => AspectPreset::Ratio9x16,
        "ratio1x1" | "1:1" => AspectPreset::Ratio1x1,
        "custom" => AspectPreset::Custom,
        _ => AspectPreset::Ratio16x9,
    }
}

fn cover_to_frb(c: CoverRecord) -> FrbCoverItem {
    FrbCoverItem {
        id: c.id,
        media_id: c.media_id,
        file_path: c.file_path,
        aspect: aspect_to_frb(c.aspect),
        frame_time_ms: c.frame_time_ms,
    }
}

fn media_with_cover_to_frb(item: MediaWithCover) -> FrbMediaItem {
    let m = item.media;
    FrbMediaItem {
        id: m.id,
        device_id: m.device_id,
        device_name: item.device_name.unwrap_or_default(),
        device_online: item.device_online,
        path: m.path,
        folder_path: m.folder_path,
        file_name: m.file_name,
        media_type: m.media_type.as_str().to_string(),
        size: m.size,
        created_at: m.created_at,
        duration_ms: m.duration_ms,
        width: m.width,
        height: m.height,
        alias: m.alias,
        auto_alias: m.auto_alias,
        sync_state: m.sync_state.as_str().to_ascii_lowercase(),
        place: m.place,
        artist: m.tags.artist,
        album: m.tags.album,
        cover: item.cover.map(cover_to_frb),
        is_duplicate: m.is_duplicate,
        is_trashed: m.is_trashed,
    }
}

fn media_record_to_frb(index: &MediaIndex, m: MediaRecord) -> FrbMediaItem {
    if let Ok(Some(full)) = index.get_media_by_id(&m.id) {
        return media_with_cover_to_frb(full);
    }
    FrbMediaItem {
        id: m.id,
        device_id: m.device_id,
        device_name: String::new(),
        device_online: true,
        path: m.path,
        folder_path: m.folder_path,
        file_name: m.file_name,
        media_type: m.media_type.as_str().to_string(),
        size: m.size,
        created_at: m.created_at,
        duration_ms: m.duration_ms,
        width: m.width,
        height: m.height,
        alias: m.alias,
        auto_alias: m.auto_alias,
        sync_state: m.sync_state.as_str().to_ascii_lowercase(),
        place: m.place,
        artist: m.tags.artist,
        album: m.tags.album,
        cover: None,
        is_duplicate: m.is_duplicate,
        is_trashed: m.is_trashed,
    }
}

fn transport_to_frb(t: TransportKind) -> String {
    match t {
        TransportKind::Wired => "wired".to_string(),
        TransportKind::WifiLan => "wifiLan".to_string(),
        TransportKind::HolePunched => "holePunched".to_string(),
        TransportKind::Relay => "relay".to_string(),
        TransportKind::Bluetooth => "bluetooth".to_string(),
    }
}

fn parse_transport_frb(raw: &str) -> Option<TransportKind> {
    match raw {
        "wired" => Some(TransportKind::Wired),
        "wifiLan" | "wifi_lan" => Some(TransportKind::WifiLan),
        "holePunched" | "hole_punched" => Some(TransportKind::HolePunched),
        "relay" => Some(TransportKind::Relay),
        "bluetooth" => Some(TransportKind::Bluetooth),
        _ => None,
    }
}

pub fn frb_init_core(
    db_path: Option<String>,
    cache_dir: String,
    self_device_id: String,
    self_device_name: String,
    is_phone: bool,
) -> Result<(), ApiError> {
    let core = MediaGalleryCore::init(CoreConfig {
        db_path: db_path.map(PathBuf::from),
        cache_dir: PathBuf::from(cache_dir),
        self_device_id,
        self_device_name,
        self_device_type: if is_phone {
            DeviceType::Phone
        } else {
            DeviceType::Pc
        },
        max_thumb_cache_bytes: Some(512 * 1024 * 1024),
    })?;
    let mut guard = core_slot().lock().map_err(|_| ApiError {
        code: ApiErrorCode::Internal,
        message: "core lock poisoned".to_string(),
    })?;
    *guard = Some(Arc::new(core));
    Ok(())
}

pub async fn frb_scan_folders(roots: Vec<String>, excluded: Vec<String>) -> Result<u64, ApiError> {
    let core = get_core()?;
    let root_paths: Vec<PathBuf> = roots.into_iter().map(PathBuf::from).collect();
    let excl_paths: Vec<PathBuf> = excluded.into_iter().map(PathBuf::from).collect();
    tokio::task::spawn_blocking(move || core.scan_now(root_paths, excl_paths))
        .await
        .map_err(|e| ApiError {
            code: ApiErrorCode::Internal,
            message: e.to_string(),
        })?
}

pub async fn frb_ingest_android_mediastore(
    device_id: String,
    device_name: String,
    rows_json: String,
) -> Result<u32, ApiError> {
    let core = get_core()?;
    let rows: Vec<AndroidMediaStoreRow> =
        serde_json::from_str(&rows_json).map_err(|e| ApiError {
            code: ApiErrorCode::InvalidInput,
            message: format!("invalid mediastore json: {e}"),
        })?;
    tokio::task::spawn_blocking(move || {
        core.ingest_android_mediastore_batch(&device_id, &device_name, &rows)
    })
    .await
    .map_err(|e| ApiError {
        code: ApiErrorCode::Internal,
        message: e.to_string(),
    })?
}

#[allow(clippy::too_many_arguments)]
pub async fn frb_list_media(
    query: String,
    media_type: Option<String>,
    device_id: Option<String>,
    folder_path: Option<String>,
    sync_state: Option<String>,
    min_size: Option<u64>,
    only_duplicates: bool,
    trashed_only: bool,
    sort_field: String,
    sort_dir: String,
    cursor: Option<String>,
    limit: u32,
) -> Result<FrbMediaPage, ApiError> {
    let core = get_core()?;
    let filter = MediaFilter {
        query,
        media_type: media_type.as_deref().map(MediaType::parse),
        device_id,
        sync_state: sync_state.as_deref().map(SyncState::parse),
        folder_path,
        min_size,
        only_duplicates,
        trashed_only,
        include_trashed: trashed_only,
        ..Default::default()
    };
    let field = match sort_field.as_str() {
        "name" => SortField::Name,
        "size" => SortField::Size,
        "duration" => SortField::Duration,
        "type" => SortField::Type,
        _ => SortField::Date,
    };
    let dir = if sort_dir == "asc" {
        SortDir::Asc
    } else {
        SortDir::Desc
    };
    tokio::task::spawn_blocking(move || {
        let page = core.list_media(&filter, SortSpec { field, dir }, cursor.as_deref(), limit)?;
        Ok(FrbMediaPage {
            items: page
                .items
                .into_iter()
                .map(media_with_cover_to_frb)
                .collect(),
            total_count: page.total_count,
            next_cursor: page.next_cursor,
        })
    })
    .await
    .map_err(|e| ApiError {
        code: ApiErrorCode::Internal,
        message: e.to_string(),
    })?
}

pub async fn frb_list_folders(
    device_id: Option<String>,
    media_type: Option<String>,
) -> Result<Vec<FrbFolderSummary>, ApiError> {
    let core = get_core()?;
    let mt = media_type.as_deref().map(MediaType::parse);
    tokio::task::spawn_blocking(move || {
        let folders = core.list_folders(device_id.as_deref(), mt)?;
        Ok(folders
            .into_iter()
            .map(|f| FrbFolderSummary {
                path: f.path,
                name: f.name,
                device_id: f.device_id,
                item_count: f.item_count,
                preview_covers: f.preview_covers,
            })
            .collect())
    })
    .await
    .map_err(|e| ApiError {
        code: ApiErrorCode::Internal,
        message: e.to_string(),
    })?
}

pub async fn frb_list_devices() -> Result<Vec<FrbDeviceItem>, ApiError> {
    let core = get_core()?;
    tokio::task::spawn_blocking(move || {
        let devs = core.list_devices()?;
        Ok(devs
            .into_iter()
            .map(|d| FrbDeviceItem {
                id: d.id,
                name: d.name,
                device_type: d.device_type.as_str().to_string(),
                transports: d.transports.into_iter().map(transport_to_frb).collect(),
                is_self: d.is_self,
                is_online: d.is_online,
                paired_at: d.paired_at,
            })
            .collect())
    })
    .await
    .map_err(|e| ApiError {
        code: ApiErrorCode::Internal,
        message: e.to_string(),
    })?
}

pub async fn frb_get_storage_stats() -> Result<FrbStorageStats, ApiError> {
    let core = get_core()?;
    tokio::task::spawn_blocking(move || {
        let s = core.get_storage_stats()?;
        Ok(FrbStorageStats {
            total_files: s.total_files,
            total_bytes: s.total_bytes,
            video_count: s.video_count,
            video_bytes: s.video_bytes,
            image_count: s.image_count,
            image_bytes: s.image_bytes,
            audio_count: s.audio_count,
            audio_bytes: s.audio_bytes,
            duplicate_groups: s.duplicate_groups,
            duplicate_extra_bytes: s.duplicate_extra_bytes,
            trashed_count: s.trashed_count,
            trashed_bytes: s.trashed_bytes,
            large_files_count: s.large_files_count,
        })
    })
    .await
    .map_err(|e| ApiError {
        code: ApiErrorCode::Internal,
        message: e.to_string(),
    })?
}

pub async fn frb_ensure_thumbnails_batch(
    media_ids: Vec<String>,
) -> Result<Vec<FrbThumbnailEntry>, ApiError> {
    let core = get_core()?;
    tokio::task::spawn_blocking(move || {
        let entries = core.ensure_thumbnails_batch(&media_ids);
        Ok(entries
            .into_iter()
            .map(|(media_id, file_path)| FrbThumbnailEntry {
                media_id,
                file_path,
            })
            .collect())
    })
    .await
    .map_err(|e| ApiError {
        code: ApiErrorCode::Internal,
        message: e.to_string(),
    })?
}

pub async fn frb_set_alias(media_id: String, alias: String) -> Result<FrbMediaItem, ApiError> {
    let core = get_core()?;
    tokio::task::spawn_blocking(move || {
        let rec = core.set_alias(&media_id, &alias)?;
        Ok(media_record_to_frb(&core.index, rec))
    })
    .await
    .map_err(|e| ApiError {
        code: ApiErrorCode::Internal,
        message: e.to_string(),
    })?
}

pub async fn frb_apply_real_rename(
    media_id: String,
    new_name: String,
) -> Result<FrbMediaItem, ApiError> {
    let core = get_core()?;
    tokio::task::spawn_blocking(move || {
        let rec = core.apply_real_rename(&media_id, &new_name)?;
        Ok(media_record_to_frb(&core.index, rec))
    })
    .await
    .map_err(|e| ApiError {
        code: ApiErrorCode::Internal,
        message: e.to_string(),
    })?
}

pub async fn frb_undo_last_rename(media_id: Option<String>) -> Result<bool, ApiError> {
    let core = get_core()?;
    tokio::task::spawn_blocking(move || Ok(core.undo_last_rename(media_id.as_deref())?.is_some()))
        .await
        .map_err(|e| ApiError {
            code: ApiErrorCode::Internal,
            message: e.to_string(),
        })?
}

pub async fn frb_preview_bulk_rename(
    media_ids: Vec<String>,
    template: String,
) -> Result<Vec<FrbBulkRenamePreview>, ApiError> {
    let core = get_core()?;
    tokio::task::spawn_blocking(move || {
        let list = core.preview_bulk_rename(&media_ids, &template)?;
        Ok(list
            .into_iter()
            .map(|item| {
                let conflict = item.old_path != item.new_path && Path::new(&item.new_path).exists();
                FrbBulkRenamePreview {
                    media_id: item.media_id,
                    old_file_name: item.old_name,
                    new_file_name: item.new_name,
                    conflict,
                }
            })
            .collect())
    })
    .await
    .map_err(|e| ApiError {
        code: ApiErrorCode::Internal,
        message: e.to_string(),
    })?
}

pub async fn frb_apply_bulk_rename(
    media_ids: Vec<String>,
    template: String,
) -> Result<FrbBulkRenameResult, ApiError> {
    let core = get_core()?;
    tokio::task::spawn_blocking(move || {
        let (batch_id, logs) = core.apply_bulk_rename(&media_ids, &template)?;
        Ok(FrbBulkRenameResult {
            batch_id,
            renamed_count: logs.len() as u32,
        })
    })
    .await
    .map_err(|e| ApiError {
        code: ApiErrorCode::Internal,
        message: e.to_string(),
    })?
}

pub async fn frb_undo_bulk_rename(batch_id: String) -> Result<u32, ApiError> {
    let core = get_core()?;
    tokio::task::spawn_blocking(move || core.undo_bulk_rename(&batch_id))
        .await
        .map_err(|e| ApiError {
            code: ApiErrorCode::Internal,
            message: e.to_string(),
        })?
}

pub async fn frb_move_media_to_folder(
    media_id: String,
    target_folder: String,
) -> Result<FrbMediaItem, ApiError> {
    let core = get_core()?;
    tokio::task::spawn_blocking(move || {
        let rec = core.move_media_to_folder(&media_id, Path::new(&target_folder))?;
        Ok(media_record_to_frb(&core.index, rec))
    })
    .await
    .map_err(|e| ApiError {
        code: ApiErrorCode::Internal,
        message: e.to_string(),
    })?
}

pub async fn frb_move_to_trash(media_id: String) -> Result<String, ApiError> {
    let core = get_core()?;
    tokio::task::spawn_blocking(move || core.move_to_trash(&media_id))
        .await
        .map_err(|e| ApiError {
            code: ApiErrorCode::Internal,
            message: e.to_string(),
        })?
}

pub async fn frb_restore_from_trash(media_id: String) -> Result<FrbMediaItem, ApiError> {
    let core = get_core()?;
    tokio::task::spawn_blocking(move || {
        let rec = core.restore_from_trash(&media_id)?;
        Ok(media_record_to_frb(&core.index, rec))
    })
    .await
    .map_err(|e| ApiError {
        code: ApiErrorCode::Internal,
        message: e.to_string(),
    })?
}

pub async fn frb_empty_trash(media_id: Option<String>) -> Result<u64, ApiError> {
    let core = get_core()?;
    tokio::task::spawn_blocking(move || core.empty_trash(media_id.as_deref()))
        .await
        .map_err(|e| ApiError {
            code: ApiErrorCode::Internal,
            message: e.to_string(),
        })?
}

pub async fn frb_update_cover_aspect(
    media_id: String,
    aspect: String,
) -> Result<FrbCoverItem, ApiError> {
    let core = get_core()?;
    let preset = parse_aspect_frb(&aspect);
    tokio::task::spawn_blocking(move || {
        let cov = core.update_cover_aspect(&media_id, preset)?;
        Ok(cover_to_frb(cov))
    })
    .await
    .map_err(|e| ApiError {
        code: ApiErrorCode::Internal,
        message: e.to_string(),
    })?
}

pub async fn frb_reroll_video_thumbnail(media_id: String) -> Result<FrbCoverItem, ApiError> {
    let core = get_core()?;
    tokio::task::spawn_blocking(move || {
        let cov = core.reroll_video_thumbnail(&media_id)?;
        Ok(cover_to_frb(cov))
    })
    .await
    .map_err(|e| ApiError {
        code: ApiErrorCode::Internal,
        message: e.to_string(),
    })?
}

pub async fn frb_pick_video_frame(
    media_id: String,
    frame_time_ms: u64,
) -> Result<FrbCoverItem, ApiError> {
    let core = get_core()?;
    tokio::task::spawn_blocking(move || {
        let cov = core.pick_video_frame(&media_id, frame_time_ms)?;
        Ok(cover_to_frb(cov))
    })
    .await
    .map_err(|e| ApiError {
        code: ApiErrorCode::Internal,
        message: e.to_string(),
    })?
}

pub async fn frb_set_custom_cover(
    media_id: String,
    image_path: String,
    aspect: String,
    confirmed_embedded_write: bool,
) -> Result<FrbCoverItem, ApiError> {
    let core = get_core()?;
    let preset = parse_aspect_frb(&aspect);
    tokio::task::spawn_blocking(move || {
        let cov = core.set_custom_cover(
            &media_id,
            Path::new(&image_path),
            preset,
            None,
            confirmed_embedded_write,
        )?;
        Ok(cover_to_frb(cov))
    })
    .await
    .map_err(|e| ApiError {
        code: ApiErrorCode::Internal,
        message: e.to_string(),
    })?
}

pub async fn frb_list_pinned_folders() -> Result<Vec<FrbPinnedFolder>, ApiError> {
    let core = get_core()?;
    tokio::task::spawn_blocking(move || {
        let list = core.list_pinned_folders()?;
        Ok(list
            .into_iter()
            .map(|p| FrbPinnedFolder {
                id: p.id,
                device_id: p.device_id,
                path: p.path,
                label: p.label,
                icon: p.icon,
                position: p.position,
            })
            .collect())
    })
    .await
    .map_err(|e| ApiError {
        code: ApiErrorCode::Internal,
        message: e.to_string(),
    })?
}

pub async fn frb_upsert_pinned_folder(folder: FrbPinnedFolder) -> Result<(), ApiError> {
    let core = get_core()?;
    tokio::task::spawn_blocking(move || {
        core.upsert_pinned_folder(&PinnedFolderRecord {
            id: folder.id,
            device_id: folder.device_id,
            path: folder.path,
            label: folder.label,
            icon: folder.icon,
            position: folder.position,
        })
    })
    .await
    .map_err(|e| ApiError {
        code: ApiErrorCode::Internal,
        message: e.to_string(),
    })?
}

pub async fn frb_remove_pinned_folder(id: String) -> Result<(), ApiError> {
    let core = get_core()?;
    tokio::task::spawn_blocking(move || core.remove_pinned_folder(&id))
        .await
        .map_err(|e| ApiError {
            code: ApiErrorCode::Internal,
            message: e.to_string(),
        })?
}

pub async fn frb_reorder_pinned_folders(ordered_ids: Vec<String>) -> Result<(), ApiError> {
    let core = get_core()?;
    tokio::task::spawn_blocking(move || core.reorder_pinned_folders(&ordered_ids))
        .await
        .map_err(|e| ApiError {
            code: ApiErrorCode::Internal,
            message: e.to_string(),
        })?
}

pub async fn frb_create_pairing_session() -> Result<FrbPairingSession, ApiError> {
    let core = get_core()?;
    let addr_json = match core.ensure_iroh_node().await {
        Ok((node, _)) => node.addr_json().unwrap_or_else(|_| "{}".to_string()),
        Err(_) => "{}".to_string(),
    };
    tokio::task::spawn_blocking(move || {
        let self_dev = core
            .index
            .get_device(&core.self_device_id)?
            .ok_or_else(|| ApiError {
                code: ApiErrorCode::NotFound,
                message: "self device missing".into(),
            })?;
        let s = core
            .pairing
            .create_session(&self_dev, &addr_json, 300_000)?;
        Ok(FrbPairingSession {
            pin: s.pin,
            qr_payload: s.qr_payload,
            expires_at: s.expires_at,
        })
    })
    .await
    .map_err(|e| ApiError {
        code: ApiErrorCode::Internal,
        message: e.to_string(),
    })?
}

fn device_to_frb(d: DeviceRecord) -> FrbDeviceItem {
    FrbDeviceItem {
        id: d.id,
        name: d.name,
        device_type: d.device_type.as_str().to_string(),
        transports: d.transports.into_iter().map(transport_to_frb).collect(),
        is_self: d.is_self,
        is_online: d.is_online,
        paired_at: d.paired_at,
    }
}

fn transfer_to_frb(t: TransferRecord) -> FrbTransferItem {
    FrbTransferItem {
        id: t.id,
        media_id: t.media_id,
        file_name: t.file_name,
        from_device: t.from_device,
        to_device: t.to_device,
        status: t.status.as_str().to_string(),
        bytes_done: t.bytes_done,
        bytes_total: t.bytes_total,
        path_kind: t.path_kind,
        speed_bps: t.speed_bps,
        error: t.error,
    }
}

pub async fn frb_pair_with_device(
    payload_or_pin: String,
    peer_name: Option<String>,
) -> Result<FrbDeviceItem, ApiError> {
    let core = get_core()?;
    let trimmed = payload_or_pin.trim().to_string();
    if trimmed.starts_with('{') {
        let qr: QrPairingData = serde_json::from_str(&trimmed).map_err(|e| ApiError {
            code: ApiErrorCode::InvalidInput,
            message: format!("invalid pairing QR payload: {e}"),
        })?;
        if let Ok(addr) = serde_json::from_str::<EndpointAddr>(&qr.endpoint_addr_json) {
            if let Ok((_, transport)) = core.ensure_iroh_node().await {
                transport.register_peer_addr(&qr.device_id, addr).await;
            }
        }
        let core_clone = Arc::clone(&core);
        let dev = tokio::task::spawn_blocking(move || {
            let mut override_dev = None;
            if let Some(name) = peer_name.filter(|s| !s.trim().is_empty()) {
                override_dev = Some(DeviceRecord {
                    id: qr.device_id.clone(),
                    name: name.trim().to_string(),
                    device_type: qr.device_type,
                    public_key: qr.public_key.clone(),
                    last_seen: now_ms(),
                    transports: vec![TransportKind::WifiLan, TransportKind::Relay],
                    host: qr.host.clone(),
                    port: qr.port,
                    is_self: false,
                    is_online: true,
                    paired_at: Some(now_ms()),
                });
            }
            Ok::<DeviceRecord, ApiError>(core_clone.pairing.pair_with_qr(&trimmed, override_dev)?)
        })
        .await
        .map_err(|e| ApiError {
            code: ApiErrorCode::Internal,
            message: e.to_string(),
        })??;
        return Ok(device_to_frb(dev));
    }

    let remote = DeviceRecord {
        id: format!("peer-{}", trimmed),
        name: peer_name
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| format!("Paired Device ({trimmed})")),
        device_type: DeviceType::Phone,
        public_key: format!("pin:{trimmed}"),
        last_seen: now_ms(),
        transports: vec![
            TransportKind::WifiLan,
            TransportKind::HolePunched,
            TransportKind::Relay,
        ],
        host: None,
        port: None,
        is_self: false,
        is_online: true,
        paired_at: Some(now_ms()),
    };
    let dev = tokio::task::spawn_blocking(move || core.pair_with_pin(&trimmed, remote))
        .await
        .map_err(|e| ApiError {
            code: ApiErrorCode::Internal,
            message: e.to_string(),
        })??;
    Ok(device_to_frb(dev))
}

pub async fn frb_list_duplicate_groups() -> Result<Vec<FrbDuplicateGroup>, ApiError> {
    let core = get_core()?;
    tokio::task::spawn_blocking(move || {
        let groups = core.detect_duplicates(None)?;
        Ok(groups
            .into_iter()
            .map(|g| {
                let reclaimable_bytes = g
                    .size
                    .saturating_mul((g.items.len() as u64).saturating_sub(1));
                FrbDuplicateGroup {
                    group_id: g.group_id,
                    size: g.size,
                    reclaimable_bytes,
                    items: g.items.into_iter().map(media_with_cover_to_frb).collect(),
                }
            })
            .collect())
    })
    .await
    .map_err(|e| ApiError {
        code: ApiErrorCode::Internal,
        message: e.to_string(),
    })?
}

pub async fn frb_clean_duplicates(keep_newest: bool) -> Result<FrbCleanupReport, ApiError> {
    let core = get_core()?;
    tokio::task::spawn_blocking(move || {
        let groups = core.detect_duplicates(None)?;
        let mut cleaned_count = 0u64;
        let mut freed_bytes = 0u64;
        for group in groups {
            if group.items.len() <= 1 {
                continue;
            }
            let mut sorted = group.items;
            sorted.sort_by_key(|m| m.media.created_at);
            if keep_newest {
                sorted.reverse();
            }
            for redundant in sorted.into_iter().skip(1) {
                if core.move_to_trash(&redundant.media.id).is_ok() {
                    cleaned_count += 1;
                    freed_bytes += redundant.media.size;
                }
            }
        }
        let _ = core.detect_duplicates(None);
        Ok(FrbCleanupReport {
            cleaned_count,
            freed_bytes,
        })
    })
    .await
    .map_err(|e| ApiError {
        code: ApiErrorCode::Internal,
        message: e.to_string(),
    })?
}

pub async fn frb_list_unified_music(group_by: String) -> Result<Vec<FrbMusicGroup>, ApiError> {
    let core = get_core()?;
    let gb = match group_by.as_str() {
        "artist" => MusicGroupBy::Artist,
        "album" => MusicGroupBy::Album,
        "folder" => MusicGroupBy::Folder,
        "device" => MusicGroupBy::Device,
        _ => MusicGroupBy::All,
    };
    tokio::task::spawn_blocking(move || {
        let groups = core.list_unified_music(gb)?;
        Ok(groups
            .into_iter()
            .map(|(group_name, tracks)| FrbMusicGroup {
                group_name,
                tracks: tracks
                    .into_iter()
                    .map(|t| FrbUnifiedTrack {
                        canonical_key: t.canonical_key,
                        title: t.title,
                        artist: t.artist,
                        album: t.album,
                        duration_ms: t.duration_ms,
                        primary: media_with_cover_to_frb(t.primary),
                        source_devices: t.sources.into_iter().map(|s| s.device_name).collect(),
                    })
                    .collect(),
            })
            .collect())
    })
    .await
    .map_err(|e| ApiError {
        code: ApiErrorCode::Internal,
        message: e.to_string(),
    })?
}

pub async fn frb_send_media_to_device(
    media_id: String,
    target_device_id: String,
) -> Result<FrbTransferItem, ApiError> {
    let core = get_core()?;
    let item = core
        .index
        .get_media_by_id(&media_id)?
        .ok_or_else(|| ApiError {
            code: ApiErrorCode::NotFound,
            message: format!("media not found: {media_id}"),
        })?
        .media;
    let target = core
        .index
        .get_device(&target_device_id)?
        .ok_or_else(|| ApiError {
            code: ApiErrorCode::NotFound,
            message: format!("target device not found: {target_device_id}"),
        })?;
    if !target.is_online {
        return Err(ApiError {
            code: ApiErrorCode::PeerOffline,
            message: format!("{} is currently offline", target.name),
        });
    }

    let tr = TransferRecord {
        id: Uuid::new_v4().to_string(),
        media_id: item.id.clone(),
        file_name: item.file_name.clone(),
        from_device: core.self_device_id.clone(),
        to_device: target.id.clone(),
        status: TransferStatus::Queued,
        bytes_done: 0,
        bytes_total: item.size,
        last_chunk_index: 0,
        chunk_size: 262_144,
        path_kind: "WifiLan".to_string(),
        speed_bps: 0,
        expected_blake3: item.blake3.clone(),
        error: None,
        updated_at: now_ms(),
    };
    core.index.upsert_transfer(&tr)?;

    if let Ok((_, transport)) = core.ensure_iroh_node().await {
        let engine = TransferEngine::new(core.index.clone(), Some(Arc::clone(&core.pairing)));
        let core_bg = Arc::clone(&core);
        let item_bg = item.clone();
        let target_bg = target.clone();
        let tr_id = tr.id.clone();
        tokio::spawn(async move {
            let res = engine
                .send_file(
                    &item_bg,
                    &core_bg.self_device_id,
                    &target_bg,
                    transport.as_ref(),
                    None,
                    None,
                    |prog| {
                        let _ = core_bg
                            .events_tx
                            .send(CoreEvent::TransferProgress(prog.clone()));
                    },
                )
                .await;
            if let Err(err) = res {
                if let Ok(Some(mut rec)) = core_bg.index.get_transfer(&tr_id) {
                    rec.status = TransferStatus::Error;
                    rec.error = Some(err.to_string());
                    rec.updated_at = now_ms();
                    let _ = core_bg.index.upsert_transfer(&rec);
                }
            }
        });
    }

    Ok(transfer_to_frb(tr))
}

pub async fn frb_download_remote_media(media_id: String) -> Result<FrbTransferItem, ApiError> {
    let core = get_core()?;
    let owner = core.assert_remote_download_allowed(&media_id)?;
    let item = core
        .index
        .get_media_by_id(&media_id)?
        .ok_or_else(|| ApiError {
            code: ApiErrorCode::NotFound,
            message: format!("media not found: {media_id}"),
        })?
        .media;
    let tr = TransferRecord {
        id: Uuid::new_v4().to_string(),
        media_id: item.id,
        file_name: item.file_name,
        from_device: owner.id,
        to_device: core.self_device_id.clone(),
        status: TransferStatus::Queued,
        bytes_done: 0,
        bytes_total: item.size,
        last_chunk_index: 0,
        chunk_size: 262_144,
        path_kind: "WifiLan".to_string(),
        speed_bps: 0,
        expected_blake3: item.blake3,
        error: None,
        updated_at: now_ms(),
    };
    core.index.upsert_transfer(&tr)?;
    Ok(transfer_to_frb(tr))
}

pub async fn frb_backup_local_media_to_device(target_device_id: String) -> Result<u32, ApiError> {
    let core = get_core()?;
    let page = core.index.list_media(
        &MediaFilter {
            device_id: Some(core.self_device_id.clone()),
            sync_state: Some(SyncState::Local),
            ..Default::default()
        },
        SortSpec::default(),
        None,
        500,
    )?;
    let mut queued = 0u32;
    for entry in page.items {
        if frb_send_media_to_device(entry.media.id, target_device_id.clone())
            .await
            .is_ok()
        {
            queued += 1;
        }
    }
    Ok(queued)
}

pub async fn frb_list_transfers() -> Result<Vec<FrbTransferItem>, ApiError> {
    let core = get_core()?;
    tokio::task::spawn_blocking(move || {
        let list = core.index.list_transfers()?;
        Ok(list.into_iter().map(transfer_to_frb).collect())
    })
    .await
    .map_err(|e| ApiError {
        code: ApiErrorCode::Internal,
        message: e.to_string(),
    })?
}

pub async fn frb_pause_transfer(transfer_id: String) -> Result<(), ApiError> {
    let core = get_core()?;
    let engine = TransferEngine::new(core.index.clone(), Some(Arc::clone(&core.pairing)));
    Ok(engine.pause_transfer(&transfer_id)?)
}

pub async fn frb_cancel_transfer(transfer_id: String) -> Result<(), ApiError> {
    let core = get_core()?;
    let engine = TransferEngine::new(core.index.clone(), Some(Arc::clone(&core.pairing)));
    Ok(engine.cancel_transfer(&transfer_id)?)
}

pub async fn frb_list_auto_sync_rules() -> Result<Vec<FrbAutoSyncRule>, ApiError> {
    let core = get_core()?;
    tokio::task::spawn_blocking(move || {
        let rules = core.index.list_auto_sync_rules()?;
        Ok(rules
            .into_iter()
            .map(|r| FrbAutoSyncRule {
                id: r.id,
                name: r.name,
                source_device_id: r.source_device_id,
                target_device_id: r.target_device_id,
                folder_pattern: r.folder_pattern,
                enabled: r.enabled,
            })
            .collect())
    })
    .await
    .map_err(|e| ApiError {
        code: ApiErrorCode::Internal,
        message: e.to_string(),
    })?
}

pub async fn frb_upsert_auto_sync_rule(
    id: String,
    name: String,
    target_device_id: String,
    folder_pattern: String,
    enabled: bool,
) -> Result<(), ApiError> {
    let core = get_core()?;
    tokio::task::spawn_blocking(move || {
        let rule = AutoSyncRuleRecord {
            id,
            name,
            source_device_id: core.self_device_id.clone(),
            target_device_id,
            folder_pattern,
            media_types: vec![MediaType::Image, MediaType::Video],
            enabled,
            created_at: now_ms(),
        };
        Ok(core.index.upsert_auto_sync_rule(&rule)?)
    })
    .await
    .map_err(|e| ApiError {
        code: ApiErrorCode::Internal,
        message: e.to_string(),
    })?
}

pub async fn frb_list_saved_searches() -> Result<Vec<FrbSavedSearch>, ApiError> {
    let core = get_core()?;
    tokio::task::spawn_blocking(move || {
        let list = core.list_saved_searches()?;
        Ok(list
            .into_iter()
            .map(|s| FrbSavedSearch {
                id: s.id,
                name: s.name,
                query: s.query,
                media_type: s.filter.media_type.map(|m| m.as_str().to_string()),
            })
            .collect())
    })
    .await
    .map_err(|e| ApiError {
        code: ApiErrorCode::Internal,
        message: e.to_string(),
    })?
}

pub async fn frb_save_search(
    name: String,
    query: String,
    media_type: Option<String>,
) -> Result<(), ApiError> {
    let core = get_core()?;
    tokio::task::spawn_blocking(move || {
        let saved = SavedSearchRecord {
            id: Uuid::new_v4().to_string(),
            name,
            query: query.clone(),
            filter: MediaFilter {
                query,
                media_type: media_type.as_deref().map(MediaType::parse),
                ..Default::default()
            },
            created_at: now_ms(),
        };
        core.save_search(&saved)
    })
    .await
    .map_err(|e| ApiError {
        code: ApiErrorCode::Internal,
        message: e.to_string(),
    })?
}

pub async fn frb_forget_device(device_id: String) -> Result<(), ApiError> {
    let core = get_core()?;
    tokio::task::spawn_blocking(move || core.forget_device(&device_id))
        .await
        .map_err(|e| ApiError {
            code: ApiErrorCode::Internal,
            message: e.to_string(),
        })?
}

pub async fn frb_enqueue_youtube_download(
    url: String,
    format: String,
    quality: String,
    destination_folder: String,
) -> Result<FrbDownloadJob, ApiError> {
    let core = get_core()?;
    let fmt = if format.eq_ignore_ascii_case("mp3") {
        DownloadFormat::Mp3
    } else {
        DownloadFormat::Mp4
    };
    let resolved_dest = {
        let p = PathBuf::from(&destination_folder);
        if p.is_relative() {
            let home = std::env::var_os("USERPROFILE")
                .or_else(|| std::env::var_os("HOME"))
                .map(PathBuf::from)
                .unwrap_or_else(std::env::temp_dir);
            home.join(p).to_string_lossy().to_string()
        } else {
            destination_folder
        }
    };
    tokio::task::spawn_blocking(move || {
        let req = YoutubeDownloadRequest {
            url,
            format: fmt,
            quality,
            destination_device_id: core.self_device_id.clone(),
            destination_folder: resolved_dest,
            allow_playlist: false,
            ffmpeg_bin: None,
        };
        let job = core.enqueue_youtube_download(&req)?;
        Ok(FrbDownloadJob {
            id: job.id,
            url: job.url,
            title: job.title,
            format: if job.format == DownloadFormat::Mp3 {
                "mp3".to_string()
            } else {
                "mp4".to_string()
            },
            quality: job.quality,
            destination_folder: job.destination_folder,
            status: format!("{:?}", job.status).to_ascii_lowercase(),
            progress: f64::from(job.progress),
            error: job.error,
        })
    })
    .await
    .map_err(|e| ApiError {
        code: ApiErrorCode::Internal,
        message: e.to_string(),
    })?
}

pub async fn frb_list_download_jobs() -> Result<Vec<FrbDownloadJob>, ApiError> {
    let core = get_core()?;
    tokio::task::spawn_blocking(move || {
        let jobs = core.list_download_jobs()?;
        Ok(jobs
            .into_iter()
            .map(|job| FrbDownloadJob {
                id: job.id,
                url: job.url,
                title: job.title,
                format: if job.format == DownloadFormat::Mp3 {
                    "mp3".to_string()
                } else {
                    "mp4".to_string()
                },
                quality: job.quality,
                destination_folder: job.destination_folder,
                status: format!("{:?}", job.status).to_ascii_lowercase(),
                progress: f64::from(job.progress),
                error: job.error,
            })
            .collect())
    })
    .await
    .map_err(|e| ApiError {
        code: ApiErrorCode::Internal,
        message: e.to_string(),
    })?
}

pub async fn frb_get_settings() -> Result<FrbAppSettings, ApiError> {
    let core = get_core()?;
    tokio::task::spawn_blocking(move || {
        let s = core.get_settings()?;
        Ok(FrbAppSettings {
            sidebar_width: s.sidebar_width,
            sidebar_collapsed: s.sidebar_collapsed,
            dark_mode: s.theme != "light",
            manual_transport_override: s.manual_transport_override.map(transport_to_frb),
            alias_template: s.alias_template,
            scan_folders: s.scan_folders,
            excluded_folders: s.excluded_folders,
        })
    })
    .await
    .map_err(|e| ApiError {
        code: ApiErrorCode::Internal,
        message: e.to_string(),
    })?
}

pub async fn frb_save_settings(settings: FrbAppSettings) -> Result<(), ApiError> {
    let core = get_core()?;
    tokio::task::spawn_blocking(move || {
        let mut current = core.get_settings()?;
        current.sidebar_width = settings.sidebar_width;
        current.sidebar_collapsed = settings.sidebar_collapsed;
        current.theme = if settings.dark_mode {
            "dark".to_string()
        } else {
            "light".to_string()
        };
        current.manual_transport_override = settings
            .manual_transport_override
            .as_deref()
            .and_then(parse_transport_frb);
        current.alias_template = settings.alias_template;
        current.scan_folders = settings.scan_folders;
        current.excluded_folders = settings.excluded_folders;
        core.save_settings(&current)?;
        let prefs = core_platform::load_extended_prefs(&core.cache_dir);
        let _ = core_platform::set_windows_startup(prefs.open_at_startup);
        Ok(())
    })
    .await
    .map_err(|e| ApiError {
        code: ApiErrorCode::Internal,
        message: e.to_string(),
    })?
}

#[flutter_rust_bridge::frb(sync)]
pub fn frb_health_check() -> String {
    "core-api v0.2.1 ready".to_string()
}

#[flutter_rust_bridge::frb(sync)]
pub fn frb_clean_alias_name(file_name: String) -> String {
    core_media::clean_file_name_for_alias(&file_name)
}

#[flutter_rust_bridge::frb(sync)]
pub fn frb_select_first_third_frame_ms(duration_ms: u64, sample: Option<f64>) -> u64 {
    core_media::select_random_first_third_frame_ms(duration_ms, sample)
}

pub fn frb_detect_legacy_database(legacy_db_path: String) -> bool {
    Path::new(&legacy_db_path).exists()
}

pub fn frb_import_legacy_database(
    target_db_path: String,
    legacy_db_path: String,
) -> Result<FrbLegacyImportReport, ApiError> {
    let target = MediaIndex::open(Path::new(&target_db_path))?;
    let report = import_legacy_sqlite(&target, Path::new(&legacy_db_path))?;
    Ok(FrbLegacyImportReport {
        imported_devices: report.imported_devices,
        imported_media: report.imported_media,
        imported_covers: report.imported_covers,
        imported_pinned_folders: report.imported_pinned_folders,
        imported_rename_logs: report.imported_rename_logs,
        skipped_items: report.skipped_items,
        auth_relogin_required: report.auth_relogin_required,
        errors: report.errors,
    })
}
