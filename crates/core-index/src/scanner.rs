use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::UNIX_EPOCH;

use core_platform::AndroidMediaStoreRow;
use rayon::prelude::*;
use walkdir::{DirEntry, WalkDir};

use crate::hash::compute_partial_blake3;
use crate::models::{MediaRecord, MediaTags, MediaType, ScanProgress, SyncState};
use crate::repository::{normalize_db_path, now_ms, IndexError, MediaIndex};

/// Supported PRD A1 media formats and MIME types.
pub fn classify_extension(ext_with_or_without_dot: &str) -> Option<(MediaType, &'static str)> {
    let clean = ext_with_or_without_dot
        .trim_start_matches('.')
        .to_ascii_lowercase();
    match clean.as_str() {
        "mp4" => Some((MediaType::Video, "video/mp4")),
        "mkv" => Some((MediaType::Video, "video/x-matroska")),
        "webm" => Some((MediaType::Video, "video/webm")),
        "mov" => Some((MediaType::Video, "video/quicktime")),
        "avi" => Some((MediaType::Video, "video/x-msvideo")),
        "mp3" => Some((MediaType::Audio, "audio/mpeg")),
        "m4a" => Some((MediaType::Audio, "audio/mp4")),
        "flac" => Some((MediaType::Audio, "audio/flac")),
        "wav" => Some((MediaType::Audio, "audio/wav")),
        "ogg" => Some((MediaType::Audio, "audio/ogg")),
        "png" => Some((MediaType::Image, "image/png")),
        "jpg" | "jpeg" => Some((MediaType::Image, "image/jpeg")),
        "gif" => Some((MediaType::Image, "image/gif")),
        "webp" => Some((MediaType::Image, "image/webp")),
        "heic" => Some((MediaType::Image, "image/heic")),
        _ => None,
    }
}

pub fn stable_media_id(device_id: &str, full_path: &str) -> String {
    let norm = normalize_db_path(full_path).to_ascii_lowercase();
    let hash = blake3::hash(format!("{device_id}::{norm}").as_bytes());
    hash.to_hex()[..24].to_string()
}

fn normalize_prefix(p: &Path) -> String {
    normalize_db_path(&p.to_string_lossy()).to_ascii_lowercase()
}

const SYSTEM_SKIP_DIR_NAMES: &[&str] = &[
    "$recycle.bin",
    "$windows.~bt",
    "$windows.~ws",
    "system volume information",
    "windows",
    "windows.old",
    "program files",
    "program files (x86)",
    "programdata",
    "recovery",
    "perflogs",
    "msocache",
    "config.msi",
    "documents and settings",
    "appdata",
    "winsxs",
    "recycler",
    "node_modules",
    ".git",
    ".hg",
    ".svn",
    ".media-gallery-trash",
    "proc",
    "sys",
    "dev",
];

fn is_reparse_or_symlink(entry: &DirEntry) -> bool {
    if entry.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
        if entry
            .metadata()
            .is_ok_and(|meta| (meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT) != 0)
        {
            return true;
        }
    }
    false
}

fn is_excluded_entry(
    entry: &DirEntry,
    excluded_prefixes: &[String],
    skip_system_dirs: bool,
) -> bool {
    let path = entry.path();
    let depth = entry.depth();
    if depth > 0 {
        if is_reparse_or_symlink(entry) {
            return true;
        }
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            let lower = name.to_ascii_lowercase();
            if lower == ".media-gallery-trash" || (lower.starts_with('.') && lower.len() > 1) {
                return true;
            }
            if skip_system_dirs
                && entry.file_type().is_dir()
                && SYSTEM_SKIP_DIR_NAMES.contains(&lower.as_str())
            {
                return true;
            }
        }
    }
    let norm = normalize_prefix(path);
    excluded_prefixes
        .iter()
        .any(|ex| norm == *ex || norm.starts_with(&format!("{ex}/")))
}

