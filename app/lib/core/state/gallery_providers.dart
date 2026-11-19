import 'dart:async';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../bridge/core_bridge.dart';
import '../bridge/live_core_bridge.dart';
import '../bridge/mock_core_bridge.dart';
import '../models/media_models.dart';

enum NavSection {
  home,
  recent,
  videos,
  images,
  audio,
  clean,
  youtube,
  devices,
  settings,
}

final bridgeProvider = Provider<CoreBridge>((ref) {
  return MockCoreBridge(simulateLegacyPresent: false);
});

class GalleryUiState {
  const GalleryUiState({
    this.section = NavSection.home,
    this.activeFolder,
    this.searchQuery = '',
    this.gridDensity = GridDensity.medium,
    this.sortField = SortField.date,
    this.sortDir = SortDir.desc,
    this.onlyDuplicates = false,
    this.minSizeBytes,
    this.trashedOnly = false,
    this.syncFilter,
    this.selectedMediaId,
    this.selectedIds = const {},
    this.selectionMode = false,
    this.detailsHiddenForSession = false,
    this.settings = const AppSettings(),
    this.stats = const StorageStats(),
    this.items = const [],
    this.recentItems = const [],
    this.folders = const [],
    this.devices = const [],
    this.pinnedFolders = const [],
    this.downloadJobs = const [],
    this.googleEmail,
    this.playingTrack,
    this.isPlaying = false,
    this.shuffle = false,
    this.repeatOne = false,
    this.isScanning = false,
    this.statusBanner,
    this.lastBulkBatchId,
  });

  final NavSection section;
  final String? activeFolder;
  final String searchQuery;
  final GridDensity gridDensity;
  final SortField sortField;
  final SortDir sortDir;
  final bool onlyDuplicates;
  final int? minSizeBytes;
  final bool trashedOnly;
  final SyncState? syncFilter;
  final String? selectedMediaId;
  final Set<String> selectedIds;
  final bool selectionMode;
  final bool detailsHiddenForSession;
  final AppSettings settings;
  final StorageStats stats;
  final List<MediaItem> items;
  final List<MediaItem> recentItems;
  final List<FolderSummary> folders;
  final List<DeviceItem> devices;
  final List<PinnedFolderItem> pinnedFolders;
  final List<DownloadJobItem> downloadJobs;
  final String? googleEmail;
  final MediaItem? playingTrack;
  final bool isPlaying;
  final bool shuffle;
  final bool repeatOne;
  final bool isScanning;
  final String? statusBanner;
  final String? lastBulkBatchId;

  MediaItem? get selectedMedia {
    if (detailsHiddenForSession || selectedMediaId == null) return null;
    for (final item in items) {
      if (item.id == selectedMediaId) return item;
    }
    for (final item in recentItems) {
      if (item.id == selectedMediaId) return item;
    }
    return null;
  }

