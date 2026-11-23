import 'dart:async';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../bridge/core_bridge.dart';
import '../bridge/live_core_bridge.dart';
import '../bridge/mock_core_bridge.dart';
import '../models/media_models.dart';
import '../services/media_playback_service.dart';

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
    this.musicGroupBy = MusicGroupBy.all,
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
    this.duplicateGroups = const [],
    this.musicGroups = const [],
    this.transfers = const [],
    this.autoSyncRules = const [],
    this.savedSearches = const [],
    this.googleEmail,
    this.playingTrack,
    this.isPlaying = false,
    this.shuffle = false,
    this.repeatOne = false,
    this.isScanning = false,
    this.isBackgroundScanning = false,
    this.statusBanner,
    this.lastBulkBatchId,
    this.lastCompletedDownload,
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
  final MusicGroupBy musicGroupBy;
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
  final List<DuplicateGroupItem> duplicateGroups;
  final List<MusicGroupItem> musicGroups;
  final List<TransferItem> transfers;
  final List<AutoSyncRuleItem> autoSyncRules;
  final List<SavedSearchItem> savedSearches;
  final String? googleEmail;
  final MediaItem? playingTrack;
  final bool isPlaying;
  final bool shuffle;
  final bool repeatOne;
  final bool isScanning;
  final bool isBackgroundScanning;
  final String? statusBanner;
  final String? lastBulkBatchId;
  final DownloadJobItem? lastCompletedDownload;

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
    MusicGroupBy? musicGroupBy,
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
    List<DuplicateGroupItem>? duplicateGroups,
    List<MusicGroupItem>? musicGroups,
    List<TransferItem>? transfers,
    List<AutoSyncRuleItem> ? autoSyncRules,
    List<SavedSearchItem>? savedSearches,
    String? googleEmail,
    bool clearGoogle = false,
    MediaItem? playingTrack,
    bool clearPlaying = false,
    bool? isPlaying,
    bool? shuffle,
    bool? repeatOne,
    bool? isScanning,
    bool? isBackgroundScanning,
    String? statusBanner,
    bool clearBanner = false,
    String? lastBulkBatchId,
    bool clearBulkBatch = false,
    DownloadJobItem? lastCompletedDownload,
    bool clearCompletedDownload = false,
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
        musicGroupBy: musicGroupBy ?? this.musicGroupBy,
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
        duplicateGroups: duplicateGroups ?? this.duplicateGroups,
        musicGroups: musicGroups ?? this.musicGroups,
        transfers: transfers ?? this.transfers,
        autoSyncRules: autoSyncRules ?? this.autoSyncRules,
        savedSearches: savedSearches ?? this.savedSearches,
        googleEmail: clearGoogle ? null : (googleEmail ?? this.googleEmail),
        playingTrack: clearPlaying ? null : (playingTrack ?? this.playingTrack),
        isPlaying: isPlaying ?? this.isPlaying,
        shuffle: shuffle ?? this.shuffle,
        repeatOne: repeatOne ?? this.repeatOne,
        isScanning: isScanning ?? this.isScanning,
        isBackgroundScanning: isBackgroundScanning ?? this.isBackgroundScanning,
        statusBanner: clearBanner ? null : (statusBanner ?? this.statusBanner),
        lastBulkBatchId: clearBulkBatch ? null : (lastBulkBatchId ?? this.lastBulkBatchId),
        lastCompletedDownload: clearCompletedDownload
            ? null
            : (lastCompletedDownload ?? this.lastCompletedDownload),
      );
}

String _normPathKey(String raw) =>
    raw.replaceAll('\\', '/').replaceAll(RegExp(r'/+$'), '').toLowerCase();

class GalleryController extends StateNotifier<GalleryUiState> {
  GalleryController(this._bridge) : super(const GalleryUiState()) {
    MediaPlaybackService.instance.onTrackCompleted = _handleTrackCompleted;
    _bootstrap();
  }

  final CoreBridge _bridge;
  final Set<String> _attemptedThumbIds = {};

  void _handleTrackCompleted() {
    if (!mounted || state.playingTrack == null) return;
    if (state.repeatOne) {
      playTrack(state.playingTrack!);
    } else {
      skipTrack(1);
    }
  }