#[derive(Debug, Clone, Default)]
pub struct ProbedMediaMetadata {
    pub duration_ms: u64,
    pub width: u32,
    pub height: u32,
    pub created_at: Option<i64>,
    pub gps_lat: Option<f64>,
    pub gps_lon: Option<f64>,
    pub place: Option<String>,
    pub tags: MediaTags,
    pub auto_alias: Option<String>,
}

pub struct ScanRequest<'a> {
    pub operation_id: String,
    pub device_id: String,
    pub device_name: String,
    pub roots: Vec<PathBuf>,
    pub excluded_folders: Vec<PathBuf>,
    pub compute_hashes: bool,
    pub batch_size: usize,
    pub cancel: Option<&'a AtomicBool>,
}

struct PendingScanFile {
    path: PathBuf,
    path_str: String,
    media_type: MediaType,
    mime: &'static str,
    size: u64,
    fs_created: i64,
    modified_at: i64,
    existing: Option<MediaRecord>,
}

fn process_scan_chunk<F>(
    index: &MediaIndex,
    req: &ScanRequest<'_>,
    enricher: &F,
    chunk: &[(PathBuf, MediaType, &'static str)],
    progress: &mut ScanProgress,
    pool: Option<&rayon::ThreadPool>,
) -> Result<(), IndexError>
where
    F: Fn(&Path, MediaType, i64, i64, &str) -> ProbedMediaMetadata + Send + Sync,
{
    let mut to_process = Vec::new();
    for (path, mt, mime) in chunk {
        progress.scanned_files += 1;
        let Ok(meta) = std::fs::metadata(path) else {
            continue;
        };
        let size = meta.len();
        let modified_at = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as i64)
            .unwrap_or_else(now_ms);
        let created_at = meta
            .created()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as i64)
            .unwrap_or(modified_at);

        let path_str = normalize_db_path(&path.to_string_lossy());
        let existing = index.get_media_by_path(&req.device_id, &path_str)?;
        if let Some(ref ex) = existing {
            if ex.size == size && ex.modified_at == modified_at {
                progress.skipped_files += 1;
                continue;
            }
        }
        to_process.push(PendingScanFile {
            path: path.clone(),
            path_str,
            media_type: *mt,
            mime: *mime,
            size,
            fs_created: created_at,
            modified_at,
            existing,
        });
    }

    if to_process.is_empty() {
        return Ok(());
    }

    let build_one = |item: PendingScanFile| {
        let PendingScanFile {
            path,
            path_str,
            media_type: mt,
            mime,
            size,
            fs_created,
            modified_at,
            existing,
        } = item;
        let partial_hash = if req.compute_hashes {
            compute_partial_blake3(&path).unwrap_or_default()
        } else {
            String::new()
        };
        let probed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            enricher(&path, mt, fs_created, modified_at, &req.device_name)
        }))
        .unwrap_or_default();
        let created_at = probed.created_at.unwrap_or(fs_created);
        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let folder_path = path
            .parent()
            .map(|p| normalize_db_path(&p.to_string_lossy()))
            .unwrap_or_default();
        let auto_alias = probed.auto_alias.unwrap_or_else(|| file_name.clone());

        let is_update = existing.is_some();
        let (id, alias, blake3_val, cover_id, sync_state, is_dup, dup_grp) = match existing {
            Some(ex) => (
                ex.id,
                if ex.alias.is_empty() {
                    auto_alias.clone()
                } else {
                    ex.alias
                },
                if !ex.blake3.is_empty() {
                    ex.blake3
                } else {
                    partial_hash.clone()
                },
                ex.cover_id,
                ex.sync_state,
                ex.is_duplicate,
                ex.duplicate_group_id,
            ),
            None => (
                stable_media_id(&req.device_id, &path_str),
                auto_alias.clone(),
                partial_hash.clone(),
                None,
                SyncState::Local,
                false,
                None,
            ),
        };

        (
            MediaRecord {
                id,
                device_id: req.device_id.clone(),
                path: path_str,
                folder_path,
                file_name,
                media_type: mt,
                mime: mime.to_string(),
                size,
                blake3: blake3_val,
                partial_hash,
                created_at,
                modified_at,
                duration_ms: probed.duration_ms,
                width: probed.width,
                height: probed.height,
                gps_lat: probed.gps_lat,
                gps_lon: probed.gps_lon,
                place: probed.place,
                tags: probed.tags,
                alias,
                auto_alias,
                cover_id,
                sync_state,
                is_duplicate: is_dup,
                duplicate_group_id: dup_grp,
                is_trashed: false,
                trashed_path: None,
                updated_at: now_ms(),
            },
            is_update,
        )
    };

    let built: Vec<(MediaRecord, bool)> = match pool {
        Some(tp) => tp.install(|| to_process.into_par_iter().map(build_one).collect()),
        None => to_process.into_iter().map(build_one).collect(),
    };

    let mut batch_records = Vec::with_capacity(built.len());
    for (rec, is_update) in built {
        progress.current_folder = rec.folder_path.clone();
        if is_update {
            progress.updated_files += 1;
        } else {
            progress.added_files += 1;
        }
        batch_records.push(rec);
    }
    index.upsert_media_batch(&batch_records)?;
    Ok(())
}

