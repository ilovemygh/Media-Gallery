import '../models/media_models.dart';

/// Coarse-grained async FFI bridge contract matching `crates/core-api` and `docs/API.md`.
/// Heavy scanning, hashing, thumbnailing, SQLite queries, and iroh networking stay in Rust.
abstract class CoreBridge {
  bool get isLiveCore;

  Future<int> scanFolders(List<String> roots, {List<String> excluded = const []});
  Future<int> ingestAndroidMediaStore(String deviceId, String deviceName, String rowsJson);

  Future<MediaPage> listMedia({
    required MediaFilter filter,
    SortField sortBy = SortField.date,
    SortDir sortDir = SortDir.desc,
    String? cursor,
    int limit = 200,
  });

  Future<List<FolderSummary>> listFolders({String? deviceId, MediaType? mediaType});
  Future<List<DeviceItem>> listDevices();
  Future<List<PinnedFolderItem>> listPinnedFolders();
  Future<AppSettings> getSettings();
  Future<void> saveSettings(AppSettings settings);
  Future<StorageStats> getStorageStats();
  Future<Map<String, String>> ensureThumbnailsBatch(List<String> mediaIds);

  Future<MediaItem> setAlias(String mediaId, String alias);
  Future<MediaItem> applyRealRename(String mediaId, String newName);
  Future<bool> undoLastRename(String? mediaId);
  Future<List<BulkRenamePreview>> previewBulkRename(List<String> mediaIds, String template);
  Future<BulkRenameResult> applyBulkRename(List<String> mediaIds, String template);
  Future<int> undoBulkRename(String batchId);
  Future<MediaItem> moveMediaToFolder(String mediaId, String targetFolder);
  Future<void> moveToTrash(String mediaId);
  Future<MediaItem> restoreFromTrash(String mediaId);
  Future<int> emptyTrash({String? mediaId});

  Future<CoverItem> updateCoverAspect(String mediaId, AspectPreset preset);
  Future<CoverItem> rerollVideoThumbnail(String mediaId);
  Future<CoverItem> pickVideoFrame(String mediaId, int frameTimeMs);
  Future<CoverItem> setCustomCover(
    String mediaId,
    String imagePath,
    AspectPreset preset, {
    bool confirmedEmbeddedWrite = true,
  });

  Future<void> upsertPinnedFolder(PinnedFolderItem folder);
  Future<void> removePinnedFolder(String id);
  Future<void> reorderPinnedFolders(List<String> orderedIds);

  Future<void> downloadRemoteMedia(String mediaId);
  Future<void> sendMediaToDevice(String mediaId, String targetDeviceId);
  Future<String> createPairingPin();
  Future<PairingSessionInfo> createPairingSession();
  Future<void> forgetDevice(String deviceId);
  Future<void> enqueueYoutubeDownload({
    required String url,
    required String format,
    required String quality,
    required String destinationFolder,
  });
  Future<List<DownloadJobItem>> listDownloadJobs();

  Future<bool> detectLegacyDatabase();
  Future<LegacyImportReport> importLegacyDatabase();
}