  GalleryUiState copyWith({
    NavSection? section,
    String? activeFolder,
    bool clearFolder = false,
    String? searchQuery,
    GridDensity? gridDensity,
    SortField? sortField,
    SortDir? sortDir,
    bool? onlyDuplicates,
    int? minSizeBytes,
    bool clearMinSize = false,
    bool? trashedOnly,
    SyncState? syncFilter,
    bool clearSyncFilter = false,
    String? selectedMediaId,
    bool clearSelection = false,
    Set<String>? selectedIds,
    bool? selectionMode,
    bool? detailsHiddenForSession,
    AppSettings? settings,
    StorageStats? stats,
    List<MediaItem>? items,
    List<MediaItem>? recentItems,
    List<FolderSummary>? folders,
    List<DeviceItem>? devices,
    List<PinnedFolderItem>? pinnedFolders,
    List<DownloadJobItem>? downloadJobs,
    String? googleEmail,
    bool clearGoogle = false,
    MediaItem? playingTrack,
    bool clearPlaying = false,
    bool? isPlaying,
    bool? shuffle,
    bool? repeatOne,
    bool? isScanning,
    String? statusBanner,
    bool clearBanner = false,
    String? lastBulkBatchId,
    bool clearBulkBatch = false,
  }) =>
      GalleryUiState(
        section: section ?? this.section,
        activeFolder: clearFolder ? null : (activeFolder ?? this.activeFolder),
        searchQuery: searchQuery ?? this.searchQuery,
        gridDensity: gridDensity ?? this.gridDensity,
        sortField: sortField ?? this.sortField,
        sortDir: sortDir ?? this.sortDir,
        onlyDuplicates: onlyDuplicates ?? this.onlyDuplicates,
        minSizeBytes: clearMinSize ? null : (minSizeBytes ?? this.minSizeBytes),
        trashedOnly: trashedOnly ?? this.trashedOnly,
        syncFilter: clearSyncFilter ? null : (syncFilter ?? this.syncFilter),
        selectedMediaId:
            clearSelection ? null : (selectedMediaId ?? this.selectedMediaId),
        selectedIds: selectedIds ?? this.selectedIds,
        selectionMode: selectionMode ?? this.selectionMode,
        detailsHiddenForSession:
            detailsHiddenForSession ?? this.detailsHiddenForSession,
        settings: settings ?? this.settings,
        stats: stats ?? this.stats,
        items: items ?? this.items,
        recentItems: recentItems ?? this.recentItems,
        folders: folders ?? this.folders,
        devices: devices ?? this.devices,
        pinnedFolders: pinnedFolders ?? this.pinnedFolders,
        downloadJobs: downloadJobs ?? this.downloadJobs,
        googleEmail: clearGoogle ? null : (googleEmail ?? this.googleEmail),
        playingTrack: clearPlaying ? null : (playingTrack ?? this.playingTrack),
        isPlaying: isPlaying ?? this.isPlaying,
        shuffle: shuffle ?? this.shuffle,
        repeatOne: repeatOne ?? this.repeatOne,
        isScanning: isScanning ?? this.isScanning,
        statusBanner: clearBanner ? null : (statusBanner ?? this.statusBanner),
        lastBulkBatchId: clearBulkBatch ? null : (lastBulkBatchId ?? this.lastBulkBatchId),
      );
}

class GalleryController extends StateNotifier<GalleryUiState> {
  GalleryController(this._bridge) : super(const GalleryUiState()) {
    _bootstrap();
  }

  final CoreBridge _bridge;

  Future<void> _bootstrap() async {
    await refresh();
    if (_bridge is LiveCoreBridge && state.stats.totalFiles == 0) {
      await runInitialScan();
    }
  }

  Future<void> runInitialScan() async {
    if (_bridge is! LiveCoreBridge) return;
    state = state.copyWith(isScanning: true, statusBanner: 'Scanning local media...');
    try {
      final count = await _bridge.runDefaultInitialScan();
      await refresh();
      state = state.copyWith(
        isScanning: false,
        statusBanner: count > 0 ? 'Indexed $count media files' : 'Library up to date',
      );
    } catch (e) {
      state = state.copyWith(isScanning: false, statusBanner: 'Scan error: $e');
    }
  }

  Future<void> scanFolderPaths(List<String> roots) async {
    if (roots.isEmpty) return;
    state = state.copyWith(isScanning: true, statusBanner: 'Scanning selected folders...');
    try {
      final merged = {...state.settings.scanFolders, ...roots}.toList();
      final nextSettings = state.settings.copyWith(scanFolders: merged);
      await _bridge.saveSettings(nextSettings);
      final count = await _bridge.scanFolders(roots);
      await refresh();
      state = state.copyWith(
        isScanning: false,
        statusBanner: 'Indexed $count files from ${roots.length} folder(s)',
      );
    } catch (e) {
      state = state.copyWith(isScanning: false, statusBanner: 'Scan failed: $e');
    }
  }

  MediaType? _sectionMediaType(NavSection section) {
    switch (section) {
      case NavSection.videos:
        return MediaType.video;
      case NavSection.images:
        return MediaType.image;
      case NavSection.audio:
        return MediaType.audio;
      default:
        return null;
    }
  }

