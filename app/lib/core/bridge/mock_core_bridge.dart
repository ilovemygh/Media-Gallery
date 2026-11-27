import '../models/media_models.dart';
import 'core_bridge.dart';

/// In-memory deterministic bridge for Flutter widget/golden tests and UI development
/// without requiring native Rust/FFI binaries.
class MockCoreBridge implements CoreBridge {
  MockCoreBridge({bool simulateLegacyPresent = false})
      : _legacyAvailable = simulateLegacyPresent {
    _seed();
  }

  final bool _legacyAvailable;
  bool _legacyImported = false;
  AppSettings _settings = const AppSettings();
  final List<DeviceItem> _devices = [];
  final List<MediaItem> _media = [];
  final List<PinnedFolderItem> _pinned = [];
  final List<DownloadJobItem> _downloadJobs = [];
  final List<TransferItem> _transfers = [];
  final List<AutoSyncRuleItem> _syncRules = [];
  final List<SavedSearchItem> _savedSearches = [];
  final List<({String mediaId, String oldName, String oldFolder, String oldPath})> _renameUndo = [];
  final Map<String, List<({String mediaId, String oldName, String oldPath})>> _bulkUndo = {};

  @override
  bool get isLiveCore => false;

  @override
  Future<int> scanFolders(List<String> roots, {List<String> excluded = const []}) async {
    return _media.length;
  }

  @override
  Future<int> ingestAndroidMediaStore(
    String deviceId,
    String deviceName,
    String rowsJson,
  ) async {
    return 0;
  }

  void _seed() {
    _devices.addAll(const [
      DeviceItem(
        id: 'pc-local',
        name: 'Windows Workstation',
        type: DeviceType.pc,
        transports: [TransportKind.wired, TransportKind.wifiLan],
        isSelf: true,
        isOnline: true,
        pairedAt: 1773489600000,
      ),
      DeviceItem(
        id: 'phone-pixel',
        name: 'Pixel 9 Pro',
        type: DeviceType.phone,
        transports: [TransportKind.wifiLan, TransportKind.relay, TransportKind.bluetooth],
        isSelf: false,
        isOnline: true,
        pairedAt: 1773489600000,
      ),
    ]);

    _media.addAll(const [
      MediaItem(
        id: 'm-1',
        deviceId: 'pc-local',
        deviceName: 'Windows Workstation',
        deviceOnline: true,
        path: 'C:/Media/Camera/IMG_2031.jpg',
        folderPath: 'C:/Media/Camera',
        fileName: 'IMG_2031.jpg',
        type: MediaType.image,
        size: 4820110,
        createdAt: 1773489600000,
        durationMs: 0,
        width: 4032,
        height: 3024,
        alias: '2026-03-14 Kuta Beach Bali',
        autoAlias: '2026-03-14 Kuta Beach Bali',
        syncState: SyncState.local,
        place: 'Kuta Beach Bali',
        cover: CoverItem(
          id: 'cov-1',
          mediaId: 'm-1',
          filePath: '',
          aspect: AspectPreset.ratio4x3,
          frameTimeMs: 0,
        ),
      ),
      MediaItem(
        id: 'm-2',
        deviceId: 'pc-local',
        deviceName: 'Windows Workstation',
        deviceOnline: true,
        path: 'C:/Media/Trips/VID_0042.mp4',
        folderPath: 'C:/Media/Trips',
        fileName: 'VID_0042.mp4',
        type: MediaType.video,
        size: 184500000,
        createdAt: 1773493200000,
        durationMs: 96000,
        width: 3840,
        height: 2160,
        alias: '2026-03-14 Kuta Beach Sunset',
        autoAlias: '2026-03-14 Kuta Beach Sunset',
        syncState: SyncState.local,
        place: 'Kuta Beach Bali',
        cover: CoverItem(
          id: 'cov-2',
          mediaId: 'm-2',
          filePath: '',
          aspect: AspectPreset.ratio16x9,
          frameTimeMs: 18400,
        ),
      ),
      MediaItem(
        id: 'm-3',
        deviceId: 'pc-local',
        deviceName: 'Windows Workstation',
        deviceOnline: true,
        path: 'C:/Media/Music/midnight_echoes.mp3',
        folderPath: 'C:/Media/Music',
        fileName: 'midnight_echoes.mp3',
        type: MediaType.audio,
        size: 8940000,
        createdAt: 1773500000000,
        durationMs: 214000,
        width: 0,
        height: 0,
        alias: 'Nova Drift - Midnight Echoes',
        autoAlias: 'Nova Drift - Midnight Echoes',
        syncState: SyncState.local,
        artist: 'Nova Drift',
        album: 'Neon Skies',
        cover: CoverItem(
          id: 'cov-3',
          mediaId: 'm-3',
          filePath: '',
          aspect: AspectPreset.ratio1x1,
          frameTimeMs: 0,
        ),
      ),
      MediaItem(
        id: 'm-4',
        deviceId: 'phone-pixel',
        deviceName: 'Pixel 9 Pro',
        deviceOnline: true,
        path: '/storage/emulated/0/DCIM/Camera/VID_surfClip.mp4',
        folderPath: '/storage/emulated/0/DCIM/Camera',
        fileName: 'VID_surfClip.mp4',
        type: MediaType.video,
        size: 245000000,
        createdAt: 1773600000000,
        durationMs: 48000,
        width: 1920,
        height: 1080,
        alias: '2026-03-15 Bondi Beach surf Clip',
        autoAlias: '2026-03-15 Bondi Beach surf Clip',
        syncState: SyncState.remote,
        place: 'Bondi Beach Sydney',
        cover: CoverItem(
          id: 'cov-4',
          mediaId: 'm-4',
          filePath: '',
          aspect: AspectPreset.ratio16x9,
          frameTimeMs: 8200,
        ),
      ),
    ]);

    _pinned.addAll(const [
      PinnedFolderItem(
        id: 'pin-cam',
        deviceId: 'pc-local',
        path: 'C:/Media/Camera',
        label: 'Camera',
        position: 0,
      ),
      PinnedFolderItem(
        id: 'pin-trips',
        deviceId: 'pc-local',
        path: 'C:/Media/Trips',
        label: 'Trips',
        position: 1,
      ),
      PinnedFolderItem(
        id: 'pin-music',
        deviceId: 'pc-local',
        path: 'C:/Media/Music',
        label: 'Music',
        position: 2,
      ),
    ]);
  }

