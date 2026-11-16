//! The ONLY crate exposed to Flutter across `flutter_rust_bridge` v2 (PRD Section 6 & 7).

pub mod api;
pub mod music;

pub use api::{ApiError, ApiErrorCode, CoreConfig, CoreEvent, MediaGalleryCore};
pub use music::{
    build_unified_music_library, group_unified_tracks, MusicGroupBy, MusicPlayerController,
    PlayerState, RepeatMode, UnifiedMusicTrack, UnifiedTrackSource,
};
