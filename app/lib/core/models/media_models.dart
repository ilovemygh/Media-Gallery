import 'package:flutter/foundation.dart';

enum MediaType { video, image, audio }

enum SyncState { local, remote, downloading, synced, error }

enum DeviceType { phone, pc }

enum TransportKind { wired, wifiLan, holePunched, relay, bluetooth }

enum AspectPreset { ratio4x3, ratio9x16, ratio16x9, ratio1x1, custom }

enum GridDensity { small, medium, large, list }

enum SortField { name, date, size, duration, type }

enum SortDir { asc, desc }

enum MusicGroupBy { all, artist, album, folder, device }

@immutable
class DeviceItem {
  const DeviceItem({
    required this.id,
    required this.name,
    required this.type,
    required this.transports,
    required this.isSelf,
    required this.isOnline,
    this.pairedAt,
  });

  final String id;
  final String name;
  final DeviceType type;
  final List<TransportKind> transports;
  final bool isSelf;
  final bool isOnline;
  final int? pairedAt;

  DeviceItem copyWith({bool? isOnline}) => DeviceItem(
        id: id,
        name: name,
        type: type,
        transports: transports,
        isSelf: isSelf,
        isOnline: isOnline ?? this.isOnline,
        pairedAt: pairedAt,
      );
}

@immutable
class CoverItem {
  const CoverItem({
    required this.id,
    required this.mediaId,
    required this.filePath,
    required this.aspect,
    required this.frameTimeMs,
  });

  final String id;
  final String mediaId;
  final String filePath;
  final AspectPreset aspect;
  final int frameTimeMs;

  CoverItem copyWith({AspectPreset? aspect, int? frameTimeMs, String? filePath}) =>
      CoverItem(
        id: id,
        mediaId: mediaId,
        filePath: filePath ?? this.filePath,
        aspect: aspect ?? this.aspect,
        frameTimeMs: frameTimeMs ?? this.frameTimeMs,
      );
}

@immutable
class MediaItem {
  const MediaItem({
    required this.id,
    required this.deviceId,
    required this.deviceName,
    required this.deviceOnline,
    required this.path,
    required this.folderPath,
    required this.fileName,
    required this.type,
    required this.size,
    required this.createdAt,
    required this.durationMs,
    required this.width,
    required this.height,
    required this.alias,
    required this.autoAlias,
    required this.syncState,
    this.place,
    this.artist,
    this.album,
    this.cover,
    this.isDuplicate = false,
    this.isTrashed = false,
  });

  final String id;
  final String deviceId;
  final String deviceName;
  final bool deviceOnline;
  final String path;
  final String folderPath;
  final String fileName;
  final MediaType type;
  final int size;
  final int createdAt;
  final int durationMs;
  final int width;
  final int height;
  final String alias;
  final String autoAlias;
  final SyncState syncState;
  final String? place;
  final String? artist;
  final String? album;
  final CoverItem? cover;
  final bool isDuplicate;
  final bool isTrashed;

  MediaItem copyWith({
    String? path,
    String? folderPath,
    String? fileName,
    String? alias,
    SyncState? syncState,
    CoverItem? cover,
    bool? isTrashed,
    bool? deviceOnline,
  }) =>
      MediaItem(
        id: id,
        deviceId: deviceId,
        deviceName: deviceName,
        deviceOnline: deviceOnline ?? this.deviceOnline,
        path: path ?? this.path,
        folderPath: folderPath ?? this.folderPath,
        fileName: fileName ?? this.fileName,
        type: type,
        size: size,
        createdAt: createdAt,
        durationMs: durationMs,
        width: width,
        height: height,
        alias: alias ?? this.alias,
        autoAlias: autoAlias,
        syncState: syncState ?? this.syncState,
        place: place,
        artist: artist,
        album: album,
        cover: cover ?? this.cover,
        isDuplicate: isDuplicate,
        isTrashed: isTrashed ?? this.isTrashed,
      );
}

@immutable
class FolderSummary {
  const FolderSummary({
    required this.path,
    required this.name,
    required this.deviceId,
    required this.itemCount,
    required this.previewCovers,
  });

  final String path;
  final String name;
  final String deviceId;
  final int itemCount;
  final List<String> previewCovers;
}