  Future<void> refresh() async {
    final mt = _sectionMediaType(state.section);
    final isClean = state.section == NavSection.clean;
    final page = await _bridge.listMedia(
      filter: MediaFilter(
        query: state.searchQuery,
        type: mt,
        folderPath: state.activeFolder,
        syncState: state.syncFilter,
        minSize: state.minSizeBytes,
        onlyDuplicates: state.onlyDuplicates,
        trashedOnly: isClean && state.trashedOnly,
      ),
      sortBy: state.section == NavSection.recent ? SortField.date : state.sortField,
      sortDir: state.section == NavSection.recent ? SortDir.desc : state.sortDir,
    );
    final recentPage = await _bridge.listMedia(
      filter: const MediaFilter(),
      sortBy: SortField.date,
      sortDir: SortDir.desc,
      limit: 12,
    );
    final folders = await _bridge.listFolders(mediaType: mt);
    final devices = await _bridge.listDevices();
    final pinned = await _bridge.listPinnedFolders();
    final settings = await _bridge.getSettings();
    final stats = await _bridge.getStorageStats();
    final jobs = await _bridge.listDownloadJobs();

    state = state.copyWith(
      items: page.items,
      recentItems: recentPage.items,
      folders: folders,
      devices: devices,
      pinnedFolders: pinned,
      settings: settings,
      stats: stats,
      downloadJobs: jobs,
    );

    // Lazily populate missing thumbnails in background batches so grids load fast.
    if (_bridge.isLiveCore) {
      final missingIds = <String>{
        for (final m in page.items.take(40))
          if (m.cover == null || m.cover!.filePath.isEmpty) m.id,
        for (final m in recentPage.items)
          if (m.cover == null || m.cover!.filePath.isEmpty) m.id,
      }.toList();
      if (missingIds.isNotEmpty) {
        unawaited(_populateThumbnails(missingIds));
      }
    }
  }

  Future<void> _populateThumbnails(List<String> missingIds) async {
    try {
      final generated = await _bridge.ensureThumbnailsBatch(missingIds);
      if (generated.isEmpty || !mounted) return;
      List<MediaItem> patch(List<MediaItem> list) => list.map((m) {
            final path = generated[m.id];
            if (path == null) return m;
            final cov = (m.cover ??
                    CoverItem(
                      id: 'cov-${m.id}',
                      mediaId: m.id,
                      filePath: path,
                      aspect: AspectPreset.ratio16x9,
                      frameTimeMs: 0,
                    ))
                .copyWith(filePath: path);
            return m.copyWith(cover: cov);
          }).toList();

      state = state.copyWith(
        items: patch(state.items),
        recentItems: patch(state.recentItems),
      );
    } catch (_) {
      // Non-fatal: thumbnails can retry on next refresh.
    }
  }

  Future<void> selectSection(NavSection section) async {
    state = state.copyWith(
      section: section,
      clearFolder: true,
      onlyDuplicates: section == NavSection.clean ? state.onlyDuplicates : false,
      trashedOnly: section == NavSection.clean ? state.trashedOnly : false,
      clearMinSize: section != NavSection.clean,
    );
    await refresh();
  }

  Future<void> openCleanerMode({
    bool duplicates = false,
    bool largeFiles = false,
    bool trashed = false,
  }) async {
    state = state.copyWith(
      section: NavSection.clean,
      clearFolder: true,
      onlyDuplicates: duplicates,
      minSizeBytes: largeFiles ? 52428800 : null,
      clearMinSize: !largeFiles,
      trashedOnly: trashed,
      sortField: largeFiles ? SortField.size : SortField.date,
      sortDir: SortDir.desc,
    );
    await refresh();
  }

  Future<void> openFolder(String? folderPath) async {
    state = state.copyWith(
      section: NavSection.home,
      activeFolder: folderPath,
      clearFolder: folderPath == null,
    );
    await refresh();
  }

  Future<void> setSearchQuery(String q) async {
    state = state.copyWith(searchQuery: q);
    await refresh();
  }

  Future<void> setSort(SortField field, SortDir dir) async {
    state = state.copyWith(sortField: field, sortDir: dir);
    await refresh();
  }

