use std::path::Path;

use core_index::{now_ms, AspectPreset, CoverRecord, CoverSource, CropRect, MediaIndex, MediaType};
use core_platform::ThumbnailSidecar;
use lofty::config::WriteOptions;
use lofty::file::TaggedFileExt;
use lofty::picture::{MimeType, Picture, PictureType};
use lofty::probe::Probe;
use lofty::tag::TagExt;
use uuid::Uuid;

use crate::rename::MediaOpError;
use crate::thumbnails::{select_random_first_third_frame_ms, ThumbnailManager};

/// Computes centered crop bounds for PRD I4 presets (`4:3`, `9:16`, `16:9`, `1:1`, `custom`).
pub fn compute_crop_for_aspect(
    source_width: u32,
    source_height: u32,
    aspect: AspectPreset,
    custom_crop: Option<CropRect>,
) -> CropRect {
    let safe_w = source_width.max(1);
    let safe_h = source_height.max(1);

    if aspect == AspectPreset::Custom {
        if let Some(custom) = custom_crop {
            let w = custom.width.clamp(1, safe_w);
            let h = custom.height.clamp(1, safe_h);
            return CropRect {
                x: custom.x.min(safe_w.saturating_sub(w)),
                y: custom.y.min(safe_h.saturating_sub(h)),
                width: w,
                height: h,
                aspect_ratio: Some(f64::from(w) / f64::from(h)),
            };
        }
    }

    let target_ratio = match aspect {
        AspectPreset::Ratio4x3 => 4.0 / 3.0,
        AspectPreset::Ratio9x16 => 9.0 / 16.0,
        AspectPreset::Ratio16x9 => 16.0 / 9.0,
        AspectPreset::Ratio1x1 => 1.0,
        AspectPreset::Custom => f64::from(safe_w) / f64::from(safe_h),
    };

    let source_ratio = f64::from(safe_w) / f64::from(safe_h);
    let (crop_w, crop_h) = if source_ratio > target_ratio {
        ((f64::from(safe_h) * target_ratio).round() as u32, safe_h)
    } else {
        (safe_w, (f64::from(safe_w) / target_ratio).round() as u32)
    };

    CropRect {
        x: (safe_w.saturating_sub(crop_w)) / 2,
        y: (safe_h.saturating_sub(crop_h)) / 2,
        width: crop_w.max(1),
        height: crop_h.max(1),
        aspect_ratio: Some(target_ratio),
    }
}

pub fn reroll_video_thumbnail(
    index: &MediaIndex,
    sidecar: &dyn ThumbnailSidecar,
    cache_dir: &Path,
    media_id: &str,
    rng_sample: Option<f64>,
) -> Result<CoverRecord, MediaOpError> {
    let media = index
        .get_media_by_id(media_id)?
        .ok_or_else(|| MediaOpError::NotFound(media_id.to_string()))?
        .media;
    let new_ms = select_random_first_third_frame_ms(media.duration_ms, rng_sample);
    pick_video_frame(index, sidecar, cache_dir, media_id, new_ms)
}

pub fn pick_video_frame(
    index: &MediaIndex,
    sidecar: &dyn ThumbnailSidecar,
    cache_dir: &Path,
    media_id: &str,
    frame_time_ms: u64,
) -> Result<CoverRecord, MediaOpError> {
    let media = index
        .get_media_by_id(media_id)?
        .ok_or_else(|| MediaOpError::NotFound(media_id.to_string()))?
        .media;
    let bounded = if media.duration_ms > 0 {
        frame_time_ms.min(media.duration_ms)
    } else {
        0
    };
    let manager = ThumbnailManager {
        index,
        sidecar,
        cache_dir,
        max_cache_bytes: None,
    };
    let file = manager.render_cover_file(&media, bounded)?;
    let existing = index.get_cover_by_media_id(media_id)?;

    let updated = CoverRecord {
        id: existing
            .as_ref()
            .map(|c| c.id.clone())
            .unwrap_or_else(|| Uuid::new_v4().to_string()),
        media_id: media.id,
        source: CoverSource::Generated,
        file_path: file.to_string_lossy().replace('\\', "/"),
        cloud_id: existing.as_ref().and_then(|c| c.cloud_id.clone()),
        aspect: existing
            .as_ref()
            .map(|c| c.aspect)
            .unwrap_or(AspectPreset::Ratio16x9),
        crop: existing.and_then(|c| c.crop),
        frame_time_ms: bounded,
        updated_at: now_ms(),
    };
    index.upsert_cover(&updated)?;
    Ok(updated)
}