  @override
  Future<MediaPage> listMedia({
    required MediaFilter filter,
    SortField sortBy = SortField.date,
    SortDir sortDir = SortDir.desc,
    String? cursor,
    int limit = 200,
  }) async {
    final tokens = filter.query
        .trim()
        .toLowerCase()
        .split(RegExp(r'\s+'))
        .where((t) => t.isNotEmpty)
        .toList();

    final filtered = _media.where((item) {
      if (filter.trashedOnly) {
        if (!item.isTrashed) return false;
      } else if (item.isTrashed) {
        return false;
      }
      if (filter.onlyDuplicates && !item.isDuplicate) return false;
      if (filter.minSize != null && item.size < filter.minSize!) return false;
      if (filter.type != null && item.type != filter.type) return false;
      if (filter.deviceId != null && item.deviceId != filter.deviceId) return false;
      if (filter.folderPath != null && item.folderPath != filter.folderPath) return false;
      if (filter.syncState != null && item.syncState != filter.syncState) return false;
      if (tokens.isNotEmpty) {
        final hay = '${item.fileName} ${item.alias} ${item.autoAlias} ${item.place ?? ''} ${item.folderPath} ${item.deviceName} 2026'.toLowerCase();
        for (final tok in tokens) {
          if (tok == 'res:4k' || tok == 'res:uhd') {
            if (item.width < 3840) return false;
          } else if (tok == 'res:hd' || tok == 'res:1080p' || tok == 'res:720p') {
            if (item.width < 1280) return false;
          } else if (tok == 'len:long') {
            if (item.durationMs < 300000) return false;
          } else if (tok == 'len:short') {
            if (item.durationMs <= 0 || item.durationMs >= 60000) return false;
          } else if (tok.startsWith('ext:')) {
            final ext = tok.substring(4);
            if (!item.fileName.toLowerCase().endsWith('.$ext')) return false;
          } else if (tok.startsWith('time:')) {
            final hr = DateTime.fromMillisecondsSinceEpoch(item.createdAt).hour;
            final t = tok.substring(5);
            if (t == 'morning' && (hr < 5 || hr > 11)) return false;
            if (t == 'afternoon' && (hr < 12 || hr > 16)) return false;
            if (t == 'evening' && (hr < 17 || hr > 20)) return false;
            if (t == 'night' && hr > 4 && hr < 21) return false;
          } else if (!hay.contains(tok)) {
            return false;
          }
        }
      }
      return true;
    }).toList();

    final dir = sortDir == SortDir.asc ? 1 : -1;
    filtered.sort((a, b) {
      switch (sortBy) {
        case SortField.name:
          return a.alias.compareTo(b.alias) * dir;
        case SortField.size:
          return a.size.compareTo(b.size) * dir;
        case SortField.duration:
          return a.durationMs.compareTo(b.durationMs) * dir;
        case SortField.type:
          return a.type.name.compareTo(b.type.name) * dir;
        case SortField.date:
          return a.createdAt.compareTo(b.createdAt) * dir;
      }
    });

    final offset = int.tryParse(cursor ?? '0') ?? 0;
    final pageItems = filtered.skip(offset).take(limit).toList();
    final nextOffset = offset + pageItems.length;
    return MediaPage(
      items: pageItems,
      totalCount: filtered.length,
      nextCursor: nextOffset < filtered.length ? nextOffset.toString() : null,
    );
  }

