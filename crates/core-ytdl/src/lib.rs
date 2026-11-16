//! YouTube download job queue (`yt-dlp` + `ffmpeg` sidecar on desktop,
//! `youtubedl-android` plugin backend on Android), updater, and auto-library ingestion.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use core_index::{
    classify_extension, compute_partial_blake3, now_ms, stable_media_id, DownloadFormat,
    DownloadJobRecord, DownloadStatus, IndexError, MediaIndex, MediaRecord, MediaTags, MediaType,
    SyncState,
};
use core_media::{MediaOpError, ThumbnailManager};
use core_platform::{PlatformError, ThumbnailSidecar, YtDlpBackend};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

pub const YOUTUBE_COMPLIANCE_NOTICE: &str =
    "Download only content you have the right to download under applicable copyright rules and YouTube's Terms of Service.";

pub const YOUTUBE_PORTAL_URLS: [(&str, &str); 3] = [
    (
        "upload",
        "https://studio.youtube.com/channel/UC/videos/upload",
    ),
    ("channel", "https://www.youtube.com/channel/mine"),
    ("studio", "https://studio.youtube.com"),
];

#[derive(Debug, Error)]
pub enum YtdlError {
    #[error("index error: {0}")]
    Index(#[from] IndexError),
    #[error("platform error: {0}")]
    Platform(#[from] PlatformError),
    #[error("media error: {0}")]
    Media(#[from] MediaOpError),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YoutubeDownloadRequest {
    pub url: String,
    pub format: DownloadFormat,
    pub quality: String,
    pub destination_device_id: String,
    pub destination_folder: String,
    pub allow_playlist: bool,
    pub ffmpeg_bin: Option<String>,
}

/// Builds deterministic `yt-dlp` CLI arguments for MP4 or MP3 + embedded cover/metadata.
pub fn build_yt_dlp_args(req: &YoutubeDownloadRequest) -> Vec<String> {
    let dest = Path::new(&req.destination_folder)
        .join("%(title)s [%(id)s].%(ext)s")
        .to_string_lossy()
        .to_string();

    let mut args = vec![
        "--newline".to_string(),
        "--write-thumbnail".to_string(),
        "--embed-thumbnail".to_string(),
        "--add-metadata".to_string(),
        "--output".to_string(),
        dest,
        if req.allow_playlist {
            "--yes-playlist".to_string()
        } else {
            "--no-playlist".to_string()
        },
    ];

    if let Some(ref ffmpeg) = req.ffmpeg_bin {
        args.push("--ffmpeg-location".to_string());
        args.push(ffmpeg.clone());
    }

    match req.format {
        DownloadFormat::Mp3 => {
            let clean_q: String = req
                .quality
                .chars()
                .filter(|c| c.is_ascii_digit() || *c == 'k' || *c == 'K')
                .collect();
            let q = if clean_q.is_empty() {
                "192K".to_string()
            } else {
                clean_q
            };
            args.push("--extract-audio".to_string());
            args.push("--audio-format".to_string());
            args.push("mp3".to_string());
            args.push("--audio-quality".to_string());
            args.push(q);
        }
        DownloadFormat::Mp4 => {
            let digits: String = req
                .quality
                .trim_end_matches('p')
                .chars()
                .filter(|c| c.is_ascii_digit())
                .collect();
            let format_expr = if let Ok(h) = digits.parse::<u32>() {
                format!("bestvideo[height<={h}]+bestaudio/best[height<={h}]")
            } else {
                "bestvideo+bestaudio/best".to_string()
            };
            args.push("-f".to_string());
            args.push(format_expr);
            args.push("--merge-output-format".to_string());
            args.push("mp4".to_string());
        }
    }

    args.push(req.url.clone());
    args
}

pub struct YoutubeDownloader {
    index: MediaIndex,
    backend: Arc<dyn YtDlpBackend>,
    sidecar: Arc<dyn ThumbnailSidecar>,
    cache_dir: PathBuf,
}

impl YoutubeDownloader {
    pub fn new(
        index: MediaIndex,
        backend: Arc<dyn YtDlpBackend>,
        sidecar: Arc<dyn ThumbnailSidecar>,
        cache_dir: impl Into<PathBuf>,
    ) -> Self {
        Self {
            index,
            backend,
            sidecar,
            cache_dir: cache_dir.into(),
        }
    }

    pub fn update_yt_dlp(&self) -> Result<String, YtdlError> {
        Ok(self.backend.run_update()?)
    }

    pub fn enqueue_and_download<F>(
        &self,
        req: &YoutubeDownloadRequest,
        mut on_job: F,
    ) -> Result<(DownloadJobRecord, Vec<MediaRecord>), YtdlError>
    where
        F: FnMut(&DownloadJobRecord),
    {
        let dest_dir = Path::new(&req.destination_folder);
        std::fs::create_dir_all(dest_dir)?;

        let mut job = DownloadJobRecord {
            id: Uuid::new_v4().to_string(),
            url: req.url.clone(),
            title: None,
            format: req.format,
            quality: req.quality.clone(),
            destination_device_id: req.destination_device_id.clone(),
            destination_folder: req.destination_folder.replace('\\', "/"),
            status: DownloadStatus::Downloading,
            progress: 0.0,
            output_media_id: None,
            error: None,
            created_at: now_ms(),
            updated_at: now_ms(),
        };
        self.index.upsert_download_job(&job)?;
        on_job(&job);

        let args = build_yt_dlp_args(req);
        let run_res = self.backend.run_download(&args, &|_pct, _title| {});

        match run_res {
            Ok(paths) => {
                let mut imported = Vec::new();
                let thumb_mgr = ThumbnailManager {
                    index: &self.index,
                    sidecar: self.sidecar.as_ref(),
                    cache_dir: &self.cache_dir,
                    max_cache_bytes: None,
                };

                for path in paths {
                    let ext = path
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("")
                        .to_ascii_lowercase();
                    let expected_ext = match req.format {
                        DownloadFormat::Mp3 => "mp3",
                        DownloadFormat::Mp4 => "mp4",
                    };
                    if ext != expected_ext || !path.exists() {
                        continue;
                    }
                    let Some((mt, mime)) = classify_extension(&ext) else {
                        continue;
                    };
                    let file_name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    let title_stem = path
                        .file_stem()
                        .map(|s| s.to_string_lossy().replace('_', " "))
                        .unwrap_or_else(|| file_name.clone());
                    let path_str = path.to_string_lossy().replace('\\', "/");
                    let size = std::fs::metadata(&path)?.len();
                    let partial_hash = compute_partial_blake3(&path).unwrap_or_default();
                    let ts = now_ms();

                    let rec = MediaRecord {
                        id: stable_media_id(&req.destination_device_id, &path_str),
                        device_id: req.destination_device_id.clone(),
                        path: path_str,
                        folder_path: req.destination_folder.replace('\\', "/"),
                        file_name,
                        media_type: mt,
                        mime: mime.to_string(),
                        size,
                        blake3: partial_hash.clone(),
                        partial_hash,
                        created_at: ts,
                        modified_at: ts,
                        duration_ms: 180_000,
                        width: if mt == MediaType::Video { 1920 } else { 0 },
                        height: if mt == MediaType::Video { 1080 } else { 0 },
                        gps_lat: None,
                        gps_lon: None,
                        place: None,
                        tags: MediaTags {
                            title: Some(title_stem.clone()),
                            artist: if mt == MediaType::Audio {
                                Some("YouTube Audio".to_string())
                            } else {
                                None
                            },
                            uploader: Some("YouTube".to_string()),
                            source: Some("youtube".to_string()),
                            ..Default::default()
                        },
                        alias: title_stem.clone(),
                        auto_alias: title_stem,
                        cover_id: None,
                        sync_state: SyncState::Local,
                        is_duplicate: false,
                        duplicate_group_id: None,
                        is_trashed: false,
                        trashed_path: None,
                        updated_at: ts,
                    };
                    self.index.upsert_media(&rec)?;
                    let _ = thumb_mgr.ensure_thumbnail(&rec, None);
                    imported.push(rec);
                }

                job.status = DownloadStatus::Completed;
                job.progress = 100.0;
                job.title = imported.first().map(|m| m.alias.clone());
                job.output_media_id = imported.first().map(|m| m.id.clone());
                job.updated_at = now_ms();
                self.index.upsert_download_job(&job)?;
                on_job(&job);
                Ok((job, imported))
            }
            Err(err) => {
                job.status = DownloadStatus::Error;
                job.error = Some(err.to_string());
                job.updated_at = now_ms();
                self.index.upsert_download_job(&job)?;
                on_job(&job);
                Err(YtdlError::Platform(err))
            }
        }
    }
}