@immutable
class PinnedFolderItem {
  const PinnedFolderItem({
    required this.id,
    required this.deviceId,
    required this.path,
    required this.label,
    required this.position,
  });

  final String id;
  final String deviceId;
  final String path;
  final String label;
  final int position;

  PinnedFolderItem copyWith({String? label, int? position}) => PinnedFolderItem(
        id: id,
        deviceId: deviceId,
        path: path,
        label: label ?? this.label,
        position: position ?? this.position,
      );
}

@immutable
class MediaFilter {
  const MediaFilter({
    this.query = '',
    this.type,
    this.deviceId,
    this.folderPath,
    this.syncState,
    this.minSize,
    this.onlyDuplicates = false,
    this.trashedOnly = false,
  });

  final String query;
  final MediaType? type;
  final String? deviceId;
  final String? folderPath;
  final SyncState? syncState;
  final int? minSize;
  final bool onlyDuplicates;
  final bool trashedOnly;

  MediaFilter copyWith({
    String? query,
    MediaType? type,
    bool clearType = false,
    String? deviceId,
    bool clearDevice = false,
    String? folderPath,
    bool clearFolder = false,
    SyncState? syncState,
    bool clearSyncState = false,
    int? minSize,
    bool clearMinSize = false,
    bool? onlyDuplicates,
    bool? trashedOnly,
  }) =>
      MediaFilter(
        query: query ?? this.query,
        type: clearType ? null : (type ?? this.type),
        deviceId: clearDevice ? null : (deviceId ?? this.deviceId),
        folderPath: clearFolder ? null : (folderPath ?? this.folderPath),
        syncState: clearSyncState ? null : (syncState ?? this.syncState),
        minSize: clearMinSize ? null : (minSize ?? this.minSize),
        onlyDuplicates: onlyDuplicates ?? this.onlyDuplicates,
        trashedOnly: trashedOnly ?? this.trashedOnly,
      );
}

@immutable
class MediaPage {
  const MediaPage({
    required this.items,
    required this.totalCount,
    this.nextCursor,
  });

  final List<MediaItem> items;
  final int totalCount;
  final String? nextCursor;
}

@immutable
class AppSettings {
  static const List<String> allSupportedExtensions = [
    'mp4',
    'mkv',
    'webm',
    'mov',
    'avi',
    'jpg',
    'jpeg',
    'png',
    'gif',
    'webp',
    'heic',
    'mp3',
    'm4a',
    'flac',
    'wav',
    'ogg',
  ];

  const AppSettings({
    this.sidebarWidth = 248.0,
    this.sidebarCollapsed = false,
    this.darkMode = true,
    this.manualTransportOverride,
    this.aliasTemplate = '{date} {place} {name}',
    this.scanFolders = const [],
    this.excludedFolders = const [],
    this.openAtStartup = false,
    this.autoScanOnOpen = true,
    this.hideDuplicatesInGallery = true,
    this.skipSystemFolders = true,
    this.scanVideos = true,
    this.scanImages = true,
    this.scanAudio = true,
    this.enabledExtensions = const [],
    this.youtubeDefaultFormat = 'mp4',
    this.youtubeDefaultQuality = '1080p',
  });

  final double sidebarWidth;
  final bool sidebarCollapsed;
  final bool darkMode;
  final TransportKind? manualTransportOverride;
  final String aliasTemplate;
  final List<String> scanFolders;
  final List<String> excludedFolders;
  final bool openAtStartup;
  final bool autoScanOnOpen;
  final bool hideDuplicatesInGallery;
  final bool skipSystemFolders;
  final bool scanVideos;
  final bool scanImages;
  final bool scanAudio;
  final List<String> enabledExtensions;
  final String youtubeDefaultFormat;
  final String youtubeDefaultQuality;

  Set<String> get effectiveExtensions => enabledExtensions.isEmpty
      ? allSupportedExtensions.toSet()
      : enabledExtensions.map((e) => e.toLowerCase()).toSet();

  Map<String, dynamic> toExtendedPrefsJson() => {
        'open_at_startup': openAtStartup,
        'auto_scan_on_open': autoScanOnOpen,
        'hide_duplicates_in_gallery': hideDuplicatesInGallery,
        'skip_system_folders': skipSystemFolders,
        'scan_videos': scanVideos,
        'scan_images': scanImages,
        'scan_audio': scanAudio,
        'enabled_extensions': enabledExtensions,
        'youtube_default_format': youtubeDefaultFormat,
        'youtube_default_quality': youtubeDefaultQuality,
      };

