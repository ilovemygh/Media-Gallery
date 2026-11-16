//! Serverless index delta synchronization, remote item states, offline owner checks,
//! LAN stream tickets, auto-sync rules, and optional Google Drive appDataFolder cache.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use core_index::{
    now_ms, DeviceRecord, IndexDelta, IndexError, MediaFilter, MediaIndex, MediaWithCover,
    SyncState,
};
use core_platform::{PlatformError, SecureStore};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const GOOGLE_OAUTH_SCOPES: &[&str] = &[
    "openid",
    "email",
    "profile",
    "https://www.googleapis.com/auth/drive.appdata",
    "https://www.googleapis.com/auth/youtube.readonly",
];

#[derive(Debug, Error)]
pub enum SyncError {
    #[error("index error: {0}")]
    Index(#[from] IndexError),
    #[error("platform error: {0}")]
    Platform(#[from] PlatformError),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("media item not found: {0}")]
    NotFound(String),
    #[error("owning device \"{0}\" is offline; cached thumbnail remains browsable, but Download is disabled until reconnected")]
    PeerOffline(String),
    #[error("Google login required for cloud cache operation")]
    AuthRequired,
}

pub struct IndexSyncManager<'a> {
    pub index: &'a MediaIndex,
}

impl<'a> IndexSyncManager<'a> {
    pub fn new(index: &'a MediaIndex) -> Self {
        Self { index }
    }

    /// Exports only metadata and small cached thumbnails (never media file contents, PRD E1).
    pub fn export_local_delta(
        &self,
        self_device_id: &str,
        since_timestamp: i64,
        include_thumbnails: bool,
    ) -> Result<IndexDelta, SyncError> {
        let mut delta = self
            .index
            .export_delta_since(self_device_id, since_timestamp)?;
        if include_thumbnails {
            for cover in &delta.upserted_covers {
                if cover.file_path.is_empty() {
                    continue;
                }
                if let Ok(bytes) = std::fs::read(&cover.file_path) {
                    let ext = Path::new(&cover.file_path)
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("png")
                        .to_ascii_lowercase();
                    let mime = match ext.as_str() {
                        "svg" => "image/svg+xml",
                        "jpg" | "jpeg" => "image/jpeg",
                        "webp" => "image/webp",
                        _ => "image/png",
                    };
                    delta.thumbnail_blobs_base64.insert(
                        cover.media_id.clone(),
                        format!("data:{mime};base64,{}", BASE64.encode(bytes)),
                    );
                }
            }
        }
        Ok(delta)
    }

    /// Imports peer metadata + cached thumbnails, keeping device ownership authoritative (PRD 11.4).
    pub fn import_remote_delta(
        &self,
        mut delta: IndexDelta,
        self_device_id: &str,
        remote_thumb_cache_dir: &Path,
    ) -> Result<u64, SyncError> {
        std::fs::create_dir_all(remote_thumb_cache_dir)?;

        for cover in &mut delta.upserted_covers {
            if let Some(data_uri) = delta.thumbnail_blobs_base64.get(&cover.media_id) {
                let (header, b64) = data_uri.split_once(',').unwrap_or(("", data_uri.as_str()));
                if let Ok(bytes) = BASE64.decode(b64) {
                    let ext = if header.contains("svg") {
                        "svg"
                    } else if header.contains("jpeg") {
                        "jpg"
                    } else if header.contains("webp") {
                        "webp"
                    } else {
                        "png"
                    };
                    let local_path =
                        remote_thumb_cache_dir.join(format!("{}.{ext}", cover.media_id));
                    std::fs::write(&local_path, bytes)?;
                    cover.file_path = local_path.to_string_lossy().replace('\\', "/");
                }
            }
        }

        Ok(self.index.apply_remote_delta(&delta, self_device_id)?)
    }

    /// Verifies remote item owner is reachable before starting an on-demand download (PRD E2, E6).
    pub fn assert_download_allowed(
        &self,
        media_id: &str,
    ) -> Result<(MediaWithCover, DeviceRecord), SyncError> {
        let item = self
            .index
            .get_media_by_id(media_id)?
            .ok_or_else(|| SyncError::NotFound(media_id.to_string()))?;
        let owner = self
            .index
            .get_device(&item.media.device_id)?
            .ok_or_else(|| SyncError::PeerOffline("unknown device".into()))?;
        if !owner.is_online {
            return Err(SyncError::PeerOffline(owner.name));
        }
        Ok((item, owner))
    }

    /// Produces a streaming preview descriptor for remote LAN items without saving locally (PRD E4).
    pub fn build_stream_preview_ticket(&self, media_id: &str) -> Result<String, SyncError> {
        let (item, owner) = self.assert_download_allowed(media_id)?;
        let host = owner.host.unwrap_or_else(|| "127.0.0.1".to_string());
        let port = owner.port.unwrap_or(9443);
        Ok(format!(
            "iroh://{host}:{port}/files/1/stream/{}",
            item.media.id
        ))
    }