  Future<void> _bootstrap() async {
    // 1. Load persisted SQLite library immediately on open without rescanning everything.
    await refresh();
    // 2. Auto-scan ONLY once on brand-new empty DB, or when user explicitly enables autoScanOnOpen.
    if (_bridge is LiveCoreBridge) {
      if (!state.settings.initialScanDone && state.stats.totalFiles == 0) {
        unawaited(runSilentBackgroundScan());
      } else if (!state.settings.initialScanDone && state.stats.totalFiles > 0) {
        final next = state.settings.copyWith(initialScanDone: true);
        await _bridge.saveSettings(next);
        if (mounted) state = state.copyWith(settings: next);
      } else if (state.settings.autoScanOnOpen) {
        unawaited(runSilentBackgroundScan());
      }
    }
  }

  /// Runs an incremental background scan on open without banners or user interruption.
  Future<void> runSilentBackgroundScan() async {
    final bridge = _bridge;
    if (bridge is! LiveCoreBridge || state.isScanning || state.isBackgroundScanning) return;
    state = state.copyWith(isBackgroundScanning: true);
    try {
      await bridge.runDefaultInitialScan(silent: true);
      if (mounted) {
        await refresh();
      }
    } catch (_) {
      // Background auto-scan must never disturb the user.
    } finally {
      if (mounted) {
        state = state.copyWith(isBackgroundScanning: false);
      }
    }
  }