  AppSettings withExtendedPrefsJson(Map<String, dynamic> json) {
    final rawExts = json['enabled_extensions'];
    final exts = rawExts is List
        ? rawExts.map((e) => e.toString().toLowerCase()).toList()
        : enabledExtensions;
    return copyWith(
      openAtStartup: json['open_at_startup'] is bool
          ? json['open_at_startup'] as bool
          : openAtStartup,
      autoScanOnOpen: json['auto_scan_on_open'] is bool
          ? json['auto_scan_on_open'] as bool
          : autoScanOnOpen,
      hideDuplicatesInGallery: json['hide_duplicates_in_gallery'] is bool
          ? json['hide_duplicates_in_gallery'] as bool
          : hideDuplicatesInGallery,
      skipSystemFolders: json['skip_system_folders'] is bool
          ? json['skip_system_folders'] as bool
          : skipSystemFolders,
      scanVideos: json['scan_videos'] is bool ? json['scan_videos'] as bool : scanVideos,
      scanImages: json['scan_images'] is bool ? json['scan_images'] as bool : scanImages,
      scanAudio: json['scan_audio'] is bool ? json['scan_audio'] as bool : scanAudio,
      enabledExtensions: exts,
      youtubeDefaultFormat: json['youtube_default_format'] is String
          ? json['youtube_default_format'] as String
          : youtubeDefaultFormat,
      youtubeDefaultQuality: json['youtube_default_quality'] is String
          ? json['youtube_default_quality'] as String
          : youtubeDefaultQuality,
    );
  }

  AppSettings copyWith({
    double? sidebarWidth,
    bool? sidebarCollapsed,
    bool? darkMode,
    TransportKind? manualTransportOverride,
    bool clearTransportOverride = false,
    String? aliasTemplate,
    List<String>? scanFolders,
    List<String>? excludedFolders,
    bool? openAtStartup,
    bool? autoScanOnOpen,
    bool? hideDuplicatesInGallery,
    bool? skipSystemFolders,
    bool? scanVideos,
    bool? scanImages,
    bool? scanAudio,
    List<String>? enabledExtensions,
    String? youtubeDefaultFormat,
    String? youtubeDefaultQuality,
  }) =>
      AppSettings(
        sidebarWidth: sidebarWidth ?? this.sidebarWidth,
        sidebarCollapsed: sidebarCollapsed ?? this.sidebarCollapsed,
        darkMode: darkMode ?? this.darkMode,
        manualTransportOverride: clearTransportOverride
            ? null
            : (manualTransportOverride ?? this.manualTransportOverride),
        aliasTemplate: aliasTemplate ?? this.aliasTemplate,
        scanFolders: scanFolders ?? this.scanFolders,
        excludedFolders: excludedFolders ?? this.excludedFolders,
        openAtStartup: openAtStartup ?? this.openAtStartup,
        autoScanOnOpen: autoScanOnOpen ?? this.autoScanOnOpen,
        hideDuplicatesInGallery: hideDuplicatesInGallery ?? this.hideDuplicatesInGallery,
        skipSystemFolders: skipSystemFolders ?? this.skipSystemFolders,
        scanVideos: scanVideos ?? this.scanVideos,
        scanImages: scanImages ?? this.scanImages,
        scanAudio: scanAudio ?? this.scanAudio,
        enabledExtensions: enabledExtensions ?? this.enabledExtensions,
        youtubeDefaultFormat: youtubeDefaultFormat ?? this.youtubeDefaultFormat,
        youtubeDefaultQuality: youtubeDefaultQuality ?? this.youtubeDefaultQuality,
      );
}

@immutable
class StorageStats {
  const StorageStats({
    this.totalFiles = 0,
    this.totalBytes = 0,
    this.videoCount = 0,
    this.videoBytes = 0,
    this.imageCount = 0,
    this.imageBytes = 0,
    this.audioCount = 0,
    this.audioBytes = 0,
    this.duplicateGroups = 0,
    this.duplicateExtraBytes = 0,
    this.trashedCount = 0,
    this.trashedBytes = 0,
    this.largeFilesCount = 0,
  });