  Future<void> setQuickFilter({
    SyncState? syncState,
    bool clearSync = false,
    bool? onlyDuplicates,
    int? minSizeBytes,
    bool clearMinSize = false,
  }) async {
    state = state.copyWith(
      syncFilter: syncState,
      clearSyncFilter: clearSync,
      onlyDuplicates: onlyDuplicates,
      minSizeBytes: minSizeBytes,
      clearMinSize: clearMinSize,
    );
    await refresh();
  }

  void toggleSelectionMode([bool? force]) {
    final next = force ?? !state.selectionMode;
    state = state.copyWith(
      selectionMode: next,
      selectedIds: next ? state.selectedIds : const {},
    );
  }

  void toggleItemSelected(String mediaId) {
    final next = {...state.selectedIds};
    if (next.contains(mediaId)) {
      next.remove(mediaId);
    } else {
      next.add(mediaId);
    }
    state = state.copyWith(
      selectionMode: next.isNotEmpty || state.selectionMode,
      selectedIds: next,
    );
  }

  void selectAllVisible() {
    state = state.copyWith(
      selectionMode: true,
      selectedIds: state.items.map((e) => e.id).toSet(),
    );
  }

  void clearMultiSelection() {
    state = state.copyWith(selectionMode: false, selectedIds: const {});
  }

  Future<void> toggleSidebarCollapsed() async {
    final next = state.settings.copyWith(
      sidebarCollapsed: !state.settings.sidebarCollapsed,
    );
    await _bridge.saveSettings(next);
    state = state.copyWith(settings: next);
  }

  Future<void> resizeSidebar(double width) async {
    final clamped = width.clamp(180.0, 380.0);
    final next = state.settings.copyWith(sidebarWidth: clamped);
    await _bridge.saveSettings(next);
    state = state.copyWith(settings: next);
  }

  Future<void> toggleTheme() async {
    final next = state.settings.copyWith(darkMode: !state.settings.darkMode);
    await _bridge.saveSettings(next);
    state = state.copyWith(settings: next);
  }

  void selectMedia(String mediaId) {
    state = state.copyWith(
      selectedMediaId: mediaId,
      detailsHiddenForSession: false,
    );
  }

  void closeDetailsForSession() {
    state = state.copyWith(detailsHiddenForSession: true);
  }

  Future<void> setGridDensity(GridDensity density) async {
    state = state.copyWith(gridDensity: density);
  }

  Future<void> moveToFolder(String mediaId, String targetFolder) async {
    await _bridge.moveMediaToFolder(mediaId, targetFolder);
    await refresh();
  }

  Future<void> saveAlias(String mediaId, String alias) async {
    await _bridge.setAlias(mediaId, alias);
    await refresh();
    state = state.copyWith(statusBanner: 'Alias saved');
  }

  Future<void> applyRealRename(String mediaId, String newName) async {
    await _bridge.applyRealRename(mediaId, newName);
    await refresh();
    state = state.copyWith(statusBanner: 'File renamed on disk');
  }

  Future<void> undoLastRename(String? mediaId) async {
    final ok = await _bridge.undoLastRename(mediaId);
    await refresh();
    state = state.copyWith(
      statusBanner: ok ? 'Rename undone' : 'Nothing to undo',
    );
  }

  Future<BulkRenameResult> applyBulkRename(List<String> mediaIds, String template) async {
    final res = await _bridge.applyBulkRename(mediaIds, template);
    await refresh();
    state = state.copyWith(
      lastBulkBatchId: res.batchId,
      selectionMode: false,
      selectedIds: const {},
      statusBanner: 'Renamed ${res.renamedCount} file(s)',
    );
    return res;
  }

  Future<void> undoLastBulkRename() async {
    final batchId = state.lastBulkBatchId;
    if (batchId == null) return;
    final count = await _bridge.undoBulkRename(batchId);
    await refresh();
    state = state.copyWith(
      clearBulkBatch: true,
      statusBanner: 'Undid bulk rename ($count files)',
    );
  }

  Future<void> updateCoverAspect(String mediaId, AspectPreset preset) async {
    await _bridge.updateCoverAspect(mediaId, preset);
    await refresh();
  }

  Future<void> rerollVideoThumbnail(String mediaId) async {
    await _bridge.rerollVideoThumbnail(mediaId);
    await refresh();
  }

