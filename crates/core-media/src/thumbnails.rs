use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use core_index::{
    now_ms, AspectPreset, CoverRecord, CoverSource, MediaIndex, MediaRecord, MediaType,
};
use core_platform::ThumbnailSidecar;
use image::{ImageBuffer, Rgba};
use lofty::file::TaggedFileExt;
use lofty::probe::Probe;
use rand::RngExt;
use uuid::Uuid;

use crate::rename::MediaOpError;

/// Selects a random frame timestamp inside the first third `[0, duration_ms / 3]` (PRD I1).
pub fn select_random_first_third_frame_ms(duration_ms: u64, sample_0_to_1: Option<f64>) -> u64 {
    if duration_ms == 0 {
        return 0;
    }
    let first_third = duration_ms / 3;
    if first_third == 0 {
        return 0;
    }
    let sample = match sample_0_to_1 {
        Some(s) => s.clamp(0.0, 0.999_999_999),
        None => rand::rng().random_range(0.0..1.0),
    };
    (sample * ((first_third + 1) as f64)).floor() as u64
}

pub struct ThumbnailManager<'a> {
    pub index: &'a MediaIndex,
    pub sidecar: &'a dyn ThumbnailSidecar,
    pub cache_dir: &'a Path,
    pub max_cache_bytes: Option<u64>,
}

impl<'a> ThumbnailManager<'a> {
    /// Generates and persists a thumbnail if not already cached.
    /// Once a video thumbnail is chosen, subsequent restarts return the fixed cached cover (PRD I1).
    pub fn ensure_thumbnail(
        &self,
        media: &MediaRecord,
        rng_sample: Option<f64>,
    ) -> Result<CoverRecord, MediaOpError> {
        let resolved_media_id = if let Some(existing) = self.index.get_media_by_id(&media.id)? {
            if let Some(cov) = existing.cover {
                return Ok(cov);
            }
            existing.media.id
        } else if let Some(by_path) = self
            .index
            .get_media_by_path(&media.device_id, &media.path)?
        {
            if let Some(cov) = self.index.get_cover_by_media_id(&by_path.id)? {
                return Ok(cov);
            }
            by_path.id
        } else {
            if let Some(cov) = self.index.get_cover_by_media_id(&media.id)? {
                return Ok(cov);
            }
            media.id.clone()
        };

        std::fs::create_dir_all(self.cache_dir)?;
        let frame_time_ms = if media.media_type == MediaType::Video {
            select_random_first_third_frame_ms(media.duration_ms, rng_sample)
        } else {
            0
        };

        let out_file = self.render_cover_file(media, frame_time_ms)?;
        let out_str = out_file.to_string_lossy().replace('\\', "/");
        let aspect = if media.media_type == MediaType::Audio {
            AspectPreset::Ratio1x1
        } else {
            AspectPreset::Ratio16x9
        };
        let source =
            if media.media_type == MediaType::Audio && media.tags.original_cover_path.is_some() {
                CoverSource::Embedded
            } else {
                CoverSource::Generated
            };

        let cover = CoverRecord {
            id: Uuid::new_v4().to_string(),
            media_id: resolved_media_id,
            source,
            file_path: out_str,
            cloud_id: None,
            aspect,
            crop: None,
            frame_time_ms,
            updated_at: now_ms(),
        };
        self.index.upsert_cover(&cover)?;

        if let Some(limit) = self.max_cache_bytes {
            let _ = enforce_lru_cache_limit(self.cache_dir, limit, &[out_file]);
        }

        Ok(cover)
    }

    pub fn render_cover_file(
        &self,
        media: &MediaRecord,
        frame_time_ms: u64,
    ) -> Result<PathBuf, MediaOpError> {
        std::fs::create_dir_all(self.cache_dir)?;
        let stem = format!("{}-{frame_time_ms}", media.id);

        match media.media_type {
            MediaType::Video => {
                let target = self.cache_dir.join(format!("{stem}.jpg"));
                Ok(self.sidecar.extract_video_frame(
                    Path::new(&media.path),
                    frame_time_ms,
                    &target,
                )?)
            }
            MediaType::Image => {
                let target = self.cache_dir.join(format!("{stem}.jpg"));
                if let Ok(img) = image::open(&media.path) {
                    let thumb = img.thumbnail(480, 480);
                    if thumb.save(&target).is_ok() {
                        return Ok(target);
                    }
                }
                write_deterministic_png(
                    &self.cache_dir.join(format!("{stem}.png")),
                    &media.file_name,
                    [15, 118, 110],
                )
            }
            MediaType::Audio => {
                if let Ok(tagged) = Probe::open(&media.path).and_then(|p| p.read()) {
                    if let Some(pic) = tagged
                        .primary_tag()
                        .or_else(|| tagged.first_tag())
                        .and_then(|t| t.pictures().first())
                    {
                        let target = self.cache_dir.join(format!("{stem}.jpg"));
                        std::fs::write(&target, pic.data())?;
                        return Ok(target);
                    }
                }
                write_deterministic_png(
                    &self.cache_dir.join(format!("{stem}.png")),
                    &media.file_name,
                    [124, 58, 237],
                )
            }
        }
    }
}

fn write_deterministic_png(
    out_path: &Path,
    seed: &str,
    rgb: [u8; 3],
) -> Result<PathBuf, MediaOpError> {
    let hash = blake3::hash(seed.as_bytes());
    let b = hash.as_bytes();
    let img = ImageBuffer::from_fn(240, 240, |x, y| {
        let r = rgb[0].saturating_add(((x ^ b[0] as u32) % 40) as u8);
        let g = rgb[1].saturating_add(((y ^ b[1] as u32) % 40) as u8);
        let bl = rgb[2].saturating_add(((x + y) % 40) as u8);
        Rgba([r, g, bl, 255])
    });
    img.save(out_path)
        .map_err(|e| MediaOpError::InvalidName(e.to_string()))?;
    Ok(out_path.to_path_buf())
}

/// Evicts least-recently-accessed thumbnail files above `max_bytes`.
pub fn enforce_lru_cache_limit(
    cache_dir: &Path,
    max_bytes: u64,
    protected: &[PathBuf],
) -> std::io::Result<usize> {
    let mut files = Vec::new();
    let mut total_bytes = 0u64;
    for entry in std::fs::read_dir(cache_dir)? {
        let entry = entry?;
        let meta = entry.metadata()?;
        if !meta.is_file() {
            continue;
        }
        let len = meta.len();
        total_bytes += len;
        let accessed = meta
            .accessed()
            .or_else(|_| meta.modified())
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        files.push((entry.path(), len, accessed));
    }

    if total_bytes <= max_bytes {
        return Ok(0);
    }

    files.sort_by_key(|(_, _, ts)| *ts);
    let mut evicted = 0usize;
    for (path, len, _) in files {
        if total_bytes <= max_bytes {
            break;
        }
        if protected.iter().any(|p| p == &path) {
            continue;
        }
        if std::fs::remove_file(&path).is_ok() {
            total_bytes = total_bytes.saturating_sub(len);
            evicted += 1;
        }
    }
    Ok(evicted)
}
