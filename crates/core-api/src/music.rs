use std::collections::HashMap;

use core_index::{MediaType, MediaWithCover, SyncState};
use rand::RngExt;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MusicGroupBy {
    All,
    Artist,
    Album,
    Folder,
    Device,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RepeatMode {
    Off,
    One,
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedTrackSource {
    pub media_id: String,
    pub device_id: String,
    pub device_name: String,
    pub sync_state: SyncState,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedMusicTrack {
    pub canonical_key: String,
    pub primary: MediaWithCover,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: u64,
    pub sources: Vec<UnifiedTrackSource>,
}

fn canonical_key(item: &MediaWithCover) -> String {
    let m = &item.media;
    if !m.blake3.is_empty() && !m.blake3.starts_with("seed-") {
        return format!("b3:{}", m.blake3);
    }
    if let (Some(artist), Some(title)) = (&m.tags.artist, &m.tags.title) {
        let dur_bucket = m.duration_ms / 2000;
        return format!(
            "meta:{}::{}::{dur_bucket}",
            artist.trim().to_ascii_lowercase(),
            title.trim().to_ascii_lowercase()
        );
    }
    format!("file:{}::{}", m.file_name.to_ascii_lowercase(), m.size)
}

/// Merges and de-duplicates PC, Phone, and YouTube tracks while retaining all source devices (PRD F2, F5).
pub fn build_unified_music_library(items: &[MediaWithCover]) -> Vec<UnifiedMusicTrack> {
    let mut order = Vec::new();
    let mut by_key: HashMap<String, UnifiedMusicTrack> = HashMap::new();

    for item in items {
        if item.media.media_type != MediaType::Audio || item.media.is_trashed {
            continue;
        }
        let key = canonical_key(item);
        let source = UnifiedTrackSource {
            media_id: item.media.id.clone(),
            device_id: item.media.device_id.clone(),
            device_name: item
                .device_name
                .clone()
                .unwrap_or_else(|| item.media.device_id.clone()),
            sync_state: item.media.sync_state,
            path: item.media.path.clone(),
        };

        if let Some(existing) = by_key.get_mut(&key) {
            existing.sources.push(source);
            if existing.primary.media.sync_state == SyncState::Remote
                && item.media.sync_state != SyncState::Remote
            {
                existing.primary = item.clone();
            }
        } else {
            let title = item
                .media
                .tags
                .title
                .clone()
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| {
                    if item.media.alias.is_empty() {
                        item.media.file_name.clone()
                    } else {
                        item.media.alias.clone()
                    }
                });
            let artist = item
                .media
                .tags
                .artist
                .clone()
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "Unknown Artist".to_string());
            let album = item
                .media
                .tags
                .album
                .clone()
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "Singles & Downloads".to_string());

            order.push(key.clone());
            by_key.insert(
                key.clone(),
                UnifiedMusicTrack {
                    canonical_key: key,
                    primary: item.clone(),
                    title,
                    artist,
                    album,
                    duration_ms: item.media.duration_ms,
                    sources: vec![source],
                },
            );
        }
    }

    order
        .into_iter()
        .filter_map(|k| by_key.remove(&k))
        .collect()
}

pub fn group_unified_tracks(
    tracks: &[UnifiedMusicTrack],
    group_by: MusicGroupBy,
) -> Vec<(String, Vec<UnifiedMusicTrack>)> {
    if group_by == MusicGroupBy::All {
        return vec![("All Tracks".to_string(), tracks.to_vec())];
    }
    let mut order = Vec::new();
    let mut map: HashMap<String, Vec<UnifiedMusicTrack>> = HashMap::new();
    for track in tracks {
        let key = match group_by {
            MusicGroupBy::All => "All Tracks".to_string(),
            MusicGroupBy::Artist => track.artist.clone(),
            MusicGroupBy::Album => track.album.clone(),
            MusicGroupBy::Folder => track.primary.media.folder_path.clone(),
            MusicGroupBy::Device => track
                .sources
                .iter()
                .map(|s| s.device_name.as_str())
                .collect::<Vec<_>>()
                .join(" + "),
        };
        if !map.contains_key(&key) {
            order.push(key.clone());
        }
        map.entry(key).or_default().push(track.clone());
    }
    order
        .into_iter()
        .filter_map(|k| map.remove(&k).map(|v| (k, v)))
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerState {
    pub queue: Vec<UnifiedMusicTrack>,
    pub current_index: Option<usize>,
    pub is_playing: bool,
    pub position_ms: u64,
    pub volume: f32,
    pub shuffle: bool,
    pub repeat: RepeatMode,
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            queue: Vec::new(),
            current_index: None,
            is_playing: false,
            position_ms: 0,
            volume: 0.85,
            shuffle: false,
            repeat: RepeatMode::Off,
        }
    }
}

