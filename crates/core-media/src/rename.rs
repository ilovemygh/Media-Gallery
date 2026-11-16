use std::collections::HashSet;
use std::path::{Path, PathBuf};

use core_index::{now_ms, IndexError, MediaIndex, MediaRecord, RenameLogRecord};
use core_platform::{PlatformError, TrashBackend};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::alias::{clean_file_name_for_alias, format_date_token};

#[derive(Debug, Error)]
pub enum MediaOpError {
    #[error("index error: {0}")]
    Index(#[from] IndexError),
    #[error("platform error: {0}")]
    Platform(#[from] PlatformError),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("media item not found: {0}")]
    NotFound(String),
    #[error("destination already exists: {0}")]
    Conflict(String),
    #[error("invalid name: {0}")]
    InvalidName(String),
    #[error("confirmation required: {0}")]
    ConfirmationRequired(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkRenamePreviewItem {
    pub media_id: String,
    pub old_name: String,
    pub new_name: String,
    pub old_path: String,
    pub new_path: String,
}

pub fn sanitize_filename_segment(raw: &str) -> String {
    let cleaned: String = raw
        .chars()
        .map(|c| {
            if matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') || c.is_control() {
                ' '
            } else {
                c
            }
        })
        .collect();
    cleaned.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Non-destructive DB alias update (PRD C3).
pub fn set_alias(index: &MediaIndex, media_id: &str, alias: &str) -> Result<MediaRecord, MediaOpError> {
    if index.get_media_by_id(media_id)?.is_none() {
        return Err(MediaOpError::NotFound(media_id.to_string()));
    }
    index.update_alias(media_id, alias.trim())?;
    Ok(index
        .get_media_by_id(media_id)?
        .ok_or_else(|| MediaOpError::NotFound(media_id.to_string()))?
        .media)
}

/// Applies real rename on disk and logs to `rename_log` for undo (PRD C3).
pub fn apply_real_rename(
    index: &MediaIndex,
    media_id: &str,
    requested_name: &str,
    batch_id: Option<&str>,
) -> Result<(MediaRecord, RenameLogRecord), MediaOpError> {
    let item = index
        .get_media_by_id(media_id)?
        .ok_or_else(|| MediaOpError::NotFound(media_id.to_string()))?
        .media;

    let sanitized = sanitize_filename_segment(requested_name);
    if sanitized.is_empty() {
        return Err(MediaOpError::InvalidName(
            "new file name cannot be empty".into(),
        ));
    }

    let old_path = PathBuf::from(&item.path);
    let old_ext = old_path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{e}"))
        .unwrap_or_default();

    let new_file_name = if !old_ext.is_empty()
        && sanitized
            .to_ascii_lowercase()
            .ends_with(&old_ext.to_ascii_lowercase())
    {
        sanitized
    } else {
        format!("{sanitized}{old_ext}")
    };

    let parent = old_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(&item.folder_path));
    let new_path = parent.join(&new_file_name);

    if new_path != old_path && new_path.exists() {
        return Err(MediaOpError::Conflict(
            new_path.to_string_lossy().to_string(),
        ));
    }

    if new_path != old_path && old_path.exists() {
        std::fs::rename(&old_path, &new_path)?;
    }

    let new_path_str = new_path.to_string_lossy().replace('\\', "/");
    let new_folder_str = parent.to_string_lossy().replace('\\', "/");
    let log = RenameLogRecord {
        id: Uuid::new_v4().to_string(),
        media_id: item.id.clone(),
        batch_id: batch_id.map(str::to_string),
        old_name: item.file_name.clone(),
        new_name: new_file_name.clone(),
        old_path: item.path.clone(),
        new_path: new_path_str.clone(),
        applied_at: now_ms(),
        undone_at: None,
    };

    index.insert_rename_log(&log)?;
    index.update_path_and_name(&item.id, &new_path_str, &new_file_name, &new_folder_str)?;

    let updated = index
        .get_media_by_id(&item.id)?
        .ok_or_else(|| MediaOpError::NotFound(item.id))?
        .media;
    Ok((updated, log))
}

pub fn undo_last_rename(
    index: &MediaIndex,
    media_id: Option<&str>,
) -> Result<Option<RenameLogRecord>, MediaOpError> {
    let Some(log) = index.get_latest_undoable_rename(media_id)? else {
        return Ok(None);
    };
    revert_single_log(index, &log)?;
    Ok(Some(log))
}

pub fn undo_bulk_rename(index: &MediaIndex, batch_id: &str) -> Result<u32, MediaOpError> {
    let entries = index.get_rename_batch(batch_id)?;
    let mut count = 0u32;
    for entry in entries {
        revert_single_log(index, &entry)?;
        count += 1;
    }
    Ok(count)
}

fn revert_single_log(index: &MediaIndex, log: &RenameLogRecord) -> Result<(), MediaOpError> {
    let old_p = PathBuf::from(&log.old_path);
    let new_p = PathBuf::from(&log.new_path);
    if old_p != new_p {
        if old_p.exists() {
            return Err(MediaOpError::Conflict(log.old_path.clone()));
        }
        if new_p.exists() {
            std::fs::rename(&new_p, &old_p)?;
        }
    }
    let old_folder = old_p
        .parent()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_default();
    index.update_path_and_name(&log.media_id, &log.old_path, &log.old_name, &old_folder)?;
    index.mark_rename_undone(&log.id)?;
    Ok(())
}

pub fn preview_bulk_rename(
    index: &MediaIndex,
    media_ids: &[String],
    template: &str,
) -> Result<Vec<BulkRenamePreviewItem>, MediaOpError> {
    let mut used_paths: HashSet<String> = HashSet::new();
    let mut out = Vec::new();

    for (idx, media_id) in media_ids.iter().enumerate() {
        let Some(item_wrap) = index.get_media_by_id(media_id)? else {
            continue;
        };
        let item = item_wrap.media;
        let p = Path::new(&item.path);
        let ext = p
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| format!(".{e}"))
            .unwrap_or_default();
        let date_tok = format_date_token(
            if item.created_at > 0 {
                item.created_at
            } else {
                item.modified_at
            },
            true,
        );
        let clean_name = {
            let cleaned = clean_file_name_for_alias(&item.file_name);
            if cleaned.is_empty() {
                p.file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| item.file_name.clone())
            } else {
                cleaned
            }
        };
        let rendered = sanitize_filename_segment(
            &template
                .replace("{date}", &date_tok)
                .replace("{place}", item.place.as_deref().unwrap_or(""))
                .replace("{name}", &clean_name)
                .replace(
                    "{alias}",
                    if item.alias.is_empty() {
                        &clean_name
                    } else {
                        &item.alias
                    },
                )
                .replace(
                    "{device}",
                    item_wrap.device_name.as_deref().unwrap_or(&item.device_id),
                )
                .replace("{index}", &format!("{:03}", idx + 1)),
        );
        let stem = if rendered.is_empty() {
            clean_name
        } else {
            rendered
        };
        let parent = p
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from(&item.folder_path));

        let mut candidate_name = format!("{stem}{ext}");
        let mut candidate_path = parent.join(&candidate_name);
        let mut suffix = 2u32;
        while used_paths.contains(&candidate_path.to_string_lossy().to_ascii_lowercase()) {
            candidate_name = format!("{stem} ({suffix}){ext}");
            candidate_path = parent.join(&candidate_name);
            suffix += 1;
        }
        used_paths.insert(candidate_path.to_string_lossy().to_ascii_lowercase());

        out.push(BulkRenamePreviewItem {
            media_id: item.id,
            old_name: item.file_name,
            new_name: candidate_name,
            old_path: item.path,
            new_path: candidate_path.to_string_lossy().replace('\\', "/"),
        });
    }
    Ok(out)
}

