# Rust <-> Flutter FFI Contract (`docs/API.md`)

Only `crates/core-api` is exposed across `flutter_rust_bridge` v2 (`frb`). All heavy I/O, hashing, scanning, SQLite queries, networking, and thumbnail generation run inside Rust's `tokio` / `rayon` worker pools without blocking Dart's UI isolate.

---

## 1. Contract Rules (PRD Section 7)

1. **Coarse-grained async functions only:** No per-file FFI round-trips while rendering grids or scrolling.
2. **Paged queries:** `list_media(filter, sort, cursor, limit)` returns `MediaPage { items, next_cursor, total_count }`.
3. **Event streams:** Progress, transfer state, download progress, and device status emit via FRB `StreamSink` backed by `tokio::sync::broadcast`.
4. **Thumbnails by file path:** Rust writes thumbnails to the local cache directory and passes `cover_path: Option<String>` to Flutter (`Image.file`). Raw image byte buffers are never pushed across FFI during scrolling.
5. **Typed errors (`ApiError`):** Every public function returns `Result<T, ApiError>`. Panics across FFI are forbidden.
6. **Cancellation:** Long-running scans, thumbnail batches, transfers, and downloads accept `cancel_token_id` and stop cleanly via `cancel_operation(op_id)`.

---

## 2. Typed Error Model

```rust
pub enum ApiErrorCode {
    InvalidInput,
    NotFound,
    AlreadyExists,
    IoFailure,
    DatabaseFailure,
    NetworkFailure,
    UnpairedPeer,
    PeerOffline,
    HashMismatch,
    ConfirmationRequired,
    Cancelled,
    Internal,
}

pub struct ApiError {
    pub code: ApiErrorCode,
    pub message: String,
}
```

---

## 3. Exposed Functions (`core_api::api`)

### Lifecycle & Event Subscription
- `init_core(config: CoreInitConfig) -> Result<CoreStatus, ApiError>`
- `subscribe_events(sink: StreamSink<CoreEvent>) -> Result<(), ApiError>`
- `cancel_operation(operation_id: String) -> Result<bool, ApiError>`
- `shutdown_core() -> Result<(), ApiError>`

### Scanning, Library & Search
- `start_scan(roots: Vec<String>, excluded: Vec<String>) -> Result<String /* operation_id */, ApiError>`
- `list_media(filter: MediaFilterDto, sort: SortSpecDto, cursor: Option<String>, limit: u32) -> Result<MediaPageDto, ApiError>`
- `list_folders(device_id: Option<String>, media_type: Option<MediaTypeDto>) -> Result<Vec<FolderSummaryDto>, ApiError>`
- `detect_duplicates(device_id: Option<String>) -> Result<Vec<DuplicateGroupDto>, ApiError>`
- `ingest_android_mediastore_batch(device_id: String, device_name: String, rows: Vec<AndroidMediaRowDto>) -> Result<u32, ApiError>`
- `save_search(name: String, query: String, filter: MediaFilterDto) -> Result<SavedSearchDto, ApiError>`
- `list_saved_searches() -> Result<Vec<SavedSearchDto>, ApiError>`

### Alias, Rename, Trash & Covers
- `set_alias(media_id: String, alias: String) -> Result<MediaItemDto, ApiError>`
- `apply_real_rename(media_id: String, new_name: String) -> Result<MediaItemDto, ApiError>`
- `undo_last_rename(media_id: Option<String>) -> Result<Option<RenameLogDto>, ApiError>`
- `preview_bulk_rename(media_ids: Vec<String>, template: String) -> Result<Vec<BulkRenamePreviewDto>, ApiError>`
- `apply_bulk_rename(media_ids: Vec<String>, template: String) -> Result<BulkRenameResultDto, ApiError>`
- `undo_bulk_rename(batch_id: String) -> Result<u32, ApiError>`
- `move_media_to_folder(media_id: String, target_folder: String) -> Result<MediaItemDto, ApiError>`
- `move_to_trash(media_id: String) -> Result<String, ApiError>`
- `ensure_thumbnail(media_id: String) -> Result<CoverDto, ApiError>`
- `reroll_video_thumbnail(media_id: String) -> Result<CoverDto, ApiError>`
- `pick_video_frame(media_id: String, frame_time_ms: u64) -> Result<CoverDto, ApiError>`
- `set_custom_cover(media_id: String, image_path: String, aspect: AspectPresetDto, crop: Option<CropRectDto>, confirmed_embedded_write: bool) -> Result<CoverDto, ApiError>`
- `remove_audio_cover(media_id: String, confirmed: bool) -> Result<(), ApiError>`
- `restore_audio_cover(media_id: String) -> Result<Option<CoverDto>, ApiError>`

### P2P Pairing, Transfer & Serverless Sync (`iroh`)
- `create_pairing_session() -> Result<PairingSessionDto, ApiError>`
- `pair_with_pin_or_qr(payload_or_pin: String, peer_info: Option<DeviceDto>) -> Result<DeviceDto, ApiError>`
- `list_devices() -> Result<Vec<DeviceDto>, ApiError>`
- `forget_device(device_id: String) -> Result<(), ApiError>`
- `send_media_to_device(media_id: String, target_device_id: String) -> Result<TransferDto, ApiError>`
- `download_remote_media(media_id: String) -> Result<TransferDto, ApiError>`
- `pause_transfer(transfer_id: String) -> Result<(), ApiError>`
- `resume_transfer(transfer_id: String) -> Result<TransferDto, ApiError>`
- `cancel_transfer(transfer_id: String) -> Result<(), ApiError>`
- `list_transfers() -> Result<Vec<TransferDto>, ApiError>`
- `sync_with_peer(peer_id: String) -> Result<SyncSummaryDto, ApiError>`
- `set_google_session(email: Option<String>, access_token: Option<String>) -> Result<(), ApiError>` (logout wipes token + offline cloud cache)

### Unified Music Library, Pinned Folders, Settings, YouTube & Migration
- `list_unified_music(group_by: MusicGroupByDto) -> Result<Vec<UnifiedTrackDto>, ApiError>`
- `list_pinned_folders() -> Result<Vec<PinnedFolderDto>, ApiError>`
- `upsert_pinned_folder(folder: PinnedFolderDto) -> Result<(), ApiError>`
- `remove_pinned_folder(id: String) -> Result<(), ApiError>`
- `reorder_pinned_folders(ordered_ids: Vec<String>) -> Result<(), ApiError>`
- `enqueue_youtube_download(req: YoutubeDownloadReqDto) -> Result<DownloadJobDto, ApiError>`
- `update_yt_dlp() -> Result<String, ApiError>`
- `list_download_jobs() -> Result<Vec<DownloadJobDto>, ApiError>`
- `get_settings() -> Result<AppSettingsDto, ApiError>`
- `save_settings(settings: AppSettingsDto) -> Result<AppSettingsDto, ApiError>`
- `detect_legacy_data(legacy_db_path: Option<String>) -> Result<bool, ApiError>`
- `import_legacy_database(legacy_db_path: String, legacy_thumb_dir: Option<String>) -> Result<LegacyImportReportDto, ApiError>`