#[derive(Default)]
pub struct MusicPlayerController {
    state: PlayerState,
}

impl MusicPlayerController {
    pub fn state(&self) -> &PlayerState {
        &self.state
    }

    pub fn play_list(&mut self, tracks: Vec<UnifiedMusicTrack>, start_index: usize) {
        if tracks.is_empty() {
            self.state.queue.clear();
            self.state.current_index = None;
            self.state.is_playing = false;
            return;
        }
        self.state.current_index = Some(start_index.min(tracks.len() - 1));
        self.state.queue = tracks;
        self.state.is_playing = true;
        self.state.position_ms = 0;
    }

    pub fn toggle_play(&mut self) {
        if !self.state.queue.is_empty() {
            self.state.is_playing = !self.state.is_playing;
        }
    }

    pub fn seek(&mut self, pos_ms: u64) {
        let max_dur = self
            .current_track()
            .map(|t| t.duration_ms)
            .unwrap_or(pos_ms);
        self.state.position_ms = pos_ms.min(max_dur);
    }

    pub fn set_volume(&mut self, vol: f32) {
        self.state.volume = vol.clamp(0.0, 1.0);
    }

    pub fn toggle_shuffle(&mut self) {
        self.state.shuffle = !self.state.shuffle;
    }

    pub fn cycle_repeat(&mut self) {
        self.state.repeat = match self.state.repeat {
            RepeatMode::Off => RepeatMode::All,
            RepeatMode::All => RepeatMode::One,
            RepeatMode::One => RepeatMode::Off,
        };
    }

    pub fn current_track(&self) -> Option<&UnifiedMusicTrack> {
        self.state
            .current_index
            .and_then(|idx| self.state.queue.get(idx))
    }

    pub fn next(&mut self) {
        let len = self.state.queue.len();
        if len == 0 {
            return;
        }
        let cur = self.state.current_index.unwrap_or(0);
        if self.state.repeat == RepeatMode::One {
            self.state.position_ms = 0;
            self.state.is_playing = true;
            return;
        }
        if self.state.shuffle && len > 1 {
            let mut idx = rand::rng().random_range(0..len);
            if idx == cur {
                idx = (idx + 1) % len;
            }
            self.state.current_index = Some(idx);
            self.state.position_ms = 0;
            self.state.is_playing = true;
            return;
        }
        if cur + 1 < len {
            self.state.current_index = Some(cur + 1);
            self.state.position_ms = 0;
            self.state.is_playing = true;
        } else if self.state.repeat == RepeatMode::All {
            self.state.current_index = Some(0);
            self.state.position_ms = 0;
            self.state.is_playing = true;
        } else {
            self.state.is_playing = false;
            self.state.position_ms = 0;
        }
    }

    pub fn previous(&mut self) {
        let len = self.state.queue.len();
        if len == 0 {
            return;
        }
        if self.state.position_ms > 3000 {
            self.state.position_ms = 0;
            return;
        }
        let cur = self.state.current_index.unwrap_or(0);
        let prev = if cur > 0 { cur - 1 } else { len - 1 };
        self.state.current_index = Some(prev);
        self.state.position_ms = 0;
        self.state.is_playing = true;
    }
}
