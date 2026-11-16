//! Auto-alias rules, offline reverse geocoding, EXIF/ID3 metadata extraction,
//! undoable disk rename/trash operations, fixed first-third video thumbnails, and cover cropping.

pub mod alias;
pub mod covers;
pub mod geocoder;
pub mod metadata;
pub mod rename;
pub mod thumbnails;

pub use alias::{
    clean_file_name_for_alias, format_date_token, format_time_period_token, generate_auto_alias,
    AutoAliasInput,
};
pub use covers::{
    compute_crop_for_aspect, pick_video_frame, remove_audio_cover, replace_audio_cover,
    reroll_video_thumbnail, restore_audio_cover, set_custom_cover,
};
pub use geocoder::lookup_place_offline;
pub use metadata::extract_metadata_and_alias;
pub use rename::{
    apply_bulk_rename, apply_real_rename, move_media_to_folder, move_to_trash, preview_bulk_rename,
    sanitize_filename_segment, set_alias, undo_bulk_rename, undo_last_rename,
    BulkRenamePreviewItem, MediaOpError,
};
pub use thumbnails::{
    enforce_lru_cache_limit, select_random_first_third_frame_ms, ThumbnailManager,
};
