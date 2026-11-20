import 'dart:convert';
import 'dart:io';
import 'package:flutter/services.dart';
import '../../src/rust/api.dart' as rust;
import '../../src/rust/frb_generated.dart';
import '../models/media_models.dart';
import 'core_bridge.dart';

/// Live FFI-backed bridge connecting Flutter directly to `crates/core-api` (Rust + SQLite + iroh).
class LiveCoreBridge implements CoreBridge {
  LiveCoreBridge._({
    required this.dataDir,
    required this.dbPath,
    required this.cacheDir,
    required this.selfDeviceId,
    required this.selfDeviceName,
  });

  static const MethodChannel _androidChannel = MethodChannel('io.mediagallery.app/mediastore');

  final String dataDir;
  final String dbPath;
  final String cacheDir;
  final String selfDeviceId;
  final String selfDeviceName;

  File get _extendedPrefsFile => File('$dataDir/user_prefs.json');

  static Future<LiveCoreBridge> initialize() async {
    await RustLib.init();

    String dataDir;
    String thumbCache;
    String deviceName = Platform.localHostname;
    final bool isPhone = Platform.isAndroid;

    if (isPhone) {
      final dirs = await _androidChannel.invokeMapMethod<String, dynamic>('getAppDirs');
      dataDir = (dirs?['filesDir'] as String?) ?? Directory.systemTemp.path;
      thumbCache = '${(dirs?['cacheDir'] as String?) ?? dataDir}/thumbs';
      final model = dirs?['deviceName'] as String?;
      if (model != null && model.trim().isNotEmpty) {
        deviceName = model.trim();
      }
    } else if (Platform.isWindows) {
      final localApp = Platform.environment['LOCALAPPDATA'] ?? Directory.systemTemp.path;
      dataDir = '$localApp/MediaGallery';
      thumbCache = '$dataDir/thumbs';
    } else {
      final home = Platform.environment['HOME'] ?? Directory.systemTemp.path;
      dataDir = '$home/.local/share/MediaGallery';
      thumbCache = '$home/.cache/MediaGallery/thumbs';
    }

    await Directory(dataDir).create(recursive: true);
    await Directory(thumbCache).create(recursive: true);

    final dbPath = '$dataDir/media_gallery.db';
    final deviceId = isPhone ? 'android-local' : 'pc-${Platform.localHostname.toLowerCase()}';

    await rust.frbInitCore(
      dbPath: dbPath,
      cacheDir: thumbCache,
      selfDeviceId: deviceId,
      selfDeviceName: deviceName,
      isPhone: isPhone,
    );

    return LiveCoreBridge._(
      dataDir: dataDir,
      dbPath: dbPath,
      cacheDir: thumbCache,
      selfDeviceId: deviceId,
      selfDeviceName: deviceName,
    );
  }

  /// Discovers default user media folders on desktop or queries Android MediaStore.
  /// When [silent] is true, runs in the background without prompting for Android permissions.
  Future<int> runDefaultInitialScan({bool silent = false}) async {
    if (Platform.isAndroid) {
      final method = silent ? 'scanMediaStoreSilent' : 'scanMediaStore';
      final rowsJson = await _androidChannel.invokeMethod<String>(method);
      if (rowsJson != null && rowsJson.isNotEmpty && rowsJson != '[]') {
        return ingestAndroidMediaStore(selfDeviceId, selfDeviceName, rowsJson);
      }
      return 0;
    }

    final settings = await getSettings();
    final roots = <String>[...settings.scanFolders];
    if (roots.isEmpty) {
      final home = Platform.isWindows
          ? Platform.environment['USERPROFILE']
          : Platform.environment['HOME'];
      if (home != null && home.isNotEmpty) {
        for (final sub in ['Pictures', 'Videos', 'Music', 'Downloads']) {
          final dir = Directory('$home${Platform.pathSeparator}$sub');
          if (dir.existsSync()) {
            roots.add(dir.path);
          }
        }
      }
    }
    if (roots.isEmpty) return 0;
    return scanFolders(roots, excluded: settings.excludedFolders);
  }

