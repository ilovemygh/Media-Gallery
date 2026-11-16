use chrono::{DateTime, Local, Timelike, Utc};
use core_index::{MediaTags, MediaType};

use crate::geocoder::lookup_place_offline;

const KNOWN_PREFIXES: &[&str] = &[
    "img_",
    "img-",
    "vid_",
    "vid-",
    "screenshot_",
    "screenshot-",
    "pxl_",
    "pxl-",
    "dcim_",
    "dcim-",
    "mvimg_",
    "win_",
    "aud_",
    "rec_",
];

pub fn format_date_token(timestamp_ms: i64, use_utc: bool) -> String {
    let secs = timestamp_ms.div_euclid(1000);
    if use_utc {
        DateTime::<Utc>::from_timestamp(secs, 0)
            .map(|dt| dt.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "1970-01-01".to_string())
    } else {
        DateTime::<Utc>::from_timestamp(secs, 0)
            .map(|dt| dt.with_timezone(&Local).format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "1970-01-01".to_string())
    }
}

pub fn format_time_period_token(timestamp_ms: i64, use_utc: bool) -> &'static str {
    let secs = timestamp_ms.div_euclid(1000);
    let hour = DateTime::<Utc>::from_timestamp(secs, 0)
        .map(|dt| {
            if use_utc {
                dt.hour()
            } else {
                dt.with_timezone(&Local).hour()
            }
        })
        .unwrap_or(12);
    match hour {
        5..=11 => "morning",
        12..=16 => "afternoon",
        17..=20 => "evening",
        _ => "night",
    }
}

/// Cleans raw filename per PRD Section 13:
/// strips prefixes (`IMG_`, `VID_`, `Screenshot_`), replaces `_`/`-` with spaces,
/// splits `camelCase`, and trims standalone numeric tokens.
pub fn clean_file_name_for_alias(file_name: &str) -> String {
    let base = file_name
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(file_name);
    let stem = match base.rfind('.') {
        Some(idx) if idx > 0 => &base[..idx],
        _ => base,
    };

    let lower = stem.to_ascii_lowercase();
    let mut stripped = stem;
    for prefix in KNOWN_PREFIXES {
        if lower.starts_with(prefix) {
            stripped = &stem[prefix.len()..];
            break;
        }
    }

    let spaced: String = stripped
        .chars()
        .map(|c| if c == '_' || c == '-' { ' ' } else { c })
        .collect();

    // Split camelCase boundaries cleanly
    let chars: Vec<char> = spaced.chars().collect();
    let mut camel_split = String::with_capacity(chars.len() + 8);
    for i in 0..chars.len() {
        let cur = chars[i];
        if i > 0 {
            let prev = chars[i - 1];
            let next_lower = chars.get(i + 1).is_some_and(|n| n.is_ascii_lowercase());
            if ((prev.is_ascii_lowercase() || prev.is_ascii_digit()) && cur.is_ascii_uppercase())
                || (prev.is_ascii_uppercase() && cur.is_ascii_uppercase() && next_lower)
            {
                camel_split.push(' ');
            }
        }
        camel_split.push(cur);
    }

    // Trim standalone numeric tokens (e.g., 2031, 0042)
    let kept_tokens: Vec<&str> = camel_split
        .split_whitespace()
        .filter(|tok| !tok.chars().all(|c| c.is_ascii_digit()))
        .collect();

    kept_tokens.join(" ")
}

#[derive(Debug, Clone)]
pub struct AutoAliasInput<'a> {
    pub file_name: &'a str,
    pub media_type: MediaType,
    pub created_at: i64,
    pub modified_at: i64,
    pub gps_lat: Option<f64>,
    pub gps_lon: Option<f64>,
    pub place: Option<&'a str>,
    pub tags: &'a MediaTags,
    pub device_name: &'a str,
    pub template: Option<&'a str>,
    pub use_utc_date: bool,
}

pub fn generate_auto_alias(input: &AutoAliasInput<'_>) -> (String, Option<String>) {
    let ts = if input.created_at > 0 {
        input.created_at
    } else {
        input.modified_at
    };
    let date_token = format_date_token(ts, input.use_utc_date);
    let time_token = format_time_period_token(ts, input.use_utc_date);

    let resolved_place = input
        .place
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .or_else(|| lookup_place_offline(input.gps_lat, input.gps_lon));

    let name_token = if input.media_type == MediaType::Audio
        && (input.tags.artist.is_some() || input.tags.title.is_some())
    {
        match (&input.tags.artist, &input.tags.title) {
            (Some(a), Some(t)) => format!("{a} - {t}"),
            (_, Some(t)) => t.clone(),
            (Some(a), None) => a.clone(),
            _ => String::new(),
        }
    } else if input.media_type == MediaType::Video
        && (input.tags.source.as_deref() == Some("youtube") || input.tags.youtube_id.is_some())
        && input.tags.title.is_some()
    {
        input.tags.title.clone().unwrap_or_default()
    } else {
        clean_file_name_for_alias(input.file_name)
    };

    let template = input
        .template
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("{date} {place} {name}");

    let is_explicit_audio = input.media_type == MediaType::Audio
        && input.tags.artist.is_some()
        && input.tags.title.is_some()
        && template == "{date} {place} {name}";
    let is_explicit_yt = input.media_type == MediaType::Video
        && (input.tags.source.as_deref() == Some("youtube") || input.tags.youtube_id.is_some())
        && input.tags.title.is_some()
        && template == "{date} {place} {name}";

    let mut rendered = if is_explicit_audio || is_explicit_yt {
        name_token
    } else {
        template
            .replace("{date}", &date_token)
            .replace("{time}", time_token)
            .replace("{place}", resolved_place.as_deref().unwrap_or(""))
            .replace("{name}", &name_token)
            .replace("{device}", input.device_name)
            .replace("{artist}", input.tags.artist.as_deref().unwrap_or(""))
            .replace("{title}", input.tags.title.as_deref().unwrap_or(""))
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    };

    if rendered.is_empty() {
        rendered = input
            .file_name
            .rfind('.')
            .map(|i| &input.file_name[..i])
            .unwrap_or(input.file_name)
            .to_string();
    }

    (rendered, resolved_place)
}