/// Scans folders in streaming, bounded-concurrency batches using `walkdir` + `rayon` and writes
/// transactionally to SQLite. Safe for full-drive scans (`C:\`): skips OS system trees and NTFS
/// junctions, deduplicates overlapping roots, isolates corrupt files via `catch_unwind`, and
/// skips unchanged files incrementally.
pub fn scan_folders<F, P>(
    index: &MediaIndex,
    req: &ScanRequest<'_>,
    enricher: &F,
    mut on_progress: P,
) -> Result<ScanProgress, IndexError>
where
    F: Fn(&Path, MediaType, i64, i64, &str) -> ProbedMediaMetadata + Send + Sync,
    P: FnMut(&ScanProgress),
{
    let mut excluded_prefixes: Vec<String> = Vec::new();
    let mut disabled_types: HashSet<MediaType> = HashSet::new();
    let mut disabled_exts: HashSet<String> = HashSet::new();
    let mut skip_system_dirs = true;

    for raw in &req.excluded_folders {
        let s = raw.to_string_lossy();
        if let Some(t) = s.strip_prefix("__mg_disable_type__:") {
            disabled_types.insert(MediaType::parse(t));
        } else if let Some(ext) = s.strip_prefix("__mg_disable_ext__:") {
            disabled_exts.insert(ext.trim_start_matches('.').to_ascii_lowercase());
        } else if s == "__mg_allow_system_dirs__" {
            skip_system_dirs = false;
        } else {
            excluded_prefixes.push(normalize_prefix(raw));
        }
    }

    let mut progress = ScanProgress {
        operation_id: req.operation_id.clone(),
        scanned_files: 0,
        added_files: 0,
        updated_files: 0,
        skipped_files: 0,
        current_folder: String::new(),
        in_progress: true,
    };

    let worker_threads = std::thread::available_parallelism()
        .map(|n| n.get().clamp(1, 4))
        .unwrap_or(2);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(worker_threads)
        .build()
        .ok();

    let chunk_size = req.batch_size.clamp(32, 200);
    let mut pending_batch: Vec<(PathBuf, MediaType, &'static str)> = Vec::with_capacity(chunk_size);
    let mut seen_paths: HashSet<String> = HashSet::new();
    let mut seen_roots: HashSet<String> = HashSet::new();

    for root in &req.roots {
        if !root.exists() {
            continue;
        }
        let root_key = normalize_prefix(root);
        if !seen_roots.insert(root_key) {
            continue;
        }

        let iter = WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| !is_excluded_entry(e, &excluded_prefixes, skip_system_dirs));

        for entry in iter.flatten() {
            if let Some(cancel) = req.cancel {
                if cancel.load(Ordering::Relaxed) {
                    progress.in_progress = false;
                    on_progress(&progress);
                    return Ok(progress);
                }
            }
            if !entry.file_type().is_file() {
                continue;
            }
            let Some(ext) = entry.path().extension().and_then(|e| e.to_str()) else {
                continue;
            };
            let ext_lower = ext.to_ascii_lowercase();
            if disabled_exts.contains(&ext_lower) {
                continue;
            }
            let Some((mt, mime)) = classify_extension(&ext_lower) else {
                continue;
            };
            if disabled_types.contains(&mt) {
                continue;
            }

            let file_key = normalize_db_path(&entry.path().to_string_lossy()).to_ascii_lowercase();
            if !seen_paths.insert(file_key) {
                continue;
            }

            pending_batch.push((entry.into_path(), mt, mime));
            if pending_batch.len() >= chunk_size {
                process_scan_chunk(
                    index,
                    req,
                    enricher,
                    &pending_batch,
                    &mut progress,
                    pool.as_ref(),
                )?;
                pending_batch.clear();
                on_progress(&progress);
            }
        }
    }

    if !pending_batch.is_empty() {
        process_scan_chunk(
            index,
            req,
            enricher,
            &pending_batch,
            &mut progress,
            pool.as_ref(),
        )?;
        pending_batch.clear();
        on_progress(&progress);
    }

    progress.in_progress = false;
    on_progress(&progress);
    Ok(progress)
}