  @override
  bool get isLiveCore => true;

  @override
  Future<int> scanFolders(List<String> roots, {List<String> excluded = const []}) async {
    final count = await rust.frbScanFolders(roots: roots, excluded: excluded);
    return count.toInt();
  }

  @override
  Future<int> ingestAndroidMediaStore(
    String deviceId,
    String deviceName,
    String rowsJson,
  ) async {
    return rust.frbIngestAndroidMediastore(
      deviceId: deviceId,
      deviceName: deviceName,
      rowsJson: rowsJson,
    );
  }

  @override
  Future<MediaPage> listMedia({
    required MediaFilter filter,
    SortField sortBy = SortField.date,
    SortDir sortDir = SortDir.desc,
    String? cursor,
    int limit = 200,
  }) async {
    final page = await rust.frbListMedia(
      query: filter.query,
      mediaType: filter.type?.name,
      deviceId: filter.deviceId,
      folderPath: filter.folderPath,
      syncState: filter.syncState != null ? _syncToString(filter.syncState!) : null,
      minSize: filter.minSize != null ? BigInt.from(filter.minSize!) : null,
      onlyDuplicates: filter.onlyDuplicates,
      trashedOnly: filter.trashedOnly,
      sortField: sortBy.name,
      sortDir: sortDir.name,
      cursor: cursor,
      limit: limit,
    );
    return MediaPage(
      items: page.items.map(_mapMedia).toList(),
      totalCount: page.totalCount.toInt(),
      nextCursor: page.nextCursor,
    );
  }

  @override
  Future<List<FolderSummary>> listFolders({String? deviceId, MediaType? mediaType}) async {
    final list = await rust.frbListFolders(
      deviceId: deviceId,
      mediaType: mediaType?.name,
    );
    return list
        .map(
          (f) => FolderSummary(
            path: f.path,
            name: f.name,
            deviceId: f.deviceId,
            itemCount: f.itemCount.toInt(),
            previewCovers: f.previewCovers,
          ),
        )
        .toList();
  }

  @override
  Future<List<DeviceItem>> listDevices() async {
    final devs = await rust.frbListDevices();
    return devs
        .map(
          (d) => DeviceItem(
            id: d.id,
            name: d.name,
            type: d.deviceType == 'phone' ? DeviceType.phone : DeviceType.pc,
            transports: d.transports.map(_parseTransport).whereType<TransportKind>().toList(),
            isSelf: d.isSelf,
            isOnline: d.isOnline,
            pairedAt: d.pairedAt?.toInt(),
          ),
        )
        .toList();
  }

  @override
  Future<List<PinnedFolderItem>> listPinnedFolders() async {
    final pins = await rust.frbListPinnedFolders();
    return pins
        .map(
          (p) => PinnedFolderItem(
            id: p.id,
            deviceId: p.deviceId,
            path: p.path,
            label: p.label,
            position: p.position,
          ),
        )
        .toList();
  }

  @override
  Future<AppSettings> getSettings() async {
    final s = await rust.frbGetSettings();
    var merged = AppSettings(
      sidebarWidth: s.sidebarWidth.toDouble(),
      sidebarCollapsed: s.sidebarCollapsed,
      darkMode: s.darkMode,
      manualTransportOverride:
          s.manualTransportOverride != null ? _parseTransport(s.manualTransportOverride!) : null,
      aliasTemplate: s.aliasTemplate,
      scanFolders: s.scanFolders,
      excludedFolders: s.excludedFolders,
    );
    try {
      if (_extendedPrefsFile.existsSync()) {
        final raw = await _extendedPrefsFile.readAsString();
        final decoded = jsonDecode(raw);
        if (decoded is Map<String, dynamic>) {
          merged = merged.withExtendedPrefsJson(decoded);
        }
      }
    } catch (_) {
      // Ignore malformed prefs file and fall back to defaults.
    }
    return merged;
  }

