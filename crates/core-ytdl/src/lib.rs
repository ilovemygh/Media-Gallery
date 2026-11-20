//! YouTube download job queue (`yt-dlp` + `ffmpeg` sidecar on desktop,
//! `youtubedl-android` plugin backend on Android), updater, and auto-library ingestion.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use core_index::{
    classify_extension, compute_partial_blake3, now_ms, repository::normalize_db_path,
    stable_media_id, DownloadFormat, DownloadJobRecord, DownloadStatus, IndexError, MediaIndex,
    MediaRecord, MediaTags, MediaType, SyncState,
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
        let resolved_dest = {
            let p = Path::new(&req.destination_folder);
            if p.is_relative() {
                if let Ok(home) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
                    PathBuf::from(home).join(p)
                } else {
                    p.to_path_buf()
                }
            } else {
                p.to_path_buf()
            }
        };
        std::fs::create_dir_all(&resolved_dest)?;
        let resolved_dest_str = normalize_db_path(&resolved_dest.to_string_lossy());
        let mut effective_req = req.clone();
        effective_req.destination_folder = resolved_dest_str.clone();

        let mut job = DownloadJobRecord {
            id: Uuid::new_v4().to_string(),
            url: req.url.clone(),
            title: None,
            format: req.format,
            quality: req.quality.clone(),
            destination_device_id: req.destination_device_id.clone(),
            destination_folder: resolved_dest_str.clone(),
            status: DownloadStatus::Downloading,
            progress: 0.0,
            output_media_id: None,
            error: None,
            created_at: now_ms(),
            updated_at: now_ms(),
        };
        self.index.upsert_download_job(&job)?;
        on_job(&job);

        let started_at = std::time::SystemTime::now();
        let args = build_yt_dlp_args(&effective_req);
        let run_res = self.backend.run_download(&args, &|_pct, _title| {});

        match run_res {
            Ok(mut paths) => {
                let expected_ext = match req.format {
                    DownloadFormat::Mp3 => "mp3",
                    DownloadFormat::Mp4 => "mp4",
                };
                // Fallback: if yt-dlp merged/renamed without printing the final path, pick newly written matching files.
                let has_expected = paths.iter().any(|p| {
                    p.exists()
                        && p.extension()
                            .and_then(|e| e.to_str())
                            .is_some_and(|e| e.eq_ignore_ascii_case(expected_ext))
                });
                if !has_expected {
                    if let Ok(entries) = std::fs::read_dir(&resolved_dest) {
                        for entry in entries.flatten() {
                            let p = entry.path();
                            let matches_ext = p
                                .extension()
                                .and_then(|e| e.to_str())
                                .is_some_and(|e| e.eq_ignore_ascii_case(expected_ext));
                            if !matches_ext || !p.is_file() {
                                continue;
                            }
                            let is_recent = entry
                                .metadata()
                                .ok()
                                .and_then(|m| m.modified().ok())
                                .is_none_or(|m| m >= started_at);
                            if is_recent && !paths.contains(&p) {
                                paths.push(p);
                            }
                        }
                    }
                }

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
                    let path_str = normalize_db_path(&path.to_string_lossy());
                    let folder_str = path
                        .parent()
                        .map(|p| normalize_db_path(&p.to_string_lossy()))
                        .unwrap_or_else(|| resolved_dest_str.clone());
                    let size = std::fs::metadata(&path)?.len();
                    let partial_hash = compute_partial_blake3(&path).unwrap_or_default();
                    let ts = now_ms();
                    let (duration_ms, width, height) = if mt == MediaType::Video {
                        let vm = self.sidecar.probe_video(&path).unwrap_or_default();
                        let dur = if vm.duration_ms > 0 {
                            vm.duration_ms
                        } else {
                            180_000
                        };
                        let w = if vm.width > 0 { vm.width } else { 1920 };
                        let h = if vm.height > 0 { vm.height } else { 1080 };
                        (dur, w, h)
                    } else {
                        (180_000, 0, 0)
                    };

                    let rec = MediaRecord {
                        id: stable_media_id(&req.destination_device_id, &path_str),
                        device_id: req.destination_device_id.clone(),
                        path: path_str,
                        folder_path: folder_str,
                        file_name,
                        media_type: mt,
                        mime: mime.to_string(),
                        size,
                        blake3: partial_hash.clone(),
                        partial_hash,
                        created_at: ts,
                        modified_at: ts,
                        duration_ms,
                        width,
                        height,
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
