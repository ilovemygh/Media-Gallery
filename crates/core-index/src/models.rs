use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MediaType {
    Video,
    Image,
    Audio,
}

impl MediaType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Video => "video",
            Self::Image => "image",
            Self::Audio => "audio",
        }
    }

    pub fn parse(raw: &str) -> Self {
        match raw.to_ascii_lowercase().as_str() {
            "video" => Self::Video,
            "audio" => Self::Audio,
            _ => Self::Image,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncState {
    Local,
    Remote,
    Downloading,
    Synced,
    Error,
}

impl SyncState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Local => "Local",
            Self::Remote => "Remote",
            Self::Downloading => "Downloading",
            Self::Synced => "Synced",
            Self::Error => "Error",
        }
    }

    pub fn parse(raw: &str) -> Self {
        match raw {
            "Remote" => Self::Remote,
            "Downloading" => Self::Downloading,
            "Synced" => Self::Synced,
            "Error" => Self::Error,
            _ => Self::Local,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeviceType {
    Phone,
    Pc,
}

impl DeviceType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Phone => "phone",
            Self::Pc => "pc",
        }
    }

    pub fn parse(raw: &str) -> Self {
        if raw.eq_ignore_ascii_case("phone") {
            Self::Phone
        } else {
            Self::Pc
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportKind {
    Wired,
    WifiLan,
    HolePunched,
    Relay,
    Bluetooth,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CoverSource {
    Generated,
    Custom,
    Embedded,
}

impl CoverSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Generated => "generated",
            Self::Custom => "custom",
            Self::Embedded => "embedded",
        }
    }

    pub fn parse(raw: &str) -> Self {
        match raw {
            "custom" => Self::Custom,
            "embedded" => Self::Embedded,
            _ => Self::Generated,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AspectPreset {
    #[serde(rename = "4:3")]
    Ratio4x3,
    #[serde(rename = "9:16")]
    Ratio9x16,
    #[serde(rename = "16:9")]
    Ratio16x9,
    #[serde(rename = "1:1")]
    Ratio1x1,
    #[serde(rename = "custom")]
    Custom,
}

impl AspectPreset {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ratio4x3 => "4:3",
            Self::Ratio9x16 => "9:16",
            Self::Ratio16x9 => "16:9",
            Self::Ratio1x1 => "1:1",
            Self::Custom => "custom",
        }
    }

    pub fn parse(raw: &str) -> Self {
        match raw {
            "4:3" => Self::Ratio4x3,
            "9:16" => Self::Ratio9x16,
            "1:1" => Self::Ratio1x1,
            "custom" => Self::Custom,
            _ => Self::Ratio16x9,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CropRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub aspect_ratio: Option<f64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct MediaTags {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub uploader: Option<String>,
    pub source: Option<String>,
    pub youtube_id: Option<String>,
    pub original_cover_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeviceRecord {
    pub id: String,
    pub name: String,
    pub device_type: DeviceType,
    pub public_key: String,
    pub last_seen: i64,
    pub transports: Vec<TransportKind>,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub is_self: bool,
    pub is_online: bool,
    pub paired_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MediaRecord {
    pub id: String,
    pub device_id: String,
    pub path: String,
    pub folder_path: String,
    pub file_name: String,
    pub media_type: MediaType,
    pub mime: String,
    pub size: u64,
    pub blake3: String,
    pub partial_hash: String,
    pub created_at: i64,
    pub modified_at: i64,
    pub duration_ms: u64,
    pub width: u32,
    pub height: u32,
    pub gps_lat: Option<f64>,
    pub gps_lon: Option<f64>,
    pub place: Option<String>,
    pub tags: MediaTags,
    pub alias: String,
    pub auto_alias: String,
    pub cover_id: Option<String>,
    pub sync_state: SyncState,
    pub is_duplicate: bool,
    pub duplicate_group_id: Option<String>,
    pub is_trashed: bool,
    pub trashed_path: Option<String>,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CoverRecord {
    pub id: String,
    pub media_id: String,
    pub source: CoverSource,
    pub file_path: String,
    pub cloud_id: Option<String>,
    pub aspect: AspectPreset,
    pub crop: Option<CropRect>,
    pub frame_time_ms: u64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MediaWithCover {
    pub media: MediaRecord,
    pub cover: Option<CoverRecord>,
    pub device_name: Option<String>,
    pub device_online: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransferStatus {
    Queued,
    Transferring,
    Paused,
    Verifying,
    Completed,
    Cancelled,
    Error,
}

impl TransferStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Transferring => "transferring",
            Self::Paused => "paused",
            Self::Verifying => "verifying",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
            Self::Error => "error",
        }
    }

    pub fn parse(raw: &str) -> Self {
        match raw {
            "transferring" => Self::Transferring,
            "paused" => Self::Paused,
            "verifying" => Self::Verifying,
            "completed" => Self::Completed,
            "cancelled" => Self::Cancelled,
            "error" => Self::Error,
            _ => Self::Queued,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TransferRecord {
    pub id: String,
    pub media_id: String,
    pub file_name: String,
    pub from_device: String,
    pub to_device: String,
    pub status: TransferStatus,
    pub bytes_done: u64,
    pub bytes_total: u64,
    pub last_chunk_index: u64,
    pub chunk_size: u64,
    pub path_kind: String,
    pub speed_bps: u64,
    pub expected_blake3: String,
    pub error: Option<String>,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PinnedFolderRecord {
    pub id: String,
    pub device_id: String,
    pub path: String,
    pub label: String,
    pub icon: String,
    pub position: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RenameLogRecord {
    pub id: String,
    pub media_id: String,
    pub batch_id: Option<String>,
    pub old_name: String,
    pub new_name: String,
    pub old_path: String,
    pub new_path: String,
    pub applied_at: i64,
    pub undone_at: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DownloadFormat {
    Mp4,
    Mp3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DownloadStatus {
    Queued,
    Downloading,
    Processing,
    Completed,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DownloadJobRecord {
    pub id: String,
    pub url: String,
    pub title: Option<String>,
    pub format: DownloadFormat,
    pub quality: String,
    pub destination_device_id: String,
    pub destination_folder: String,
    pub status: DownloadStatus,
    pub progress: f32,
    pub output_media_id: Option<String>,
    pub error: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SavedSearchRecord {
    pub id: String,
    pub name: String,
    pub query: String,
    pub filter: MediaFilter,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AutoSyncRuleRecord {
    pub id: String,
    pub name: String,
    pub source_device_id: String,
    pub target_device_id: String,
    pub folder_pattern: String,
    pub media_types: Vec<MediaType>,
    pub enabled: bool,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppSettings {
    pub sidebar_width: u32,
    pub sidebar_collapsed: bool,
    pub theme: String,
    pub transport_priority: Vec<TransportKind>,
    pub manual_transport_override: Option<TransportKind>,
    pub custom_relay_url: Option<String>,
    pub alias_template: String,
    pub scan_folders: Vec<String>,
    pub excluded_folders: Vec<String>,
    pub online_geocoding_enabled: bool,
    pub youtube_default_video_folder: String,
    pub youtube_default_audio_folder: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            sidebar_width: 248,
            sidebar_collapsed: false,
            theme: "dark".to_string(),
            transport_priority: vec![
                TransportKind::Wired,
                TransportKind::WifiLan,
                TransportKind::HolePunched,
                TransportKind::Relay,
                TransportKind::Bluetooth,
            ],
            manual_transport_override: None,
            custom_relay_url: None,
            alias_template: "{date} {place} {name}".to_string(),
            scan_folders: Vec::new(),
            excluded_folders: Vec::new(),
            online_geocoding_enabled: false,
            youtube_default_video_folder: "Videos/YouTube".to_string(),
            youtube_default_audio_folder: "Music/YouTube".to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TimeOfDayFilter {
    All,
    Morning,
    Afternoon,
    Evening,
    Night,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct MediaFilter {
    pub query: String,
    pub media_type: Option<MediaType>,
    pub device_id: Option<String>,
    pub sync_state: Option<SyncState>,
    pub folder_path: Option<String>,
    pub date_from: Option<i64>,
    pub date_to: Option<i64>,
    pub time_of_day: Option<TimeOfDayFilter>,
    pub min_size: Option<u64>,
    pub max_size: Option<u64>,
    pub min_duration_ms: Option<u64>,
    pub min_width: Option<u32>,
    pub include_trashed: bool,
    pub only_duplicates: bool,
    pub trashed_only: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct StorageStats {
    pub total_files: u64,
    pub total_bytes: u64,
    pub video_count: u64,
    pub video_bytes: u64,
    pub image_count: u64,
    pub image_bytes: u64,
    pub audio_count: u64,
    pub audio_bytes: u64,
    pub duplicate_groups: u64,
    pub duplicate_extra_bytes: u64,
    pub trashed_count: u64,
    pub trashed_bytes: u64,
    pub large_files_count: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortField {
    Name,
    Date,
    Size,
    Duration,
    Type,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortDir {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SortSpec {
    pub field: SortField,
    pub dir: SortDir,
}

impl Default for SortSpec {
    fn default() -> Self {
        Self {
            field: SortField::Date,
            dir: SortDir::Desc,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaPage {
    pub items: Vec<MediaWithCover>,
    pub next_cursor: Option<String>,
    pub total_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderSummary {
    pub path: String,
    pub name: String,
    pub device_id: String,
    pub item_count: u64,
    pub preview_covers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateGroup {
    pub group_id: String,
    pub size: u64,
    pub partial_hash: String,
    pub items: Vec<MediaWithCover>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanProgress {
    pub operation_id: String,
    pub scanned_files: u64,
    pub added_files: u64,
    pub updated_files: u64,
    pub skipped_files: u64,
    pub current_folder: String,
    pub in_progress: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexDelta {
    pub device: DeviceRecord,
    pub generated_at: i64,
    pub since_timestamp: i64,
    pub upserted_media: Vec<MediaRecord>,
    pub upserted_covers: Vec<CoverRecord>,
    pub deleted_media_ids: Vec<String>,
    pub thumbnail_blobs_base64: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LegacyImportReport {
    pub imported_devices: u64,
    pub imported_media: u64,
    pub imported_covers: u64,
    pub imported_pinned_folders: u64,
    pub imported_rename_logs: u64,
    pub imported_saved_searches: u64,
    pub imported_auto_sync_rules: u64,
    pub skipped_items: u64,
    pub auth_relogin_required: bool,
    pub errors: Vec<String>,
}