  @override
  Future<void> saveSettings(AppSettings settings) async {
    try {
      await _extendedPrefsFile.writeAsString(jsonEncode(settings.toExtendedPrefsJson()));
    } catch (_) {
      // Non-fatal: still persist core settings in SQLite.
    }
    await rust.frbSaveSettings(
      settings: rust.FrbAppSettings(
        sidebarWidth: settings.sidebarWidth.round(),
        sidebarCollapsed: settings.sidebarCollapsed,
        darkMode: settings.darkMode,
        manualTransportOverride: settings.manualTransportOverride?.name,
        aliasTemplate: settings.aliasTemplate,
        scanFolders: settings.scanFolders,
        excludedFolders: settings.excludedFolders,
      ),
    );
  }

  @override
  Future<StorageStats> getStorageStats() async {
    final s = await rust.frbGetStorageStats();
    return StorageStats(
      totalFiles: s.totalFiles.toInt(),
      totalBytes: s.totalBytes.toInt(),
      videoCount: s.videoCount.toInt(),
      videoBytes: s.videoBytes.toInt(),
      imageCount: s.imageCount.toInt(),
      imageBytes: s.imageBytes.toInt(),
      audioCount: s.audioCount.toInt(),
      audioBytes: s.audioBytes.toInt(),
      duplicateGroups: s.duplicateGroups.toInt(),
      duplicateExtraBytes: s.duplicateExtraBytes.toInt(),
      trashedCount: s.trashedCount.toInt(),
      trashedBytes: s.trashedBytes.toInt(),
      largeFilesCount: s.largeFilesCount.toInt(),
    );
  }

  @override
  Future<Map<String, String>> ensureThumbnailsBatch(List<String> mediaIds) async {
    if (mediaIds.isEmpty) return {};
    final entries = await rust.frbEnsureThumbnailsBatch(mediaIds: mediaIds);
    return {for (final e in entries) e.mediaId: e.filePath};
  }

  @override
  Future<MediaItem> setAlias(String mediaId, String alias) async {
    return _mapMedia(await rust.frbSetAlias(mediaId: mediaId, alias: alias));
  }

  @override
  Future<MediaItem> applyRealRename(String mediaId, String newName) async {
    return _mapMedia(await rust.frbApplyRealRename(mediaId: mediaId, newName: newName));
  }

  @override
  Future<bool> undoLastRename(String? mediaId) async {
    return rust.frbUndoLastRename(mediaId: mediaId);
  }

  @override
  Future<List<BulkRenamePreview>> previewBulkRename(List<String> mediaIds, String template) async {
    final list = await rust.frbPreviewBulkRename(mediaIds: mediaIds, template: template);
    return list
        .map(
          (p) => BulkRenamePreview(
            mediaId: p.mediaId,
            oldFileName: p.oldFileName,
            newFileName: p.newFileName,
            conflict: p.conflict,
          ),
        )
        .toList();
  }

  @override
  Future<BulkRenameResult> applyBulkRename(List<String> mediaIds, String template) async {
    final r = await rust.frbApplyBulkRename(mediaIds: mediaIds, template: template);
    return BulkRenameResult(batchId: r.batchId, renamedCount: r.renamedCount);
  }

  @override
  Future<int> undoBulkRename(String batchId) async {
    return rust.frbUndoBulkRename(batchId: batchId);
  }

  @override
  Future<MediaItem> moveMediaToFolder(String mediaId, String targetFolder) async {
    return _mapMedia(
      await rust.frbMoveMediaToFolder(mediaId: mediaId, targetFolder: targetFolder),
    );
  }

  @override
  Future<void> moveToTrash(String mediaId) async {
    await rust.frbMoveToTrash(mediaId: mediaId);
  }

  @override
  Future<MediaItem> restoreFromTrash(String mediaId) async {
    return _mapMedia(await rust.frbRestoreFromTrash(mediaId: mediaId));
  }

  @override
  Future<int> emptyTrash({String? mediaId}) async {
    final count = await rust.frbEmptyTrash(mediaId: mediaId);
    return count.toInt();
  }