  @override
  Future<List<FolderSummary>> listFolders({String? deviceId, MediaType? mediaType}) async {
    final map = <String, List<MediaItem>>{};
    for (final m in _media) {
      if (m.isTrashed) continue;
      if (deviceId != null && m.deviceId != deviceId) continue;
      if (mediaType != null && m.type != mediaType) continue;
      map.putIfAbsent('${m.deviceId}::${m.folderPath}', () => []).add(m);
    }
    return map.values.map((items) {
      final first = items.first;
      final parts = first.folderPath.split('/').where((s) => s.isNotEmpty).toList();
      return FolderSummary(
        path: first.folderPath,
        name: parts.isEmpty ? first.folderPath : parts.last,
        deviceId: first.deviceId,
        itemCount: items.length,
        previewCovers: items.take(4).map((e) => e.cover?.filePath ?? '').toList(),
      );
    }).toList();
  }

  @override
  Future<List<DeviceItem>> listDevices() async => List.unmodifiable(_devices);

  @override
  Future<List<PinnedFolderItem>> listPinnedFolders() async {
    final copy = [..._pinned]..sort((a, b) => a.position.compareTo(b.position));
    return copy;
  }

  @override
  Future<AppSettings> getSettings() async => _settings;

  @override
  Future<void> saveSettings(AppSettings settings) async {
    _settings = settings;
  }

  @override
  Future<MediaItem> setAlias(String mediaId, String alias) async {
    final idx = _media.indexWhere((m) => m.id == mediaId);
    _media[idx] = _media[idx].copyWith(alias: alias.trim());
    return _media[idx];
  }

  @override
  Future<MediaItem> applyRealRename(String mediaId, String newName) async {
    final idx = _media.indexWhere((m) => m.id == mediaId);
    final old = _media[idx];
    final dot = old.fileName.lastIndexOf('.');
    final ext = dot >= 0 ? old.fileName.substring(dot) : '';
    final finalName = newName.endsWith(ext) ? newName : '$newName$ext';
    _renameUndo.add((
      mediaId: old.id,
      oldName: old.fileName,
      oldFolder: old.folderPath,
      oldPath: old.path,
    ));
    _media[idx] = old.copyWith(
      fileName: finalName,
      path: '${old.folderPath}/$finalName',
      alias: newName,
    );
    return _media[idx];
  }

