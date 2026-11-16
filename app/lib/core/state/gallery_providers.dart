import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../bridge/core_bridge.dart';
import '../bridge/mock_core_bridge.dart';
import '../models/media_models.dart';

enum NavSection { home, videos, images, audio, youtube, devices, settings }

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
    this.selectedMediaId,
    this.detailsHiddenForSession = false,
    this.settings = const AppSettings(),
    this.items = const [],
    this.folders = const [],
    this.devices = const [],
    this.pinnedFolders = const [],
    this.googleEmail,
    this.playingTrack,
    this.isPlaying = false,
  });

  final NavSection section;
  final String? activeFolder;
  final String searchQuery;
  final GridDensity gridDensity;
  final SortField sortField;
  final SortDir sortDir;
  final String? selectedMediaId;
  final bool detailsHiddenForSession;
  final AppSettings settings;
  final List<MediaItem> items;
  final List<FolderSummary> folders;
  final List<DeviceItem> devices;
  final List<PinnedFolderItem> pinnedFolders;
  final String? googleEmail;
  final MediaItem? playingTrack;
  final bool isPlaying;

  MediaItem? get selectedMedia {
    if (detailsHiddenForSession || selectedMediaId == null) return null;
    for (final item in items) {
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
    String? selectedMediaId,
    bool clearSelection = false,
    bool? detailsHiddenForSession,
    AppSettings? settings,
    List<MediaItem>? items,
    List<FolderSummary>? folders,
    List<DeviceItem>? devices,
    List<PinnedFolderItem>? pinnedFolders,
    String? googleEmail,
    bool clearGoogle = false,
    MediaItem? playingTrack,
    bool? isPlaying,
  }) =>
      GalleryUiState(
        section: section ?? this.section,
        activeFolder: clearFolder ? null : (activeFolder ?? this.activeFolder),
        searchQuery: searchQuery ?? this.searchQuery,
        gridDensity: gridDensity ?? this.gridDensity,
        sortField: sortField ?? this.sortField,
        sortDir: sortDir ?? this.sortDir,
        selectedMediaId:
            clearSelection ? null : (selectedMediaId ?? this.selectedMediaId),
        detailsHiddenForSession:
            detailsHiddenForSession ?? this.detailsHiddenForSession,
        settings: settings ?? this.settings,
        items: items ?? this.items,
        folders: folders ?? this.folders,
        devices: devices ?? this.devices,
        pinnedFolders: pinnedFolders ?? this.pinnedFolders,
        googleEmail: clearGoogle ? null : (googleEmail ?? this.googleEmail),
        playingTrack: playingTrack ?? this.playingTrack,
        isPlaying: isPlaying ?? this.isPlaying,
      );
}

class GalleryController extends StateNotifier<GalleryUiState> {
  GalleryController(this._bridge) : super(const GalleryUiState()) {
    refresh();
  }

  final CoreBridge _bridge;

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
    final page = await _bridge.listMedia(
      filter: MediaFilter(
        query: state.searchQuery,
        type: mt,
        folderPath: state.activeFolder,
      ),
      sortBy: state.sortField,
      sortDir: state.sortDir,
    );
    final folders = await _bridge.listFolders(mediaType: mt);
    final devices = await _bridge.listDevices();
    final pinned = await _bridge.listPinnedFolders();
    final settings = await _bridge.getSettings();

    state = state.copyWith(
      items: page.items,
      folders: folders,
      devices: devices,
      pinnedFolders: pinned,
      settings: settings,
    );
  }

  Future<void> selectSection(NavSection section) async {
    state = state.copyWith(section: section, clearFolder: true);
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
  }

  Future<void> applyRealRename(String mediaId, String newName) async {
    await _bridge.applyRealRename(mediaId, newName);
    await refresh();
  }

  Future<void> updateCoverAspect(String mediaId, AspectPreset preset) async {
    await _bridge.updateCoverAspect(mediaId, preset);
    await refresh();
  }

  Future<void> rerollVideoThumbnail(String mediaId) async {
    await _bridge.rerollVideoThumbnail(mediaId);
    await refresh();
  }

  Future<void> downloadRemote(String mediaId) async {
    await _bridge.downloadRemoteMedia(mediaId);
    await refresh();
  }

  Future<void> moveToTrash(String mediaId) async {
    await _bridge.moveToTrash(mediaId);
    state = state.copyWith(clearSelection: true);
    await refresh();
  }

  Future<void> addPinnedFolder(String folderPath) async {
    final label = folderPath.split('/').where((e) => e.isNotEmpty).lastOrNull ?? 'Folder';
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
}

final galleryControllerProvider =
    StateNotifierProvider<GalleryController, GalleryUiState>((ref) {
  return GalleryController(ref.watch(bridgeProvider));
});