  @override
  Future<CoverItem> updateCoverAspect(String mediaId, AspectPreset preset) async {
    return _mapCover(
      await rust.frbUpdateCoverAspect(mediaId: mediaId, aspect: preset.name),
    );
  }

  @override
  Future<CoverItem> rerollVideoThumbnail(String mediaId) async {
    return _mapCover(await rust.frbRerollVideoThumbnail(mediaId: mediaId));
  }

  @override
  Future<CoverItem> pickVideoFrame(String mediaId, int frameTimeMs) async {
    return _mapCover(
      await rust.frbPickVideoFrame(mediaId: mediaId, frameTimeMs: BigInt.from(frameTimeMs)),
    );
  }

  @override
  Future<CoverItem> setCustomCover(
    String mediaId,
    String imagePath,
    AspectPreset preset, {
    bool confirmedEmbeddedWrite = true,
  }) async {
    return _mapCover(
      await rust.frbSetCustomCover(
        mediaId: mediaId,
        imagePath: imagePath,
        aspect: preset.name,
        confirmedEmbeddedWrite: confirmedEmbeddedWrite,
      ),
    );
  }

  @override
  Future<void> upsertPinnedFolder(PinnedFolderItem folder) async {
    await rust.frbUpsertPinnedFolder(
      folder: rust.FrbPinnedFolder(
        id: folder.id,
        deviceId: folder.deviceId,
        path: folder.path,
        label: folder.label,
        icon: 'folder',
        position: folder.position,
      ),
    );
  }

  @override
  Future<void> removePinnedFolder(String id) async {
    await rust.frbRemovePinnedFolder(id: id);
  }

  @override
  Future<void> reorderPinnedFolders(List<String> orderedIds) async {
    await rust.frbReorderPinnedFolders(orderedIds: orderedIds);
  }

  @override
  Future<void> downloadRemoteMedia(String mediaId) async {
    await rust.frbDownloadRemoteMedia(mediaId: mediaId);
  }

  @override
  Future<void> sendMediaToDevice(String mediaId, String targetDeviceId) async {
    await rust.frbSendMediaToDevice(mediaId: mediaId, targetDeviceId: targetDeviceId);
  }

  @override
  Future<int> backupLocalMediaToDevice(String targetDeviceId) async {
    return rust.frbBackupLocalMediaToDevice(targetDeviceId: targetDeviceId);
  }

  @override
  Future<List<TransferItem>> listTransfers() async {
    final list = await rust.frbListTransfers();
    return list
        .map(
          (t) => TransferItem(
            id: t.id,
            mediaId: t.mediaId,
            fileName: t.fileName,
            fromDevice: t.fromDevice,
            toDevice: t.toDevice,
            status: t.status,
            bytesDone: t.bytesDone.toInt(),
            bytesTotal: t.bytesTotal.toInt(),
            pathKind: t.pathKind,
            speedBps: t.speedBps.toInt(),
            error: t.error,
          ),
        )
        .toList();
  }

  @override
  Future<void> pauseTransfer(String transferId) async {
    await rust.frbPauseTransfer(transferId: transferId);
  }

  @override
  Future<void> cancelTransfer(String transferId) async {
    await rust.frbCancelTransfer(transferId: transferId);
  }

  @override
  Future<List<AutoSyncRuleItem>> listAutoSyncRules() async {
    final list = await rust.frbListAutoSyncRules();
    return list
        .map(
          (r) => AutoSyncRuleItem(
            id: r.id,
            name: r.name,
            sourceDeviceId: r.sourceDeviceId,
            targetDeviceId: r.targetDeviceId,
            folderPattern: r.folderPattern,
            enabled: r.enabled,
          ),
        )
        .toList();
  }

  @override
  Future<void> upsertAutoSyncRule({
    required String id,
    required String name,
    required String targetDeviceId,
    required String folderPattern,
    required bool enabled,
  }) async {
    await rust.frbUpsertAutoSyncRule(
      id: id,
      name: name,
      targetDeviceId: targetDeviceId,
      folderPattern: folderPattern,
      enabled: enabled,
    );
  }