  @override
  Future<bool> undoLastRename(String? mediaId) async {
    final entryIdx = mediaId == null
        ? _renameUndo.length - 1
        : _renameUndo.lastIndexWhere((e) => e.mediaId == mediaId);
    if (entryIdx < 0) return false;
    final entry = _renameUndo.removeAt(entryIdx);
    final idx = _media.indexWhere((m) => m.id == entry.mediaId);
    if (idx >= 0) {
      _media[idx] = _media[idx].copyWith(
        fileName: entry.oldName,
        folderPath: entry.oldFolder,
        path: entry.oldPath,
      );
    }
    return true;
  }

  @override
  Future<MediaItem> moveMediaToFolder(String mediaId, String targetFolder) async {
    final idx = _media.indexWhere((m) => m.id == mediaId);
    final cur = _media[idx];
    _media[idx] = cur.copyWith(
      folderPath: targetFolder,
      path: '$targetFolder/${cur.fileName}',
    );
    return _media[idx];
  }

  @override
  Future<void> moveToTrash(String mediaId) async {
    final idx = _media.indexWhere((m) => m.id == mediaId);
    if (idx >= 0) {
      _media[idx] = _media[idx].copyWith(isTrashed: true);
    }
  }

  @override
  Future<MediaItem> restoreFromTrash(String mediaId) async {
    final idx = _media.indexWhere((m) => m.id == mediaId);
    if (idx >= 0) {
      _media[idx] = _media[idx].copyWith(isTrashed: false);
      return _media[idx];
    }
    throw StateError('Media not found: $mediaId');
  }

  @override
  Future<int> emptyTrash({String? mediaId}) async {
    final before = _media.length;
    if (mediaId != null) {
      _media.removeWhere((m) => m.id == mediaId && m.isTrashed);
    } else {
      _media.removeWhere((m) => m.isTrashed);
    }
    return before - _media.length;
  }

  @override
  Future<StorageStats> getStorageStats() async {
    var totalFiles = 0;
    var totalBytes = 0;
    var videoCount = 0;
    var videoBytes = 0;
    var imageCount = 0;
    var imageBytes = 0;
    var audioCount = 0;
    var audioBytes = 0;
    var trashedCount = 0;
    var trashedBytes = 0;
    var largeFilesCount = 0;
    var dupCount = 0;
    var dupExtraBytes = 0;

    for (final m in _media) {
      if (m.isTrashed) {
        trashedCount++;
        trashedBytes += m.size;
      } else {
        totalFiles++;
        totalBytes += m.size;
        if (m.size >= 52428800) largeFilesCount++;
        if (m.isDuplicate) {
          dupCount++;
          dupExtraBytes += m.size;
        }
        switch (m.type) {
          case MediaType.video:
            videoCount++;
            videoBytes += m.size;
          case MediaType.image:
            imageCount++;
            imageBytes += m.size;
          case MediaType.audio:
            audioCount++;
            audioBytes += m.size;
        }
      }
    }
    return StorageStats(
      totalFiles: totalFiles,
      totalBytes: totalBytes,
      videoCount: videoCount,
      videoBytes: videoBytes,
      imageCount: imageCount,
      imageBytes: imageBytes,
      audioCount: audioCount,
      audioBytes: audioBytes,
      duplicateGroups: dupCount > 0 ? 1 : 0,
      duplicateExtraBytes: dupExtraBytes,
      trashedCount: trashedCount,
      trashedBytes: trashedBytes,
      largeFilesCount: largeFilesCount,
    );
  }

  @override
  Future<Map<String, String>> ensureThumbnailsBatch(List<String> mediaIds) async => {};

  @override
  Future<List<BulkRenamePreview>> previewBulkRename(List<String> mediaIds, String template) async {
    final result = <BulkRenamePreview>[];
    for (var i = 0; i < mediaIds.length; i++) {
      final idx = _media.indexWhere((m) => m.id == mediaIds[i]);
      if (idx < 0) continue;
      final item = _media[idx];
      final dot = item.fileName.lastIndexOf('.');
      final ext = dot >= 0 ? item.fileName.substring(dot) : '';
      final stem = dot >= 0 ? item.fileName.substring(0, dot) : item.fileName;
      final replaced = template
          .replaceAll('{date}', '2026-03-14')
          .replaceAll('{place}', item.place ?? 'Local')
          .replaceAll('{name}', stem)
          .replaceAll('{index}', '${i + 1}')
          .trim()
          .replaceAll(RegExp(r'\s+'), ' ');
      final newFile = '$replaced$ext';
      result.add(
        BulkRenamePreview(
          mediaId: item.id,
          oldFileName: item.fileName,
          newFileName: newFile,
          conflict: false,
        ),
      );
    }
    return result;
  }