  Future<void> pickVideoFrame(String mediaId, int frameTimeMs) async {
    await _bridge.pickVideoFrame(mediaId, frameTimeMs);
    await refresh();
  }

  Future<void> setCustomCover(
    String mediaId,
    String imagePath,
    AspectPreset preset,
  ) async {
    await _bridge.setCustomCover(mediaId, imagePath, preset);
    await refresh();
    state = state.copyWith(statusBanner: 'Custom cover saved');
  }

  Future<void> downloadRemote(String mediaId) async {
    await _bridge.downloadRemoteMedia(mediaId);
    await refresh();
  }

  Future<void> sendToDevice(String mediaId, String targetDeviceId) async {
    await _bridge.sendMediaToDevice(mediaId, targetDeviceId);
    state = state.copyWith(statusBanner: 'Queued transfer to device');
  }

  Future<void> moveToTrash(String mediaId) async {
    await _bridge.moveToTrash(mediaId);
    state = state.copyWith(clearSelection: true, statusBanner: 'Moved to Recycle Bin');
    await refresh();
  }

  Future<void> trashSelected() async {
    final ids = state.selectedIds.toList();
    for (final id in ids) {
      await _bridge.moveToTrash(id);
    }
    state = state.copyWith(
      selectionMode: false,
      selectedIds: const {},
      clearSelection: true,
      statusBanner: 'Moved ${ids.length} item(s) to Recycle Bin',
    );
    await refresh();
  }

  Future<void> restoreFromTrash(String mediaId) async {
    await _bridge.restoreFromTrash(mediaId);
    await refresh();
    state = state.copyWith(statusBanner: 'Restored from Recycle Bin');
  }

  Future<void> emptyTrash({String? mediaId}) async {
    final removed = await _bridge.emptyTrash(mediaId: mediaId);
    await refresh();
    state = state.copyWith(statusBanner: 'Deleted $removed item(s) permanently');
  }

  Future<void> addPinnedFolder(String folderPath) async {
    final label = folderPath
            .replaceAll('\\', '/')
            .split('/')
            .where((e) => e.isNotEmpty)
            .lastOrNull ??
        'Folder';
    await _bridge.upsertPinnedFolder(
      PinnedFolderItem(
        id: 'pin-${DateTime.now().millisecondsSinceEpoch}',
        deviceId: 'pc-local',
        path: folderPath,
        label: label,
        position: state.pinnedFolders.length,
      ),
    );
    await refresh();
  }

  Future<void> removePinnedFolder(String id) async {
    await _bridge.removePinnedFolder(id);
    await refresh();
  }

  void toggleGoogleSignIn() {
    if (state.googleEmail != null) {
      state = state.copyWith(clearGoogle: true);
    } else {
      state = state.copyWith(googleEmail: 'creator@gmail.com');
    }
  }

  void playTrack(MediaItem track) {
    state = state.copyWith(playingTrack: track, isPlaying: true);
  }

  void togglePlayback() {
    if (state.playingTrack != null) {
      state = state.copyWith(isPlaying: !state.isPlaying);
    }
  }

  void stopPlayback() {
    state = state.copyWith(clearPlaying: true, isPlaying: false);
  }

  void toggleShuffle() {
    state = state.copyWith(shuffle: !state.shuffle);
  }

  void toggleRepeat() {
    state = state.copyWith(repeatOne: !state.repeatOne);
  }

  void skipTrack(int delta) {
    final audioItems = state.items.where((m) => m.type == MediaType.audio).toList();
    if (audioItems.isEmpty || state.playingTrack == null) return;
    final idx = audioItems.indexWhere((m) => m.id == state.playingTrack!.id);
    final nextIdx = idx < 0
        ? 0
        : (idx + delta) % audioItems.length;
    state = state.copyWith(playingTrack: audioItems[nextIdx < 0 ? audioItems.length - 1 : nextIdx], isPlaying: true);
  }

  void clearStatusBanner() {
    state = state.copyWith(clearBanner: true);
  }
}

final galleryControllerProvider =
    StateNotifierProvider<GalleryController, GalleryUiState>((ref) {
  return GalleryController(ref.watch(bridgeProvider));
});