  @override
  Future<String> createPairingPin() async {
    final s = await createPairingSession();
    return s.pin;
  }

  @override
  Future<PairingSessionInfo> createPairingSession() async {
    final s = await rust.frbCreatePairingSession();
    return PairingSessionInfo(
      pin: s.pin,
      qrPayload: s.qrPayload,
      expiresAt: s.expiresAt.toInt(),
    );
  }

  @override
  Future<DeviceItem> pairWithDevice(String payloadOrPin, {String? peerName}) async {
    final d = await rust.frbPairWithDevice(payloadOrPin: payloadOrPin, peerName: peerName);
    return DeviceItem(
      id: d.id,
      name: d.name,
      type: d.deviceType == 'phone' ? DeviceType.phone : DeviceType.pc,
      transports: d.transports.map(_parseTransport).whereType<TransportKind>().toList(),
      isSelf: d.isSelf,
      isOnline: d.isOnline,
      pairedAt: d.pairedAt?.toInt(),
    );
  }

  @override
  Future<void> forgetDevice(String deviceId) async {
    await rust.frbForgetDevice(deviceId: deviceId);
  }

  @override
  Future<List<DuplicateGroupItem>> listDuplicateGroups() async {
    final list = await rust.frbListDuplicateGroups();
    return list
        .map(
          (g) => DuplicateGroupItem(
            groupId: g.groupId,
            size: g.size.toInt(),
            reclaimableBytes: g.reclaimableBytes.toInt(),
            items: g.items.map(_mapMedia).toList(),
          ),
        )
        .toList();
  }

  @override
  Future<CleanupReport> cleanDuplicates({bool keepNewest = true}) async {
    final r = await rust.frbCleanDuplicates(keepNewest: keepNewest);
    return CleanupReport(
      cleanedCount: r.cleanedCount.toInt(),
      freedBytes: r.freedBytes.toInt(),
    );
  }

  @override
  Future<List<MusicGroupItem>> listUnifiedMusic(MusicGroupBy groupBy) async {
    final list = await rust.frbListUnifiedMusic(groupBy: groupBy.name);
    return list
        .map(
          (g) => MusicGroupItem(
            groupName: g.groupName,
            tracks: g.tracks
                .map(
                  (t) => UnifiedTrackItem(
                    canonicalKey: t.canonicalKey,
                    title: t.title,
                    artist: t.artist,
                    album: t.album,
                    durationMs: t.durationMs.toInt(),
                    primary: _mapMedia(t.primary),
                    sourceDevices: t.sourceDevices,
                  ),
                )
                .toList(),
          ),
        )
        .toList();
  }

  @override
  Future<List<SavedSearchItem>> listSavedSearches() async {
    final list = await rust.frbListSavedSearches();
    return list
        .map(
          (s) => SavedSearchItem(
            id: s.id,
            name: s.name,
            query: s.query,
            mediaType: s.mediaType != null ? _parseMediaType(s.mediaType!) : null,
          ),
        )
        .toList();
  }

  @override
  Future<void> saveSearch(String name, String query, {MediaType? mediaType}) async {
    await rust.frbSaveSearch(name: name, query: query, mediaType: mediaType?.name);
  }

  @override
  Future<void> enqueueYoutubeDownload({
    required String url,
    required String format,
    required String quality,
    required String destinationFolder,
  }) async {
    var resolvedDest = destinationFolder;
    final isRelative =
        !resolvedDest.startsWith('/') && !RegExp(r'^[A-Za-z]:[/\\]').hasMatch(resolvedDest);
    if (isRelative) {
      if (Platform.isAndroid) {
        resolvedDest = '$dataDir/$resolvedDest';
      } else {
        final home = Platform.isWindows
            ? Platform.environment['USERPROFILE']
            : Platform.environment['HOME'];
        if (home != null && home.isNotEmpty) {
          resolvedDest = '$home/$resolvedDest';
        }
      }
    }
    await rust.frbEnqueueYoutubeDownload(
      url: url,
      format: format,
      quality: quality,
      destinationFolder: resolvedDest,
    );
  }