  @override
  Future<BulkRenameResult> applyBulkRename(List<String> mediaIds, String template) async {
    final previews = await previewBulkRename(mediaIds, template);
    final batchId = 'batch-${DateTime.now().millisecondsSinceEpoch}';
    final undoList = <({String mediaId, String oldName, String oldPath})>[];
    for (final p in previews) {
      final idx = _media.indexWhere((m) => m.id == p.mediaId);
      if (idx < 0) continue;
      final old = _media[idx];
      undoList.add((mediaId: old.id, oldName: old.fileName, oldPath: old.path));
      _media[idx] = old.copyWith(
        fileName: p.newFileName,
        path: '${old.folderPath}/${p.newFileName}',
        alias: p.newFileName,
      );
    }
    _bulkUndo[batchId] = undoList;
    return BulkRenameResult(batchId: batchId, renamedCount: undoList.length);
  }

  @override
  Future<int> undoBulkRename(String batchId) async {
    final entries = _bulkUndo.remove(batchId);
    if (entries == null) return 0;
    for (final e in entries) {
      final idx = _media.indexWhere((m) => m.id == e.mediaId);
      if (idx >= 0) {
        _media[idx] = _media[idx].copyWith(fileName: e.oldName, path: e.oldPath);
      }
    }
    return entries.length;
  }

  @override
  Future<CoverItem> updateCoverAspect(String mediaId, AspectPreset preset) async {
    final idx = _media.indexWhere((m) => m.id == mediaId);
    final cur = _media[idx].cover!;
    final updated = cur.copyWith(aspect: preset);
    _media[idx] = _media[idx].copyWith(cover: updated);
    return updated;
  }

  @override
  Future<CoverItem> rerollVideoThumbnail(String mediaId) async {
    final idx = _media.indexWhere((m) => m.id == mediaId);
    final cur = _media[idx].cover!;
    final third = (_media[idx].durationMs ~/ 3).clamp(1, 1 << 30);
    final updated = cur.copyWith(frameTimeMs: (cur.frameTimeMs + 3500) % third);
    _media[idx] = _media[idx].copyWith(cover: updated);
    return updated;
  }

  @override
  Future<CoverItem> pickVideoFrame(String mediaId, int frameTimeMs) async {
    final idx = _media.indexWhere((m) => m.id == mediaId);
    final cur = _media[idx].cover!;
    final updated = cur.copyWith(frameTimeMs: frameTimeMs);
    _media[idx] = _media[idx].copyWith(cover: updated);
    return updated;
  }

  @override
  Future<CoverItem> setCustomCover(
    String mediaId,
    String imagePath,
    AspectPreset preset, {
    bool confirmedEmbeddedWrite = true,
  }) async {
    final idx = _media.indexWhere((m) => m.id == mediaId);
    final cur = _media[idx].cover ??
        CoverItem(
          id: 'cov-$mediaId',
          mediaId: mediaId,
          filePath: imagePath,
          aspect: preset,
          frameTimeMs: 0,
        );
    final updated = cur.copyWith(filePath: imagePath, aspect: preset);
    _media[idx] = _media[idx].copyWith(cover: updated);
    return updated;
  }

  @override
  Future<void> upsertPinnedFolder(PinnedFolderItem folder) async {
    _pinned.removeWhere((p) => p.id == folder.id);
    _pinned.add(folder);
  }

  @override
  Future<void> removePinnedFolder(String id) async {
    _pinned.removeWhere((p) => p.id == id);
  }