pub fn set_custom_cover(
    index: &MediaIndex,
    media_id: &str,
    source_image_path: &Path,
    cache_dir: &Path,
    aspect: AspectPreset,
    custom_crop: Option<CropRect>,
    source_dims: (u32, u32),
) -> Result<CoverRecord, MediaOpError> {
    let _media = index
        .get_media_by_id(media_id)?
        .ok_or_else(|| MediaOpError::NotFound(media_id.to_string()))?;

    std::fs::create_dir_all(cache_dir)?;
    let ext = source_image_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("jpg");
    let dest = cache_dir.join(format!("{media_id}-custom-{}.{ext}", now_ms()));
    std::fs::copy(source_image_path, &dest)?;

    let crop = compute_crop_for_aspect(source_dims.0, source_dims.1, aspect, custom_crop);
    let existing = index.get_cover_by_media_id(media_id)?;

    let updated = CoverRecord {
        id: existing
            .as_ref()
            .map(|c| c.id.clone())
            .unwrap_or_else(|| Uuid::new_v4().to_string()),
        media_id: media_id.to_string(),
        source: CoverSource::Custom,
        file_path: dest.to_string_lossy().replace('\\', "/"),
        cloud_id: existing.and_then(|c| c.cloud_id),
        aspect,
        crop: Some(crop),
        frame_time_ms: 0,
        updated_at: now_ms(),
    };
    index.upsert_cover(&updated)?;
    Ok(updated)
}

/// Replaces embedded audio cover art after explicit user confirmation using `lofty` (PRD I3).
pub fn replace_audio_cover(
    index: &MediaIndex,
    media_id: &str,
    image_path: &Path,
    cache_dir: &Path,
    aspect: AspectPreset,
    custom_crop: Option<CropRect>,
    confirmed: bool,
) -> Result<CoverRecord, MediaOpError> {
    if !confirmed {
        return Err(MediaOpError::ConfirmationRequired(
            "User confirmation is required before writing embedded audio cover art".into(),
        ));
    }

    let mut media = index
        .get_media_by_id(media_id)?
        .ok_or_else(|| MediaOpError::NotFound(media_id.to_string()))?
        .media;
    if media.media_type != MediaType::Audio {
        return Err(MediaOpError::InvalidName(
            "replace_audio_cover only applies to audio items".into(),
        ));
    }

    if media.tags.original_cover_path.is_none() {
        if let Some(existing) = index.get_cover_by_media_id(media_id)? {
            media.tags.original_cover_path = Some(existing.file_path);
            index.upsert_media(&media)?;
        }
    }

    // Best-effort real ID3/MP4/FLAC tag embedding via `lofty` when audio file is a real stream
    let img_bytes = std::fs::read(image_path)?;
    if let Ok(mut tagged) = Probe::open(&media.path).and_then(|p| p.read()) {
        let tag_type = tagged.primary_tag_type();
        let tag_opt = if tagged.tag(tag_type).is_some() {
            tagged.tag_mut(tag_type)
        } else {
            tagged.first_tag_mut()
        };
        if let Some(tag) = tag_opt {
            let mime = if image_path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e.eq_ignore_ascii_case("png"))
            {
                MimeType::Png
            } else {
                MimeType::Jpeg
            };
            let pic = Picture::unchecked(img_bytes)
                .pic_type(PictureType::CoverFront)
                .mime_type(mime)
                .build();
            tag.set_picture(0, pic);
            let _ = tag.save_to_path(&media.path, WriteOptions::default());
        }
    }

    set_custom_cover(
        index,
        media_id,
        image_path,
        cache_dir,
        aspect,
        custom_crop,
        (1000, 1000),
    )
}

pub fn remove_audio_cover(
    index: &MediaIndex,
    media_id: &str,
    confirmed: bool,
) -> Result<(), MediaOpError> {
    if !confirmed {
        return Err(MediaOpError::ConfirmationRequired(
            "User confirmation is required to remove audio cover art".into(),
        ));
    }
    let mut media = index
        .get_media_by_id(media_id)?
        .ok_or_else(|| MediaOpError::NotFound(media_id.to_string()))?
        .media;

    if let Some(mut existing) = index.get_cover_by_media_id(media_id)? {
        if media.tags.original_cover_path.is_none() && !existing.file_path.is_empty() {
            media.tags.original_cover_path = Some(existing.file_path.clone());
            index.upsert_media(&media)?;
        }
        existing.file_path.clear();
        existing.source = CoverSource::Generated;
        existing.updated_at = now_ms();
        index.upsert_cover(&existing)?;
    }
    Ok(())
}

pub fn restore_audio_cover(
    index: &MediaIndex,
    media_id: &str,
) -> Result<Option<CoverRecord>, MediaOpError> {
    let Some(wrap) = index.get_media_by_id(media_id)? else {
        return Ok(None);
    };
    let Some(backup_path) = wrap.media.tags.original_cover_path else {
        return Ok(None);
    };
    let existing = index.get_cover_by_media_id(media_id)?;
    let restored = CoverRecord {
        id: existing
            .as_ref()
            .map(|c| c.id.clone())
            .unwrap_or_else(|| Uuid::new_v4().to_string()),
        media_id: media_id.to_string(),
        source: CoverSource::Embedded,
        file_path: backup_path,
        cloud_id: existing.and_then(|c| c.cloud_id),
        aspect: AspectPreset::Ratio1x1,
        crop: None,
        frame_time_ms: 0,
        updated_at: now_ms(),
    };
    index.upsert_cover(&restored)?;
    Ok(Some(restored))
}