pub fn apply_bulk_rename(
    index: &MediaIndex,
    media_ids: &[String],
    template: &str,
) -> Result<(String, Vec<RenameLogRecord>), MediaOpError> {
    let previews = preview_bulk_rename(index, media_ids, template)?;
    let batch_id = Uuid::new_v4().to_string();
    let mut logs = Vec::with_capacity(previews.len());
    for preview in previews {
        let stem = Path::new(&preview.new_name)
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or(preview.new_name);
        let (_, log) = apply_real_rename(index, &preview.media_id, &stem, Some(&batch_id))?;
        logs.push(log);
    }
    Ok((batch_id, logs))
}

pub fn move_media_to_folder(
    index: &MediaIndex,
    media_id: &str,
    target_folder: &Path,
) -> Result<MediaRecord, MediaOpError> {
    let item = index
        .get_media_by_id(media_id)?
        .ok_or_else(|| MediaOpError::NotFound(media_id.to_string()))?
        .media;

    std::fs::create_dir_all(target_folder)?;
    let src = PathBuf::from(&item.path);
    let dest = target_folder.join(&item.file_name);
    if src != dest {
        if dest.exists() {
            return Err(MediaOpError::Conflict(dest.to_string_lossy().to_string()));
        }
        if src.exists() {
            std::fs::rename(&src, &dest)?;
        }
    }
    let dest_str = dest.to_string_lossy().replace('\\', "/");
    let folder_str = target_folder.to_string_lossy().replace('\\', "/");
    index.update_path_and_name(&item.id, &dest_str, &item.file_name, &folder_str)?;
    Ok(index
        .get_media_by_id(&item.id)?
        .ok_or_else(|| MediaOpError::NotFound(item.id))?
        .media)
}

pub fn move_to_trash(
    index: &MediaIndex,
    trash: &dyn TrashBackend,
    media_id: &str,
) -> Result<String, MediaOpError> {
    let item = index
        .get_media_by_id(media_id)?
        .ok_or_else(|| MediaOpError::NotFound(media_id.to_string()))?
        .media;
    let trashed_path = trash.move_to_trash(Path::new(&item.path))?;
    let trashed_str = trashed_path.to_string_lossy().replace('\\', "/");
    index.mark_media_trashed(&item.id, &trashed_str)?;
    Ok(trashed_str)
}