  Future<void> runInitialScan() async {
    final bridge = _bridge;
    if (bridge is! LiveCoreBridge) return;
    _attemptedThumbIds.clear();
    state = state.copyWith(isScanning: true, statusBanner: 'Scanning local media...');
    try {
      final count = await bridge.runDefaultInitialScan(silent: false);
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
    _attemptedThumbIds.clear();
    state = state.copyWith(isScanning: true, statusBanner: 'Scanning selected folders...');
    try {
      final seen = <String>{};
      final merged = <String>[];
      for (final p in [...state.settings.scanFolders, ...roots]) {
        if (seen.add(_normPathKey(p))) {
          merged.add(p);
        }
      }
      final nextSettings = state.settings.copyWith(
        scanFolders: merged,
        initialScanDone: true,
      );
      await _bridge.saveSettings(nextSettings);
      final count = await _bridge.scanFolders(
        roots,
        excluded: nextSettings.excludedFolders,
      );
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

  List<MediaItem> _deduplicateMediaList(
    List<MediaItem> input,
    Set<String> hiddenDuplicateIds,
    bool hideDuplicates,
  ) {
    final seenPaths = <String>{};
    final seenContentKeys = <String>{};
    final out = <MediaItem>[];
    for (final item in input) {
      final pathKey = '${item.deviceId}::${_normPathKey(item.path)}';
      if (!seenPaths.add(pathKey)) continue;
      if (hideDuplicates) {
        if (hiddenDuplicateIds.contains(item.id)) continue;
        if (item.isDuplicate) {
          final contentKey = '${item.type.name}::${item.size}::${item.fileName.toLowerCase()}';
          if (!seenContentKeys.add(contentKey)) continue;
        }
      }
      out.add(item);
    }
    return out;
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
      sortBy: state.sortField,
      sortDir: state.sortDir,
    );
    final recentPage = await _bridge.listMedia(
      filter: const MediaFilter(),
      sortBy: SortField.date,
      sortDir: SortDir.desc,
      limit: 16,
    );
    final rawFolders = await _bridge.listFolders(mediaType: mt);
    final devices = await _bridge.listDevices();
    final rawPinned = await _bridge.listPinnedFolders();
    final settings = await _bridge.getSettings();
    final stats = await _bridge.getStorageStats();
    final jobs = await _bridge.listDownloadJobs();
    final dupGroups = await _bridge.listDuplicateGroups();
    final musicGroups = state.section == NavSection.audio
        ? await _bridge.listUnifiedMusic(state.musicGroupBy)
        : state.musicGroups;
    final transfers = await _bridge.listTransfers();
    final syncRules = await _bridge.listAutoSyncRules();
    final savedSearches = await _bridge.listSavedSearches();

    // Build set of redundant duplicate IDs (keep 1 primary representative per duplicate group).
    final hiddenDupIds = <String>{};
    for (final grp in dupGroups) {
      for (final extra in grp.items.skip(1)) {
        hiddenDupIds.add(extra.id);
      }
    }
    final hideDups =
        settings.hideDuplicatesInGallery && !state.onlyDuplicates && !state.trashedOnly;
    final dedupedItems = _deduplicateMediaList(page.items, hiddenDupIds, hideDups);
    final dedupedRecent =
        _deduplicateMediaList(recentPage.items, hiddenDupIds, settings.hideDuplicatesInGallery)
            .take(12)
            .toList();

    // Deduplicate folders by normalized path so the same folder never appears twice.
    final seenFolders = <String>{};
    final dedupedFolders = <FolderSummary>[];
    for (final f in rawFolders) {
      if (seenFolders.add('${f.deviceId}::${_normPathKey(f.path)}')) {
        dedupedFolders.add(f);
      }
    }

    // Deduplicate pinned folder shortcuts by normalized path.
    final seenPins = <String>{};
    final dedupedPinned = <PinnedFolderItem>[];
    for (final p in rawPinned) {
      if (seenPins.add('${p.deviceId}::${_normPathKey(p.path)}')) {
        dedupedPinned.add(p);
      }
    }

    state = state.copyWith(
      items: dedupedItems,
      recentItems: dedupedRecent,
      folders: dedupedFolders,
      devices: devices,
      pinnedFolders: dedupedPinned,
      settings: settings,
      stats: stats,
      downloadJobs: jobs,
      duplicateGroups: dupGroups,
      musicGroups: musicGroups,
      transfers: transfers,
      autoSyncRules: syncRules,
      savedSearches: savedSearches,
    );

    // Lazily populate missing thumbnails once per item in background batches so grids load fast.
    if (_bridge.isLiveCore) {
      final missingIds = <String>{
        for (final m in page.items.take(40))
          if ((m.cover == null || m.cover!.filePath.isEmpty) &&
              !_attemptedThumbIds.contains(m.id))
            m.id,
        for (final m in recentPage.items)
          if ((m.cover == null || m.cover!.filePath.isEmpty) &&
              !_attemptedThumbIds.contains(m.id))
            m.id,
      }.toList();
      if (missingIds.isNotEmpty) {
        _attemptedThumbIds.addAll(missingIds);
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
      sortField: section == NavSection.recent ? SortField.date : state.sortField,
      sortDir: section == NavSection.recent ? SortDir.desc : state.sortDir,
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

  Future<void> cleanAllDuplicates({bool keepNewest = true}) async {
    final report = await _bridge.cleanDuplicates(keepNewest: keepNewest);
    await refresh();
    state = state.copyWith(
      statusBanner: report.cleanedCount > 0
          ? 'Moved ${report.cleanedCount} duplicate(s) to Recycle Bin'
          : 'No redundant duplicates to clean',
    );
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

  Future<void> saveCurrentSearch(String name) async {
    await _bridge.saveSearch(
      name,
      state.searchQuery,
      mediaType: _sectionMediaType(state.section),
    );
    await refresh();
    state = state.copyWith(statusBanner: 'Saved smart search "$name"');
  }

  Future<void> applySavedSearch(SavedSearchItem saved) async {
    final targetSection = saved.mediaType == MediaType.video
        ? NavSection.videos
        : saved.mediaType == MediaType.image
            ? NavSection.images
            : saved.mediaType == MediaType.audio
                ? NavSection.audio
                : NavSection.home;
    state = state.copyWith(
      section: targetSection,
      searchQuery: saved.query,
      clearFolder: true,
    );
    await refresh();
  }

  Future<void> setSort(SortField field, SortDir dir) async {
    state = state.copyWith(sortField: field, sortDir: dir);
    await refresh();
  }

  Future<void> setMusicGroupBy(MusicGroupBy groupBy) async {
    state = state.copyWith(musicGroupBy: groupBy);
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

  Future<void> updateSettings(AppSettings next) async {
    await _bridge.saveSettings(next);
    state = state.copyWith(settings: next, statusBanner: 'Settings saved');
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
    try {
      await _bridge.moveMediaToFolder(mediaId, targetFolder);
      await refresh();
      state = state.copyWith(statusBanner: 'Moved to $targetFolder');
    } catch (e) {
      state = state.copyWith(statusBanner: 'Move failed: $e');
    }
  }

  Future<void> saveAlias(String mediaId, String alias) async {
    try {
      await _bridge.setAlias(mediaId, alias);
      await refresh();
      state = state.copyWith(statusBanner: 'Alias saved');
    } catch (e) {
      state = state.copyWith(statusBanner: 'Save alias failed: $e');
    }
  }

  Future<void> applyRealRename(String mediaId, String newName) async {
    try {
      await _bridge.applyRealRename(mediaId, newName);
      await refresh();
      state = state.copyWith(statusBanner: 'File renamed on disk');
    } catch (e) {
      state = state.copyWith(statusBanner: 'Rename failed: $e');
    }
  }

  Future<void> undoLastRename(String? mediaId) async {
    try {
      final ok = await _bridge.undoLastRename(mediaId);
      await refresh();
      state = state.copyWith(
        statusBanner: ok ? 'Rename undone' : 'Nothing to undo',
      );
    } catch (e) {
      state = state.copyWith(statusBanner: 'Undo rename failed: $e');
    }
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
    try {
      final count = await _bridge.undoBulkRename(batchId);
      await refresh();
      state = state.copyWith(
        clearBulkBatch: true,
        statusBanner: 'Undid bulk rename ($count files)',
      );
    } catch (e) {
      state = state.copyWith(statusBanner: 'Undo failed: $e');
    }
  }

  Future<void> updateCoverAspect(String mediaId, AspectPreset preset) async {
    try {
      await _bridge.updateCoverAspect(mediaId, preset);
      await refresh();
    } catch (e) {
      state = state.copyWith(statusBanner: 'Cover aspect update failed: $e');
    }
  }

  Future<void> rerollVideoThumbnail(String mediaId) async {
    try {
      await _bridge.rerollVideoThumbnail(mediaId);
      await refresh();
    } catch (e) {
      state = state.copyWith(statusBanner: 'Thumbnail reroll failed: $e');
    }
  }

  Future<void> pickVideoFrame(String mediaId, int frameTimeMs) async {
    try {
      await _bridge.pickVideoFrame(mediaId, frameTimeMs);
      await refresh();
    } catch (e) {
      state = state.copyWith(statusBanner: 'Frame extract failed: $e');
    }
  }

  Future<void> setCustomCover(
    String mediaId,
    String imagePath,
    AspectPreset preset,
  ) async {
    try {
      await _bridge.setCustomCover(mediaId, imagePath, preset);
      await refresh();
      state = state.copyWith(statusBanner: 'Custom cover saved');
    } catch (e) {
      state = state.copyWith(statusBanner: 'Custom cover failed: $e');
    }
  }

  Future<void> downloadRemote(String mediaId) async {
    try {
      await _bridge.downloadRemoteMedia(mediaId);
      await refresh();
      state = state.copyWith(statusBanner: 'Download queued');
    } catch (e) {
      state = state.copyWith(statusBanner: 'Download failed: $e');
    }
  }

  Future<void> sendToDevice(String mediaId, String targetDeviceId) async {
    try {
      await _bridge.sendMediaToDevice(mediaId, targetDeviceId);
      await refresh();
      state = state.copyWith(statusBanner: 'Transfer queued');
    } catch (e) {
      state = state.copyWith(statusBanner: 'Send failed: $e');
    }
  }

  Future<void> backupToDevice(String targetDeviceId) async {
    try {
      final count = await _bridge.backupLocalMediaToDevice(targetDeviceId);
      await refresh();
      state = state.copyWith(statusBanner: 'Queued $count file(s) for backup');
    } catch (e) {
      state = state.copyWith(statusBanner: 'Backup failed: $e');
    }
  }

  Future<void> pairWithDevice(String payloadOrPin, {String? peerName}) async {
    try {
      final dev = await _bridge.pairWithDevice(payloadOrPin, peerName: peerName);
      await refresh();
      state = state.copyWith(statusBanner: 'Paired with ${dev.name}');
    } catch (e) {
      state = state.copyWith(statusBanner: 'Pairing failed: $e');
    }
  }

  Future<void> pauseTransfer(String transferId) async {
    await _bridge.pauseTransfer(transferId);
    await refresh();
  }

  Future<void> cancelTransfer(String transferId) async {
    await _bridge.cancelTransfer(transferId);
    await refresh();
  }

  Future<void> toggleAutoSyncRule(String targetDeviceId, bool enabled) async {
    await _bridge.upsertAutoSyncRule(
      id: 'rule-backup-$targetDeviceId',
      name: 'Auto-backup Photos & Videos',
      targetDeviceId: targetDeviceId,
      folderPattern: 'Camera|DCIM|Pictures|Videos',
      enabled: enabled,
    );
    await refresh();
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
    final normTarget = _normPathKey(folderPath);
    if (state.pinnedFolders.any((p) => _normPathKey(p.path) == normTarget)) {
      return;
    }
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

  Future<DownloadJobItem?> startYoutubeDownload({
    required String url,
    required String format,
    required String quality,
    String? destinationFolder,
  }) async {
    final dest = destinationFolder ??
        (format == 'mp3' ? 'Music/YouTube' : 'Videos/YouTube');
    await _bridge.enqueueYoutubeDownload(
      url: url,
      format: format,
      quality: quality,
      destinationFolder: dest,
    );
    await refresh();
    final completed = state.downloadJobs
        .where((j) => j.status == 'completed')
        .firstOrNull;
    if (completed != null) {
      state = state.copyWith(lastCompletedDownload: completed);
    }
    return completed;
  }

  void clearCompletedDownload() {
    state = state.copyWith(clearCompletedDownload: true);
  }

  Future<MediaItem?> resolveMediaForDownloadJob(DownloadJobItem job) async {
    MediaItem? matchIn(List<MediaItem> list) {
      for (final m in list) {
        if (m.id == job.id) return m;
        if (job.title != null &&
            job.title!.isNotEmpty &&
            (m.alias.toLowerCase() == job.title!.toLowerCase() ||
                m.fileName.toLowerCase().contains(job.title!.toLowerCase()))) {
          return m;
        }
      }
      final jobFolder = _normPathKey(job.destinationFolder);
      for (final m in list) {
        if (_normPathKey(m.folderPath) == jobFolder) {
          return m;
        }
      }
      return null;
    }

    var found = matchIn(state.items) ?? matchIn(state.recentItems);
    if (found != null) return found;

    final folderPage = await _bridge.listMedia(
      filter: MediaFilter(folderPath: job.destinationFolder),
      sortBy: SortField.date,
      sortDir: SortDir.desc,
      limit: 20,
    );
    found = matchIn(folderPage.items) ?? folderPage.items.firstOrNull;
    if (found != null) return found;

    final allPage = await _bridge.listMedia(
      filter: const MediaFilter(),
      sortBy: SortField.date,
      sortDir: SortDir.desc,
      limit: 30,
    );
    return matchIn(allPage.items) ?? allPage.items.firstOrNull;
  }

  Future<void> renamePinnedFolder(String id, String newLabel) async {
    final existing = state.pinnedFolders.where((p) => p.id == id).firstOrNull;
    if (existing == null || newLabel.trim().isEmpty) return;
    await _bridge.upsertPinnedFolder(existing.copyWith(label: newLabel.trim()));
    await refresh();
  }

  Future<void> movePinnedFolder(String id, int delta) async {
    final list = [...state.pinnedFolders];
    final idx = list.indexWhere((p) => p.id == id);
    final nextIdx = idx + delta;
    if (idx < 0 || nextIdx < 0 || nextIdx >= list.length) return;
    final item = list.removeAt(idx);
    list.insert(nextIdx, item);
    await _bridge.reorderPinnedFolders(list.map((e) => e.id).toList());
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

  Future<void> playTrack(MediaItem track) async {
    state = state.copyWith(playingTrack: track, isPlaying: true);
    await MediaPlaybackService.instance.openAndPlay(track);
  }

  Future<void> togglePlayback() async {
    if (state.playingTrack == null) return;
    await MediaPlaybackService.instance.togglePlayPause();
    state = state.copyWith(isPlaying: !state.isPlaying);
  }

  Future<void> stopPlayback() async {
    await MediaPlaybackService.instance.stop();
    state = state.copyWith(clearPlaying: true, isPlaying: false);
  }

  void toggleShuffle() {
    state = state.copyWith(shuffle: !state.shuffle);
  }

  void toggleRepeat() {
    state = state.copyWith(repeatOne: !state.repeatOne);
  }

  Future<void> skipTrack(int delta) async {
    final audioItems = state.items.where((m) => m.type == MediaType.audio).toList();
    if (audioItems.isEmpty || state.playingTrack == null) return;
    final idx = audioItems.indexWhere((m) => m.id == state.playingTrack!.id);
    if (state.shuffle && audioItems.length > 1) {
      final step = (DateTime.now().microsecondsSinceEpoch % (audioItems.length - 1)) + 1;
      final base = idx < 0 ? 0 : idx;
      await playTrack(audioItems[(base + step) % audioItems.length]);
      return;
    }
    final nextIdx = idx < 0 ? 0 : (idx + delta) % audioItems.length;
    final target = audioItems[nextIdx < 0 ? audioItems.length - 1 : nextIdx];
    await playTrack(target);
  }

  void setStatusBanner(String msg) {
    state = state.copyWith(statusBanner: msg);
  }

  void clearStatusBanner() {
    state = state.copyWith(clearBanner: true);
  }
}

final galleryControllerProvider =
    StateNotifierProvider<GalleryController, GalleryUiState>((ref) {
  return GalleryController(ref.watch(bridgeProvider));
});