/// Converts Android Kotlin `MediaStore` cursor batches into SQLite index records (PRD Section 8).
pub fn ingest_android_mediastore_batch<F>(
    index: &MediaIndex,
    device_id: &str,
    device_name: &str,
    rows: &[AndroidMediaStoreRow],
    alias_fn: &F,
) -> Result<u32, IndexError>
where
    F: Fn(&AndroidMediaStoreRow, MediaType, &str) -> (String, Option<String>),
{
    let mut records = Vec::with_capacity(rows.len());
    for row in rows {
        let media_type = if row.mime_type.starts_with("video/") {
            MediaType::Video
        } else if row.mime_type.starts_with("audio/") {
            MediaType::Audio
        } else {
            MediaType::Image
        };
        let folder_path = normalize_db_path(&format!(
            "/storage/emulated/0/{}",
            row.relative_path.trim_end_matches('/')
        ));
        let (auto_alias, place) = alias_fn(row, media_type, device_name);
        let created_at = row.date_added_sec.max(row.date_modified_sec) * 1000;
        let modified_at = row.date_modified_sec * 1000;

        records.push(MediaRecord {
            id: stable_media_id(device_id, &row.uri),
            device_id: device_id.to_string(),
            path: row.uri.clone(),
            folder_path,
            file_name: row.display_name.clone(),
            media_type,
            mime: row.mime_type.clone(),
            size: row.size,
            blake3: String::new(),
            partial_hash: format!("{}:{}", row.size, row.date_modified_sec),
            created_at,
            modified_at,
            duration_ms: row.duration_ms,
            width: row.width,
            height: row.height,
            gps_lat: row.latitude,
            gps_lon: row.longitude,
            place,
            tags: MediaTags {
                title: row.title.clone(),
                artist: row.artist.clone(),
                album: row.album.clone(),
                ..Default::default()
            },
            alias: auto_alias.clone(),
            auto_alias,
            cover_id: None,
            sync_state: SyncState::Local,
            is_duplicate: false,
            duplicate_group_id: None,
            is_trashed: false,
            trashed_path: None,
            updated_at: now_ms(),
        });
    }
    let count = records.len() as u32;
    index.upsert_media_batch(&records)?;
    Ok(count)
}
