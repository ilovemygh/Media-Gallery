//! OS-abstracting traits and desktop implementations for Media Gallery.
//!
//! All platform-specific services (thumbnail sidecars, trash, secure storage,
//! filesystem watching, yt-dlp, and Android MediaStore ingress) sit behind
//! traits here so the core crates remain testable and portable across Windows and Android.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use image::{ImageBuffer, Rgba};
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Typed platform layer errors.
#[derive(Debug, Error)]
pub enum PlatformError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("sidecar command failed: {0}")]
    Sidecar(String),
    #[error("file watcher error: {0}")]
    Watcher(String),
    #[error("secure store error: {0}")]
    SecureStore(String),
    #[error("image encoding error: {0}")]
    Image(String),
}

/// Row pushed from the Kotlin Android MediaStore plugin into Rust.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AndroidMediaStoreRow {
    pub uri: String,
    pub display_name: String,
    pub relative_path: String,
    pub mime_type: String,
    pub size: u64,
    pub date_added_sec: i64,
    pub date_modified_sec: i64,
    pub duration_ms: u64,
    pub width: u32,
    pub height: u32,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub title: Option<String>,
}

/// Basic probed metadata returned by `ffprobe` or OS media APIs.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProbedVideoMeta {
    pub duration_ms: u64,
    pub width: u32,
    pub height: u32,
    pub created_at_ms: Option<i64>,
}

/// Abstracts desktop `ffmpeg`/`ffprobe` sidecar or Android `MediaMetadataRetriever`.
pub trait ThumbnailSidecar: Send + Sync {
    fn extract_video_frame(
        &self,
        video_path: &Path,
        frame_time_ms: u64,
        out_path: &Path,
    ) -> Result<PathBuf, PlatformError>;

    fn probe_video(&self, video_path: &Path) -> Result<ProbedVideoMeta, PlatformError>;
}

/// Desktop implementation invoking an external LGPL `ffmpeg` / `ffprobe` sidecar binary,
/// with an optional deterministic PNG placeholder fallback when `ffmpeg` is absent or on synthetic fixtures.
#[derive(Debug, Clone)]
pub struct DesktopFfmpegSidecar {
    pub ffmpeg_bin: PathBuf,
    pub ffprobe_bin: PathBuf,
    pub allow_fallback_render: bool,
}

impl Default for DesktopFfmpegSidecar {
    fn default() -> Self {
        Self {
            ffmpeg_bin: PathBuf::from("ffmpeg"),
            ffprobe_bin: PathBuf::from("ffprobe"),
            allow_fallback_render: true,
        }
    }
}

impl DesktopFfmpegSidecar {
    pub fn with_bins(
        ffmpeg_bin: impl Into<PathBuf>,
        ffprobe_bin: impl Into<PathBuf>,
        allow_fallback_render: bool,
    ) -> Self {
        Self {
            ffmpeg_bin: ffmpeg_bin.into(),
            ffprobe_bin: ffprobe_bin.into(),
            allow_fallback_render,
        }
    }

    fn render_fallback_png(&self, seed_text: &str, frame_time_ms: u64, out_path: &Path) -> Result<PathBuf, PlatformError> {
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let png_path = out_path.with_extension("png");
        let hash = blake3::hash(format!("{seed_text}:{frame_time_ms}").as_bytes());
        let b = hash.as_bytes();
        let w = 320u32;
        let h = 180u32;
        let img = ImageBuffer::from_fn(w, h, |x, y| {
            let r = b[0].saturating_add(((x * 96) / w) as u8);
            let g = b[1].saturating_add(((y * 96) / h) as u8);
            let bl = b[2].saturating_add((((x + y) * 64) / (w + h)) as u8);
            Rgba([r, g, bl, 255])
        });
        img.save(&png_path)
            .map_err(|e| PlatformError::Image(e.to_string()))?;
        Ok(png_path)
    }
}

impl ThumbnailSidecar for DesktopFfmpegSidecar {
    fn extract_video_frame(
        &self,
        video_path: &Path,
        frame_time_ms: u64,
        out_path: &Path,
    ) -> Result<PathBuf, PlatformError> {
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let seek_secs = format!("{:.3}", (frame_time_ms as f64) / 1000.0);
        let output = Command::new(&self.ffmpeg_bin)
            .args([
                "-ss",
                &seek_secs,
                "-i",
                &video_path.to_string_lossy(),
                "-vframes",
                "1",
                "-q:v",
                "3",
                "-y",
                &out_path.to_string_lossy(),
            ])
            .output();

        match output {
            Ok(res) if res.status.success() && out_path.exists() => Ok(out_path.to_path_buf()),
            Ok(res) if !self.allow_fallback_render => Err(PlatformError::Sidecar(
                String::from_utf8_lossy(&res.stderr).to_string(),
            )),
            Err(err) if !self.allow_fallback_render => Err(PlatformError::Io(err)),
            _ => self.render_fallback_png(&video_path.to_string_lossy(), frame_time_ms, out_path),
        }
    }