  @override
  Future<void> reorderPinnedFolders(List<String> orderedIds) async {
    for (var i = 0; i < orderedIds.length; i++) {
      final idx = _pinned.indexWhere((p) => p.id == orderedIds[i]);
      if (idx >= 0) {
        _pinned[idx] = _pinned[idx].copyWith(position: i);
      }
    }
  }

  @override
  Future<void> downloadRemoteMedia(String mediaId) async {
    final idx = _media.indexWhere((m) => m.id == mediaId);
    if (idx >= 0) {
      _media[idx] = _media[idx].copyWith(syncState: SyncState.synced);
    }
  }

  @override
  Future<void> sendMediaToDevice(String mediaId, String targetDeviceId) async {
    final item = _media.firstWhere((m) => m.id == mediaId);
    _transfers.insert(
      0,
      TransferItem(
        id: 'tr-${DateTime.now().millisecondsSinceEpoch}',
        mediaId: mediaId,
        fileName: item.fileName,
        fromDevice: 'pc-local',
        toDevice: targetDeviceId,
        status: 'completed',
        bytesDone: item.size,
        bytesTotal: item.size,
        pathKind: 'WifiLan',
        speedBps: 45000000,
      ),
    );
  }

  @override
  Future<int> backupLocalMediaToDevice(String targetDeviceId) async {
    var count = 0;
    for (final m in _media.where((e) => !e.isTrashed && e.syncState == SyncState.local)) {
      await sendMediaToDevice(m.id, targetDeviceId);
      count++;
    }
    return count;
  }

  @override
  Future<List<TransferItem>> listTransfers() async => List.unmodifiable(_transfers);

  @override
  Future<void> pauseTransfer(String transferId) async {}

  @override
  Future<void> cancelTransfer(String transferId) async {
    _transfers.removeWhere((t) => t.id == transferId);
  }

  @override
  Future<List<AutoSyncRuleItem>> listAutoSyncRules() async => List.unmodifiable(_syncRules);

  @override
  Future<void> upsertAutoSyncRule({
    required String id,
    required String name,
    required String targetDeviceId,
    required String folderPattern,
    required bool enabled,
  }) async {
    _syncRules.removeWhere((r) => r.id == id);
    _syncRules.add(
      AutoSyncRuleItem(
        id: id,
        name: name,
        sourceDeviceId: 'pc-local',
        targetDeviceId: targetDeviceId,
        folderPattern: folderPattern,
        enabled: enabled,
      ),
    );
  }

  @override
  Future<String> createPairingPin() async => '482910';

  @override
  Future<PairingSessionInfo> createPairingSession() async => PairingSessionInfo(
        pin: '482910',
        qrPayload: 'mg://pair?pin=482910&device=pc-local',
        expiresAt: DateTime.now().millisecondsSinceEpoch + 300000,
      );

  @override
  Future<DeviceItem> pairWithDevice(String payloadOrPin, {String? peerName}) async {
    final dev = DeviceItem(
      id: 'peer-${payloadOrPin.trim()}',
      name: (peerName != null && peerName.trim().isNotEmpty)
          ? peerName.trim()
          : 'Paired Phone (${payloadOrPin.trim()})',
      type: DeviceType.phone,
      transports: const [TransportKind.wifiLan, TransportKind.relay],
      isSelf: false,
      isOnline: true,
      pairedAt: DateTime.now().millisecondsSinceEpoch,
    );
    _devices.removeWhere((d) => d.id == dev.id);
    _devices.add(dev);
    return dev;
  }

  @override
  Future<void> forgetDevice(String deviceId) async {
    _devices.removeWhere((d) => d.id == deviceId && !d.isSelf);
  }

  @override
  Future<List<DuplicateGroupItem>> listDuplicateGroups() async {
    final dups = _media.where((m) => m.isDuplicate && !m.isTrashed).toList();
    if (dups.isEmpty) return const [];
    return [
      DuplicateGroupItem(
        groupId: 'dup-mock',
        size: dups.first.size,
        reclaimableBytes: dups.skip(1).fold(0, (a, b) => a + b.size),
        items: dups,
      ),
    ];
  }

