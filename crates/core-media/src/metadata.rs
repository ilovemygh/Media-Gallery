use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use core_index::{MediaTags, MediaType, ProbedMediaMetadata};
use core_platform::ThumbnailSidecar;
use exif::{In, Reader, Tag, Value};
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::Accessor;

use crate::alias::{generate_auto_alias, AutoAliasInput};

/// Extracts EXIF (dates, GPS), audio tags/duration (`lofty`), image dimensions,
/// and video metadata (`ThumbnailSidecar`) for scanner enrichment.
pub fn extract_metadata_and_alias(
    path: &Path,
    media_type: MediaType,
    fs_created_at: i64,
    modified_at: i64,
    device_name: &str,
    template: Option<&str>,
    sidecar: Option<&dyn ThumbnailSidecar>,
) -> ProbedMediaMetadata {
    let mut probed = ProbedMediaMetadata::default();

    match media_type {
        MediaType::Image => {
            let dims = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                image::image_dimensions(path)
            }));
            if let Ok(Ok((w, h))) = dims {
                probed.width = w;
                probed.height = h;
            }
            if let Ok(file) = File::open(path) {
                let mut reader = BufReader::new(file);
                let exif_res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    Reader::new().read_from_container(&mut reader)
                }));
                if let Ok(Ok(exif)) = exif_res {
                    probed.created_at = parse_exif_datetime(&exif);
                    if let (Some(lat), Some(lon)) = parse_exif_gps(&exif) {
                        probed.gps_lat = Some(lat);
                        probed.gps_lon = Some(lon);
                    }
                }
            }
        }
        MediaType::Audio => {
            let audio_res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                Probe::open(path).and_then(|p| p.read())
            }));
            if let Ok(Ok(tagged)) = audio_res {
                probed.duration_ms = tagged.properties().duration().as_millis() as u64;
                if let Some(tag) = tagged.primary_tag().or_else(|| tagged.first_tag()) {
                    probed.tags = MediaTags {
                        title: tag.title().map(|s| s.to_string()),
                        artist: tag.artist().map(|s| s.to_string()),
                        album: tag.album().map(|s| s.to_string()),
                        ..Default::default()
                    };
                }
            }
        }
        MediaType::Video => {
            if let Some(sc) = sidecar {
                let vid_res =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| sc.probe_video(path)));
                if let Ok(Ok(vm)) = vid_res {
                    probed.duration_ms = vm.duration_ms;
                    probed.width = vm.width;
                    probed.height = vm.height;
                    probed.created_at = vm.created_at_ms;
                }
            }
        }
    }

    let effective_created = probed.created_at.unwrap_or(fs_created_at);
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let (auto_alias, place) = generate_auto_alias(&AutoAliasInput {
        file_name: &file_name,
        media_type,
        created_at: effective_created,
        modified_at,
        gps_lat: probed.gps_lat,
        gps_lon: probed.gps_lon,
        place: None,
        tags: &probed.tags,
        device_name,
        template,
        use_utc_date: true,
    });

    probed.place = place;
    probed.auto_alias = Some(auto_alias);
    probed
}

fn parse_exif_datetime(exif: &exif::Exif) -> Option<i64> {
    let field = exif
        .get_field(Tag::DateTimeOriginal, In::PRIMARY)
        .or_else(|| exif.get_field(Tag::DateTime, In::PRIMARY))?;
    let text = field.display_value().to_string();
    chrono::NaiveDateTime::parse_from_str(text.trim(), "%Y-%m-%d %H:%M:%S")
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(text.trim(), "%Y:%m:%d %H:%M:%S"))
        .ok()
        .map(|ndt| ndt.and_utc().timestamp_millis())
}

fn parse_exif_gps(exif: &exif::Exif) -> (Option<f64>, Option<f64>) {
    let lat = parse_dms(
        exif.get_field(Tag::GPSLatitude, In::PRIMARY),
        exif.get_field(Tag::GPSLatitudeRef, In::PRIMARY),
        'S',
    );
    let lon = parse_dms(
        exif.get_field(Tag::GPSLongitude, In::PRIMARY),
        exif.get_field(Tag::GPSLongitudeRef, In::PRIMARY),
        'W',
    );
    (lat, lon)
}

fn parse_dms(
    coord: Option<&exif::Field>,
    hemisphere: Option<&exif::Field>,
    neg_char: char,
) -> Option<f64> {
    let field = coord?;
    let Value::Rational(ref rationals) = field.value else {
        return None;
    };
    if rationals.len() < 3 {
        return None;
    }
    let deg = rationals[0].to_f64();
    let min = rationals[1].to_f64();
    let sec = rationals[2].to_f64();
    let mut decimal = deg + min / 60.0 + sec / 3600.0;
    if let Some(hemi) = hemisphere {
        let s = hemi.display_value().to_string();
        if s.trim().starts_with(neg_char) {
            decimal = -decimal;
        }
    }
    Some(decimal)
}