  @override
  Future<List<DownloadJobItem>> listDownloadJobs() async {
    final jobs = await rust.frbListDownloadJobs();
    return jobs
        .map(
          (j) => DownloadJobItem(
            id: j.id,
            url: j.url,
            title: j.title,
            format: j.format,
            quality: j.quality,
            destinationFolder: j.destinationFolder,
            status: j.status,
            progress: j.progress,
            error: j.error,
          ),
        )
        .toList();
  }

  @override
  Future<bool> detectLegacyDatabase() async => false;

  @override
  Future<LegacyImportReport> importLegacyDatabase() async {
    return const LegacyImportReport(
      importedDevices: 0,
      importedMedia: 0,
      importedCovers: 0,
      importedPinnedFolders: 0,
      skippedItems: 0,
      authReloginRequired: false,
    );
  }

  MediaItem _mapMedia(rust.FrbMediaItem m) {
    return MediaItem(
      id: m.id,
      deviceId: m.deviceId,
      deviceName: m.deviceName.isEmpty ? selfDeviceName : m.deviceName,
      deviceOnline: m.deviceOnline,
      path: m.path,
      folderPath: m.folderPath,
      fileName: m.fileName,
      type: _parseMediaType(m.mediaType),
      size: m.size.toInt(),
      createdAt: m.createdAt.toInt(),
      durationMs: m.durationMs.toInt(),
      width: m.width,
      height: m.height,
      alias: m.alias,
      autoAlias: m.autoAlias,
      syncState: _parseSyncState(m.syncState),
      place: m.place,
      artist: m.artist,
      album: m.album,
      cover: m.cover != null ? _mapCover(m.cover!) : null,
      isDuplicate: m.isDuplicate,
      isTrashed: m.isTrashed,
    );
  }

  CoverItem _mapCover(rust.FrbCoverItem c) {
    return CoverItem(
      id: c.id,
      mediaId: c.mediaId,
      filePath: c.filePath,
      aspect: _parseAspect(c.aspect),
      frameTimeMs: c.frameTimeMs.toInt(),
    );
  }

  MediaType _parseMediaType(String raw) {
    switch (raw) {
      case 'video':
        return MediaType.video;
      case 'audio':
        return MediaType.audio;
      default:
        return MediaType.image;
    }
  }

  SyncState _parseSyncState(String raw) {
    switch (raw) {
      case 'remote':
        return SyncState.remote;
      case 'downloading':
        return SyncState.downloading;
      case 'synced':
        return SyncState.synced;
      case 'error':
        return SyncState.error;
      default:
        return SyncState.local;
    }
  }

  String _syncToString(SyncState s) {
    switch (s) {
      case SyncState.local:
        return 'Local';
      case SyncState.remote:
        return 'Remote';
      case SyncState.downloading:
        return 'Downloading';
      case SyncState.synced:
        return 'Synced';
      case SyncState.error:
        return 'Error';
    }
  }

  AspectPreset _parseAspect(String raw) {
    switch (raw) {
      case 'ratio4x3':
        return AspectPreset.ratio4x3;
      case 'ratio9x16':
        return AspectPreset.ratio9x16;
      case 'ratio1x1':
        return AspectPreset.ratio1x1;
      case 'custom':
        return AspectPreset.custom;
      default:
        return AspectPreset.ratio16x9;
    }
  }

  TransportKind? _parseTransport(String raw) {
    switch (raw) {
      case 'wired':
        return TransportKind.wired;
      case 'wifiLan':
        return TransportKind.wifiLan;
      case 'holePunched':
        return TransportKind.holePunched;
      case 'relay':
        return TransportKind.relay;
      case 'bluetooth':
        return TransportKind.bluetooth;
      default:
        return null;
    }
  }
}