  final int totalFiles;
  final int totalBytes;
  final int videoCount;
  final int videoBytes;
  final int imageCount;
  final int imageBytes;
  final int audioCount;
  final int audioBytes;
  final int duplicateGroups;
  final int duplicateExtraBytes;
  final int trashedCount;
  final int trashedBytes;
  final int largeFilesCount;
}

@immutable
class BulkRenamePreview {
  const BulkRenamePreview({
    required this.mediaId,
    required this.oldFileName,
    required this.newFileName,
    required this.conflict,
  });

  final String mediaId;
  final String oldFileName;
  final String newFileName;
  final bool conflict;
}

@immutable
class BulkRenameResult {
  const BulkRenameResult({
    required this.batchId,
    required this.renamedCount,
  });

  final String batchId;
  final int renamedCount;
}

@immutable
class DownloadJobItem {
  const DownloadJobItem({
    required this.id,
    required this.url,
    this.title,
    required this.format,
    required this.quality,
    required this.destinationFolder,
    required this.status,
    required this.progress,
    this.error,
  });

  final String id;
  final String url;
  final String? title;
  final String format;
  final String quality;
  final String destinationFolder;
  final String status;
  final double progress;
  final String? error;
}

@immutable
class PairingSessionInfo {
  const PairingSessionInfo({
    required this.pin,
    required this.qrPayload,
    required this.expiresAt,
  });

  final String pin;
  final String qrPayload;
  final int expiresAt;
}

@immutable
class DuplicateGroupItem {
  const DuplicateGroupItem({
    required this.groupId,
    required this.size,
    required this.reclaimableBytes,
    required this.items,
  });

  final String groupId;
  final int size;
  final int reclaimableBytes;
  final List<MediaItem> items;
}

@immutable
class CleanupReport {
  const CleanupReport({
    required this.cleanedCount,
    required this.freedBytes,
  });

  final int cleanedCount;
  final int freedBytes;
}

@immutable
class UnifiedTrackItem {
  const UnifiedTrackItem({
    required this.canonicalKey,
    required this.title,
    required this.artist,
    required this.album,
    required this.durationMs,
    required this.primary,
    required this.sourceDevices,
  });

  final String canonicalKey;
  final String title;
  final String artist;
  final String album;
  final int durationMs;
  final MediaItem primary;
  final List<String> sourceDevices;
}

@immutable
class MusicGroupItem {
  const MusicGroupItem({
    required this.groupName,
    required this.tracks,
  });

  final String groupName;
  final List<UnifiedTrackItem> tracks;
}

@immutable
class TransferItem {
  const TransferItem({
    required this.id,
    required this.mediaId,
    required this.fileName,
    required this.fromDevice,
    required this.toDevice,
    required this.status,
    required this.bytesDone,
    required this.bytesTotal,
    required this.pathKind,
    required this.speedBps,
    this.error,
  });

  final String id;
  final String mediaId;
  final String fileName;
  final String fromDevice;
  final String toDevice;
  final String status;
  final int bytesDone;
  final int bytesTotal;
  final String pathKind;
  final int speedBps;
  final String? error;

  double get progress => bytesTotal <= 0 ? 0.0 : (bytesDone / bytesTotal).clamp(0.0, 1.0);
}

@immutable
class AutoSyncRuleItem {
  const AutoSyncRuleItem({
    required this.id,
    required this.name,
    required this.sourceDeviceId,
    required this.targetDeviceId,
    required this.folderPattern,
    required this.enabled,
  });

  final String id;
  final String name;
  final String sourceDeviceId;
  final String targetDeviceId;
  final String folderPattern;
  final bool enabled;
}

@immutable
class SavedSearchItem {
  const SavedSearchItem({
    required this.id,
    required this.name,
    required this.query,
    this.mediaType,
  });

  final String id;
  final String name;
  final String query;
  final MediaType? mediaType;
}

@immutable
class LegacyImportReport {
  const LegacyImportReport({
    required this.importedDevices,
    required this.importedMedia,
    required this.importedCovers,
    required this.importedPinnedFolders,
    required this.skippedItems,
    required this.authReloginRequired,
  });

  final int importedDevices;
  final int importedMedia;
  final int importedCovers;
  final int importedPinnedFolders;
  final int skippedItems;
  final bool authReloginRequired;
}