    fn probe_video(&self, video_path: &Path) -> Result<ProbedVideoMeta, PlatformError> {
        let out = Command::new(&self.ffprobe_bin)
            .args([
                "-v",
                "error",
                "-show_entries",
                "format=duration:stream=width,height",
                "-of",
                "default=noprint_wrappers=1:nokey=0",
                &video_path.to_string_lossy(),
            ])
            .output();

        let Ok(res) = out else {
            return Ok(ProbedVideoMeta::default());
        };
        if !res.status.success() {
            return Ok(ProbedVideoMeta::default());
        }
        let text = String::from_utf8_lossy(&res.stdout);
        let mut meta = ProbedVideoMeta::default();
        for line in text.lines() {
            if let Some(val) = line.strip_prefix("duration=") {
                if let Ok(secs) = val.trim().parse::<f64>() {
                    meta.duration_ms = (secs * 1000.0).max(0.0) as u64;
                }
            } else if let Some(val) = line.strip_prefix("width=") {
                if let Ok(w) = val.trim().parse::<u32>() {
                    meta.width = w;
                }
            } else if let Some(val) = line.strip_prefix("height=") {
                if let Ok(h) = val.trim().parse::<u32>() {
                    meta.height = h;
                }
            }
        }
        Ok(meta)
    }
}

/// Safe deletion abstraction: moves items to recycle bin / trash directory instead of unlinking.
pub trait TrashBackend: Send + Sync {
    fn move_to_trash(&self, file_path: &Path) -> Result<PathBuf, PlatformError>;
}

/// Portable trash directory backend (`.media-gallery-trash`) safe across tests and desktop.
#[derive(Debug, Clone, Default)]
pub struct ManagedTrashBackend {
    pub custom_trash_root: Option<PathBuf>,
}

impl TrashBackend for ManagedTrashBackend {
    fn move_to_trash(&self, file_path: &Path) -> Result<PathBuf, PlatformError> {
        let trash_dir = match &self.custom_trash_root {
            Some(root) => root.clone(),
            None => file_path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(".media-gallery-trash"),
        };
        std::fs::create_dir_all(&trash_dir)?;
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let name = file_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "trashed".to_string());
        let dest = trash_dir.join(format!("{ts}-{name}"));
        if file_path.exists() {
            std::fs::rename(file_path, &dest)?;
        }
        Ok(dest)
    }
}

/// OS Secure Storage abstraction for device keys and optional OAuth tokens.
pub trait SecureStore: Send + Sync {
    fn set_secret(&self, key: &str, secret: &str) -> Result<(), PlatformError>;
    fn get_secret(&self, key: &str) -> Result<Option<String>, PlatformError>;
    fn delete_secret(&self, key: &str) -> Result<(), PlatformError>;
}

/// Keyed BLAKE3 stream-obfuscated in-memory credential store for tests & non-GUI hosts.
/// Secrets are never persisted to plaintext files or SQLite tables.
#[derive(Debug)]
pub struct EncryptedMemoryStore {
    key: [u8; 32],
    entries: RwLock<HashMap<String, Vec<u8>>>,
}

impl Default for EncryptedMemoryStore {
    fn default() -> Self {
        let now_nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(1);
        let key = *blake3::hash(&now_nanos.to_le_bytes()).as_bytes();
        Self {
            key,
            entries: RwLock::new(HashMap::new()),
        }
    }
}

impl EncryptedMemoryStore {
    fn xor_stream(&self, label: &str, data: &[u8]) -> Vec<u8> {
        let mut hasher = blake3::Hasher::new_keyed(&self.key);
        hasher.update(label.as_bytes());
        let mut xof = hasher.finalize_xof();
        let mut pad = vec![0u8; data.len()];
        xof.fill(&mut pad);
        data.iter().zip(pad.iter()).map(|(a, b)| a ^ b).collect()
    }
}

impl SecureStore for EncryptedMemoryStore {
    fn set_secret(&self, key: &str, secret: &str) -> Result<(), PlatformError> {
        let cipher = self.xor_stream(key, secret.as_bytes());
        self.entries
            .write()
            .map_err(|_| PlatformError::SecureStore("poisoned lock".into()))?
            .insert(key.to_string(), cipher);
        Ok(())
    }