    /// Evaluates enabled auto-sync rules to select pending `Remote` items (PRD E3).
    pub fn find_pending_auto_sync_items(
        &self,
        self_device_id: &str,
    ) -> Result<Vec<MediaWithCover>, SyncError> {
        let rules = self.index.list_auto_sync_rules()?;
        let enabled: Vec<_> = rules
            .into_iter()
            .filter(|r| r.enabled && r.target_device_id == self_device_id)
            .collect();
        if enabled.is_empty() {
            return Ok(Vec::new());
        }
        let remote_page = self.index.list_media(
            &MediaFilter {
                sync_state: Some(SyncState::Remote),
                ..Default::default()
            },
            Default::default(),
            None,
            10_000,
        )?;
        Ok(remote_page
            .items
            .into_iter()
            .filter(|item| {
                enabled.iter().any(|rule| {
                    rule.source_device_id == item.media.device_id
                        && rule.media_types.contains(&item.media.media_type)
                        && item
                            .media
                            .folder_path
                            .to_ascii_lowercase()
                            .contains(&rule.folder_pattern.to_ascii_lowercase())
                })
            })
            .collect())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleSession {
    pub email: String,
    pub access_token: String,
    pub expires_at: i64,
}

pub trait DriveAppDataBackend: Send + Sync {
    fn upload_json(
        &self,
        file_name: &str,
        json_payload: &str,
        access_token: &str,
    ) -> Result<String, SyncError>;
    fn list_files(&self, access_token: &str) -> Result<Vec<String>, SyncError>;
    fn download_json(&self, file_name: &str, access_token: &str) -> Result<String, SyncError>;
}

#[derive(Default)]
pub struct InMemoryDriveBackend {
    files: Mutex<HashMap<String, String>>,
}

impl DriveAppDataBackend for InMemoryDriveBackend {
    fn upload_json(
        &self,
        file_name: &str,
        json_payload: &str,
        _access_token: &str,
    ) -> Result<String, SyncError> {
        self.files
            .lock()
            .map_err(|_| SyncError::AuthRequired)?
            .insert(file_name.to_string(), json_payload.to_string());
        Ok(file_name.to_string())
    }

    fn list_files(&self, _access_token: &str) -> Result<Vec<String>, SyncError> {
        let guard = self.files.lock().map_err(|_| SyncError::AuthRequired)?;
        Ok(guard.keys().cloned().collect())
    }

    fn download_json(&self, file_name: &str, _access_token: &str) -> Result<String, SyncError> {
        let guard = self.files.lock().map_err(|_| SyncError::AuthRequired)?;
        guard
            .get(file_name)
            .cloned()
            .ok_or_else(|| SyncError::NotFound(file_name.to_string()))
    }
}

const SESSION_KEY: &str = "google:oauth:session";
const CLOUD_META_KEY: &str = "google:drive:last_sync";

/// Optional Google Drive `appDataFolder` sync client (PRD E5, H1, H2).
pub struct GoogleCloudSyncClient {
    secure_store: Arc<dyn SecureStore>,
    index: MediaIndex,
    backend: Arc<dyn DriveAppDataBackend>,
}

impl GoogleCloudSyncClient {
    pub fn new(
        secure_store: Arc<dyn SecureStore>,
        index: MediaIndex,
        backend: Arc<dyn DriveAppDataBackend>,
    ) -> Self {
        Self {
            secure_store,
            index,
            backend,
        }
    }

    pub fn save_login_session(&self, session: &GoogleSession) -> Result<(), SyncError> {
        let raw = serde_json::to_string(session)?;
        self.secure_store.set_secret(SESSION_KEY, &raw)?;
        Ok(())
    }

    pub fn get_session(&self) -> Result<Option<GoogleSession>, SyncError> {
        let Some(raw) = self.secure_store.get_secret(SESSION_KEY)? else {
            return Ok(None);
        };
        Ok(serde_json::from_str(&raw).ok())
    }

    pub fn push_index_delta(&self, delta: &IndexDelta) -> Result<String, SyncError> {
        let session = self.get_session()?.ok_or(SyncError::AuthRequired)?;
        let file_name = format!("index-delta-{}.json", delta.device.id);
        let serialized = serde_json::to_string(delta)?;
        let id = self
            .backend
            .upload_json(&file_name, &serialized, &session.access_token)?;
        self.index
            .set_setting(CLOUD_META_KEY, &format!("{}:{id}", now_ms()))?;
        Ok(id)
    }

    pub fn pull_peer_deltas(&self, self_device_id: &str) -> Result<Vec<IndexDelta>, SyncError> {
        let session = self.get_session()?.ok_or(SyncError::AuthRequired)?;
        let files = self.backend.list_files(&session.access_token)?;
        let mut out = Vec::new();
        for name in files {
            if !name.starts_with("index-delta-") || name.contains(self_device_id) {
                continue;
            }
            let raw = self.backend.download_json(&name, &session.access_token)?;
            out.push(serde_json::from_str(&raw)?);
        }
        Ok(out)
    }

    /// Logout wipes tokens from secure store and removes cached offline cloud peers/items (PRD H2).
    pub fn logout_and_wipe(&self, self_device_id: &str) -> Result<(), SyncError> {
        self.secure_store.delete_secret(SESSION_KEY)?;
        self.index.set_setting(CLOUD_META_KEY, "")?;
        for dev in self.index.list_devices()? {
            if dev.id != self_device_id && !dev.is_online {
                self.index.forget_device(&dev.id)?;
            }
        }
        Ok(())
    }
}
