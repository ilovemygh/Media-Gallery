use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::UNIX_EPOCH;

use core_platform::AndroidMediaStoreRow;
use rayon::prelude::*;
use walkdir::WalkDir;

use crate::hash::compute_partial_blake3;
use crate::models::{MediaRecord, MediaTags, MediaType, ScanProgress, SyncState};
use crate::repository::{now_ms, IndexError, MediaIndex};

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
    let norm = full_path.replace('\\', "/").to_ascii_lowercase();
    let hash = blake3::hash(format!("{device_id}::{norm}").as_bytes());
    hash.to_hex()[..24].to_string()
}

fn normalize_prefix(p: &Path) -> String {
    p.to_string_lossy()
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_ascii_lowercase()
}

fn is_excluded(path: &Path, excluded_prefixes: &[String]) -> bool {
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        if name == ".media-gallery-trash" || (name.starts_with('.') && name.len() > 1) {
            return true;
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

/// Scans folders in parallel batches using `walkdir` + `rayon` and writes transactionally to SQLite.
/// Supports cancellation, incremental skip on matching size+mtime, and pluggable metadata/alias enrichment.
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
    let excluded_prefixes: Vec<String> = req
        .excluded_folders
        .iter()
        .map(|p| normalize_prefix(p))
        .collect();

    let mut candidate_files: Vec<(PathBuf, MediaType, &'static str)> = Vec::new();
    for root in &req.roots {
        if !root.exists() {
            continue;
        }
        let iter = WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| !is_excluded(e.path(), &excluded_prefixes));
        for entry in iter.flatten() {
            if let Some(cancel) = req.cancel {
                if cancel.load(Ordering::Relaxed) {
                    return Ok(ScanProgress {
                        operation_id: req.operation_id.clone(),
                        scanned_files: candidate_files.len() as u64,
                        added_files: 0,
                        updated_files: 0,
                        skipped_files: 0,
                        current_folder: String::new(),
                        in_progress: false,
                    });
                }
            }
            if !entry.file_type().is_file() {
                continue;
            }
            let Some(ext) = entry.path().extension().and_then(|e| e.to_str()) else {
                continue;
            };
            if let Some((mt, mime)) = classify_extension(ext) {
                candidate_files.push((entry.into_path(), mt, mime));
            }
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

    let chunk_size = req.batch_size.max(50);
    for chunk in candidate_files.chunks(chunk_size) {
        if let Some(cancel) = req.cancel {
            if cancel.load(Ordering::Relaxed) {
                progress.in_progress = false;
                on_progress(&progress);
                return Ok(progress);
            }
        }

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

            let path_str = path.to_string_lossy().replace('\\', "/");
            if let Some(existing) = index.get_media_by_path(&req.device_id, &path_str)? {
                if existing.size == size && existing.modified_at == modified_at {
                    progress.skipped_files += 1;
                    continue;
                }
                to_process.push((
                    path.clone(),
                    path_str,
                    *mt,
                    *mime,
                    size,
                    created_at,
                    modified_at,
                    Some(existing),
                ));
            } else {
                to_process.push((
                    path.clone(),
                    path_str,
                    *mt,
                    *mime,
                    size,
                    created_at,
                    modified_at,
                    None,
                ));
            }
        }

        let built: Vec<(MediaRecord, bool)> = to_process
            .into_par_iter()
            .map(
                |(path, path_str, mt, mime, size, fs_created, modified_at, existing)| {
                    let partial_hash = if req.compute_hashes {
                        compute_partial_blake3(&path).unwrap_or_default()
                    } else {
                        String::new()
                    };
                    let probed = enricher(&path, mt, fs_created, modified_at, &req.device_name);
                    let created_at = probed.created_at.unwrap_or(fs_created);
                    let file_name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    let folder_path = path
                        .parent()
                        .map(|p| p.to_string_lossy().replace('\\', "/"))
                        .unwrap_or_default();
                    let auto_alias = probed.auto_alias.unwrap_or_else(|| file_name.clone());

                    let is_update = existing.is_some();
                    let (id, alias, blake3_val, cover_id, sync_state, is_dup, dup_grp) =
                        match existing {
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
                },
            )
            .collect();

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
        let folder_path = format!(
            "/storage/emulated/0/{}",
            row.relative_path.trim_end_matches('/')
        );
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