  @override
  Future<CleanupReport> cleanDuplicates({bool keepNewest = true}) async {
    final groups = await listDuplicateGroups();
    var count = 0;
    var freed = 0;
    for (final g in groups) {
      for (final m in g.items.skip(1)) {
        await moveToTrash(m.id);
        count++;
        freed += m.size;
      }
    }
    return CleanupReport(cleanedCount: count, freedBytes: freed);
  }

  @override
  Future<List<MusicGroupItem>> listUnifiedMusic(MusicGroupBy groupBy) async {
    final tracks = _media
        .where((m) => m.type == MediaType.audio && !m.isTrashed)
        .map(
          (m) => UnifiedTrackItem(
            canonicalKey: m.id,
            title: m.alias,
            artist: m.artist ?? 'Unknown Artist',
            album: m.album ?? 'Singles & Downloads',
            durationMs: m.durationMs,
            primary: m,
            sourceDevices: [m.deviceName],
          ),
        )
        .toList();
    if (tracks.isEmpty) return const [];
    if (groupBy == MusicGroupBy.all) {
      return [MusicGroupItem(groupName: 'All Tracks', tracks: tracks)];
    }
    final grouped = <String, List<UnifiedTrackItem>>{};
    for (final tr in tracks) {
      final key = switch (groupBy) {
        MusicGroupBy.artist => tr.artist,
        MusicGroupBy.album => tr.album,
        MusicGroupBy.folder => tr.primary.folderPath,
        MusicGroupBy.device => tr.primary.deviceName,
        MusicGroupBy.all => 'All Tracks',
      };
      grouped.putIfAbsent(key, () => []).add(tr);
    }
    return grouped.entries
        .map((e) => MusicGroupItem(groupName: e.key, tracks: e.value))
        .toList();
  }

  @override
  Future<List<SavedSearchItem>> listSavedSearches() async => List.unmodifiable(_savedSearches);

  @override
  Future<void> saveSearch(String name, String query, {MediaType? mediaType}) async {
    _savedSearches.add(
      SavedSearchItem(
        id: 'search-${DateTime.now().millisecondsSinceEpoch}',
        name: name,
        query: query,
        mediaType: mediaType,
      ),
    );
  }

  @override
  Future<void> enqueueYoutubeDownload({
    required String url,
    required String format,
    required String quality,
    required String destinationFolder,
  }) async {
    final isMp3 = format == 'mp3';
    final id = 'yt-${DateTime.now().millisecondsSinceEpoch}';
    _downloadJobs.insert(
      0,
      DownloadJobItem(
        id: id,
        url: url,
        title: 'YouTube Download ($quality)',
        format: format,
        quality: quality,
        destinationFolder: destinationFolder,
        status: 'completed',
        progress: 1.0,
      ),
    );
    _media.insert(
      0,
      MediaItem(
        id: id,
        deviceId: 'pc-local',
        deviceName: 'Windows Workstation',
        deviceOnline: true,
        path: '$destinationFolder/yt_download.$format',
        folderPath: destinationFolder,
        fileName: 'yt_download.$format',
        type: isMp3 ? MediaType.audio : MediaType.video,
        size: 12000000,
        createdAt: DateTime.now().millisecondsSinceEpoch,
        durationMs: 185000,
        width: isMp3 ? 0 : 1920,
        height: isMp3 ? 0 : 1080,
        alias: 'YouTube Download ($quality)',
        autoAlias: 'YouTube Download ($quality)',
        syncState: SyncState.local,
        artist: isMp3 ? 'YouTube Audio' : null,
      ),
    );
  }

  @override
  Future<List<DownloadJobItem>> listDownloadJobs() async => List.unmodifiable(_downloadJobs);

  @override
  Future<bool> detectLegacyDatabase() async => _legacyAvailable && !_legacyImported;

  @override
  Future<LegacyImportReport> importLegacyDatabase() async {
    _legacyImported = true;
    return const LegacyImportReport(
      importedDevices: 2,
      importedMedia: 6,
      importedCovers: 6,
      importedPinnedFolders: 4,
      skippedItems: 0,
      authReloginRequired: true,
    );
  }
}