    fn get_secret(&self, key: &str) -> Result<Option<String>, PlatformError> {
        let guard = self
            .entries
            .read()
            .map_err(|_| PlatformError::SecureStore("poisoned lock".into()))?;
        let Some(cipher) = guard.get(key) else {
            return Ok(None);
        };
        let plain = self.xor_stream(key, cipher);
        String::from_utf8(plain)
            .map(Some)
            .map_err(|e| PlatformError::SecureStore(e.to_string()))
    }

    fn delete_secret(&self, key: &str) -> Result<(), PlatformError> {
        self.entries
            .write()
            .map_err(|_| PlatformError::SecureStore("poisoned lock".into()))?
            .remove(key);
        Ok(())
    }
}

/// Cross-platform desktop file watcher backed by `notify`.
pub struct DesktopFileWatcher {
    watchers: Mutex<Vec<RecommendedWatcher>>,
}

impl Default for DesktopFileWatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl DesktopFileWatcher {
    pub fn new() -> Self {
        Self {
            watchers: Mutex::new(Vec::new()),
        }
    }

    pub fn watch_roots<F>(&self, roots: &[PathBuf], on_paths: F) -> Result<(), PlatformError>
    where
        F: Fn(Vec<PathBuf>) + Send + Sync + 'static,
    {
        let cb = Arc::new(on_paths);
        let mut guard = self
            .watchers
            .lock()
            .map_err(|_| PlatformError::Watcher("watcher lock poisoned".into()))?;
        guard.clear();

        for root in roots {
            if !root.exists() {
                continue;
            }
            let cb_clone = Arc::clone(&cb);
            let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
                if let Ok(event) = res {
                    if !event.paths.is_empty() {
                        cb_clone(event.paths);
                    }
                }
            })
            .map_err(|e| PlatformError::Watcher(e.to_string()))?;

            watcher
                .watch(root, RecursiveMode::Recursive)
                .map_err(|e| PlatformError::Watcher(e.to_string()))?;
            guard.push(watcher);
        }
        Ok(())
    }

    pub fn stop(&self) {
        if let Ok(mut guard) = self.watchers.lock() {
            guard.clear();
        }
    }
}

/// Abstracts `yt-dlp` execution (sidecar CLI on desktop vs `youtubedl-android` Kotlin plugin on Android).
pub trait YtDlpBackend: Send + Sync {
    fn run_download(
        &self,
        args: &[String],
        on_progress: &(dyn Fn(f32, Option<String>) + Send + Sync),
    ) -> Result<Vec<PathBuf>, PlatformError>;

    fn run_update(&self) -> Result<String, PlatformError>;
}

/// Desktop CLI sidecar runner for `yt-dlp`.
#[derive(Debug, Clone)]
pub struct DesktopYtDlpBackend {
    pub yt_dlp_bin: PathBuf,
}

impl Default for DesktopYtDlpBackend {
    fn default() -> Self {
        Self {
            yt_dlp_bin: PathBuf::from("yt-dlp"),
        }
    }
}

impl YtDlpBackend for DesktopYtDlpBackend {
    fn run_download(
        &self,
        args: &[String],
        on_progress: &(dyn Fn(f32, Option<String>) + Send + Sync),
    ) -> Result<Vec<PathBuf>, PlatformError> {
        let output = Command::new(&self.yt_dlp_bin).args(args).output()?;
        if !output.status.success() {
            return Err(PlatformError::Sidecar(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut paths = Vec::new();
        for line in stdout.lines() {
            if let Some(rest) = line.strip_prefix("[download] ") {
                if let Some(pct_str) = rest.split('%').next() {
                    if let Ok(pct) = pct_str.trim().parse::<f32>() {
                        on_progress(pct, None);
                    }
                }
            }
            for prefix in ["Destination: ", "[ExtractAudio] Destination: ", "Merging formats into "] {
                if let Some(idx) = line.find(prefix) {
                    let raw = line[idx + prefix.len()..].trim().trim_matches('"');
                    if !raw.is_empty() {
                        paths.push(PathBuf::from(raw));
                    }
                }
            }
        }
        on_progress(100.0, None);
        Ok(paths)
    }

    fn run_update(&self) -> Result<String, PlatformError> {
        let output = Command::new(&self.yt_dlp_bin).arg("-U").output()?;
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        if output.status.success() {
            Ok(combined.trim().to_string())
        } else {
            Err(PlatformError::Sidecar(combined))
        }
    }
}
