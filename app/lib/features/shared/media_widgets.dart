import 'dart:io';
import 'package:file_picker/file_picker.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_ui/material_ui.dart';
import 'package:media_kit_video/media_kit_video.dart';
import 'package:qr_flutter/qr_flutter.dart';
import '../../core/models/media_models.dart';
import '../../core/services/media_playback_service.dart';
import '../../core/state/gallery_providers.dart';

String formatBytes(int bytes) {
  if (bytes <= 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  var value = bytes.toDouble();
  var idx = 0;
  while (value >= 1024 && idx < units.length - 1) {
    value /= 1024;
    idx++;
  }
  return '${value.toStringAsFixed(idx == 0 ? 0 : 1)} ${units[idx]}';
}

String formatDuration(int durationMs) {
  if (durationMs <= 0) return '';
  final totalSec = durationMs ~/ 1000;
  final mins = totalSec ~/ 60;
  final secs = totalSec % 60;
  final hrs = mins ~/ 60;
  if (hrs > 0) {
    return '$hrs:${(mins % 60).toString().padLeft(2, '0')}:${secs.toString().padLeft(2, '0')}';
  }
  return '$mins:${secs.toString().padLeft(2, '0')}';
}

double? aspectRatioForPreset(AspectPreset preset) {
  switch (preset) {
    case AspectPreset.ratio4x3:
      return 4 / 3;
    case AspectPreset.ratio9x16:
      return 9 / 16;
    case AspectPreset.ratio16x9:
      return 16 / 9;
    case AspectPreset.ratio1x1:
      return 1;
    case AspectPreset.custom:
      return null;
  }
}

class MediaThumbnailWidget extends StatelessWidget {
  const MediaThumbnailWidget({
    super.key,
    required this.item,
    this.cacheWidth = 420,
    this.borderRadius = 8,
  });

  final MediaItem item;
  final int cacheWidth;
  final double borderRadius;

  String? _resolvableImagePath() {
    final coverPath = item.cover?.filePath ?? '';
    if (coverPath.isNotEmpty && File(coverPath).existsSync()) {
      return coverPath;
    }
    if (item.type == MediaType.image &&
        item.path.isNotEmpty &&
        !item.path.startsWith('content://') &&
        File(item.path).existsSync()) {
      return item.path;
    }
    return null;
  }

  @override
  Widget build(BuildContext context) {
    final filePath = _resolvableImagePath();
    final durationLabel = formatDuration(item.durationMs);
    final colors = Theme.of(context).colorScheme;

    return ClipRRect(
      borderRadius: BorderRadius.circular(borderRadius),
      child: Stack(
        fit: StackFit.expand,
        children: [
          Container(
            color: colors.surfaceContainerHighest,
            child: filePath != null
                ? Image.file(
                    File(filePath),
                    fit: BoxFit.cover,
                    cacheWidth: cacheWidth,
                    frameBuilder: (ctx, child, frame, wasSync) {
                      if (wasSync || frame != null) return child;
                      return _placeholderIcon(colors);
                    },
                    errorBuilder: (_, __, ___) => _placeholderIcon(colors),
                  )
                : _placeholderIcon(colors),
          ),
          if (durationLabel.isNotEmpty)
            Positioned(
              right: 6,
              bottom: 6,
              child: Container(
                padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                decoration: BoxDecoration(
                  color: Colors.black.withValues(alpha: 0.72),
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Text(
                  durationLabel,
                  style: const TextStyle(
                    color: Colors.white,
                    fontSize: 10,
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ),
            ),
          Positioned(
            left: 6,
            top: 6,
            child: Wrap(
              spacing: 4,
              children: [
                if (item.syncState == SyncState.remote)
                  _BadgePill(
                    icon: item.deviceOnline ? Icons.cloud_outlined : Icons.cloud_off,
                    label: item.deviceOnline ? 'Remote' : 'Offline',
                    color: Colors.blueGrey.shade800,
                  ),
                if (item.isDuplicate)
                  _BadgePill(
                    icon: Icons.content_copy,
                    label: 'DUP',
                    color: Colors.orange.shade800,
                  ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _placeholderIcon(ColorScheme colors) {
    return Center(
      child: Icon(
        item.type == MediaType.video
            ? Icons.movie_outlined
            : item.type == MediaType.audio
                ? Icons.music_note_outlined
                : Icons.image_outlined,
        size: 36,
        color: colors.onSurfaceVariant,
      ),
    );
  }
}

class _BadgePill extends StatelessWidget {
  const _BadgePill({
    required this.icon,
    required this.label,
    required this.color,
  });

  final IconData icon;
  final String label;
  final Color color;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 5, vertical: 2),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.88),
        borderRadius: BorderRadius.circular(4),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(icon, size: 11, color: Colors.white),
          const SizedBox(width: 3),
          Text(
            label,
            style: const TextStyle(color: Colors.white, fontSize: 10, fontWeight: FontWeight.w600),
          ),
        ],
      ),
    );
  }
}

class FolderMosaicPreview extends StatelessWidget {
  const FolderMosaicPreview({super.key, required this.folder});

  final FolderSummary folder;

  @override
  Widget build(BuildContext context) {
    final validCovers =
        folder.previewCovers.where((p) => p.isNotEmpty && File(p).existsSync()).take(4).toList();
    if (validCovers.isEmpty) {
      return const Icon(Icons.folder, size: 36);
    }
    return SizedBox(
      width: 56,
      height: 44,
      child: ClipRRect(
        borderRadius: BorderRadius.circular(6),
        child: GridView.builder(
          physics: const NeverScrollableScrollPhysics(),
          gridDelegate: const SliverGridDelegateWithFixedCrossAxisCount(
            crossAxisCount: 2,
            mainAxisSpacing: 1,
            crossAxisSpacing: 1,
          ),
          itemCount: validCovers.length.clamp(1, 4),
          itemBuilder: (_, idx) => Image.file(
            File(validCovers[idx]),
            fit: BoxFit.cover,
            cacheWidth: 120,
            errorBuilder: (_, __, ___) => const Icon(Icons.image, size: 14),
          ),
        ),
      ),
    );
  }
}

Future<void> showMediaViewerDialog(
  BuildContext context,
  WidgetRef ref,
  MediaItem item,
) async {
  if (item.type == MediaType.video || item.type == MediaType.audio) {
    await ref.read(galleryControllerProvider.notifier).playTrack(item);
  }
  if (!context.mounted) return;
  await showDialog<void>(
    context: context,
    builder: (ctx) => _MediaViewerDialog(initialItem: item),
  );
}

class _MediaViewerDialog extends ConsumerStatefulWidget {
  const _MediaViewerDialog({required this.initialItem});

  final MediaItem initialItem;

  @override
  ConsumerState<_MediaViewerDialog> createState() => _MediaViewerDialogState();
}

class _MediaViewerDialogState extends ConsumerState<_MediaViewerDialog> {
  late String _currentId;

  @override
  void initState() {
    super.initState();
    _currentId = widget.initialItem.id;
  }

  Future<void> _step(int delta, List<MediaItem> list) async {
    if (list.isEmpty) return;
    final idx = list.indexWhere((e) => e.id == _currentId);
    final nextIdx = idx < 0 ? 0 : (idx + delta) % list.length;
    final next = list[nextIdx < 0 ? list.length - 1 : nextIdx];
    setState(() => _currentId = next.id);
    if (next.type == MediaType.video || next.type == MediaType.audio) {
      await ref.read(galleryControllerProvider.notifier).playTrack(next);
    }
  }

  @override
  Widget build(BuildContext context) {
    final ui = ref.watch(galleryControllerProvider);
    final ctrl = ref.read(galleryControllerProvider.notifier);
    final item = ui.items.firstWhere(
      (e) => e.id == _currentId,
      orElse: () => widget.initialItem,
    );
    final coverPath = item.cover?.filePath ?? '';
    final diskImage = item.type == MediaType.image && File(item.path).existsSync()
        ? item.path
        : (coverPath.isNotEmpty && File(coverPath).existsSync() ? coverPath : null);
    final playback = MediaPlaybackService.instance;
    final showVideoSurface = item.type == MediaType.video &&
        playback.isNativeReady &&
        playback.videoController != null;

    return Dialog(
      insetPadding: const EdgeInsets.all(24),
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 960, maxHeight: 700),
        child: Column(
          children: [
            AppBar(
              title: Text(item.alias, overflow: TextOverflow.ellipsis),
              automaticallyImplyLeading: false,
              actions: [
                IconButton(
                  tooltip: 'Previous',
                  icon: const Icon(Icons.chevron_left),
                  onPressed: () => _step(-1, ui.items),
                ),
                IconButton(
                  tooltip: 'Next',
                  icon: const Icon(Icons.chevron_right),
                  onPressed: () => _step(1, ui.items),
                ),
                if (item.type == MediaType.audio || item.type == MediaType.video)
                  IconButton(
                    tooltip: ui.isPlaying ? 'Pause' : 'Play',
                    icon: Icon(ui.isPlaying ? Icons.pause_circle_filled : Icons.play_circle_fill),
                    onPressed: () => ctrl.playTrack(item),
                  ),
                IconButton(
                  tooltip: 'Move to Recycle Bin',
                  icon: const Icon(Icons.delete_outline),
                  onPressed: () async {
                    await ctrl.moveToTrash(item.id);
                    if (context.mounted) Navigator.of(context).pop();
                  },
                ),
                IconButton(
                  tooltip: 'Close',
                  icon: const Icon(Icons.close),
                  onPressed: () => Navigator.of(context).pop(),
                ),
              ],
            ),
            Expanded(
              child: Container(
                width: double.infinity,
                color: Colors.black,
                child: showVideoSurface
                    ? Video(controller: playback.videoController!)
                    : (diskImage != null
                        ? InteractiveViewer(
                            minScale: 0.5,
                            maxScale: 4.0,
                            child: Center(
                              child: Image.file(File(diskImage), fit: BoxFit.contain),
                            ),
                          )
                        : Center(
                            child: Column(
                              mainAxisSize: MainAxisSize.min,
                              children: [
                                Icon(
                                  item.type == MediaType.video
                                      ? Icons.movie
                                      : item.type == MediaType.audio
                                          ? Icons.music_note
                                          : Icons.image,
                                  size: 72,
                                  color: Colors.white70,
                                ),
                                const SizedBox(height: 12),
                                Text(
                                  item.fileName,
                                  style: const TextStyle(color: Colors.white),
                                ),
                                if (item.type != MediaType.image) ...[
                                  const SizedBox(height: 12),
                                  FilledButton.icon(
                                    onPressed: () => ctrl.playTrack(item),
                                    icon: const Icon(Icons.play_arrow),
                                    label: const Text('Play Track / Media'),
                                  ),
                                ],
                              ],
                            ),
                          )),
              ),
            ),
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
              child: Row(
                children: [
                  Expanded(
                    child: Text(
                      '${item.fileName} · ${formatBytes(item.size)} · ${item.deviceName}',
                      overflow: TextOverflow.ellipsis,
                    ),
                  ),
                  if (item.place != null)
                    Chip(
                      avatar: const Icon(Icons.place, size: 15),
                      label: Text(item.place!),
                    ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

Future<void> showBulkRenameDialog(
  BuildContext context,
  WidgetRef ref,
  List<String> mediaIds,
) async {
  await showDialog<void>(
    context: context,
    builder: (_) => _BulkRenameDialog(mediaIds: mediaIds),
  );
}

class _BulkRenameDialog extends ConsumerStatefulWidget {
  const _BulkRenameDialog({required this.mediaIds});

  final List<String> mediaIds;

  @override
  ConsumerState<_BulkRenameDialog> createState() => _BulkRenameDialogState();
}

class _BulkRenameDialogState extends ConsumerState<_BulkRenameDialog> {
  final TextEditingController _templateCtrl =
      TextEditingController(text: '{date} {place} {name}');
  List<BulkRenamePreview> _previews = const [];
  bool _loading = true;

  @override
  void initState() {
    super.initState();
    _refreshPreview();
  }

  @override
  void dispose() {
    _templateCtrl.dispose();
    super.dispose();
  }

  Future<void> _refreshPreview() async {
    setState(() => _loading = true);
    final list = await ref
        .read(bridgeProvider)
        .previewBulkRename(widget.mediaIds, _templateCtrl.text);
    if (mounted) {
      setState(() {
        _previews = list;
        _loading = false;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: Text('Bulk Rename (${widget.mediaIds.length} files)'),
      content: SizedBox(
        width: 560,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            TextField(
              controller: _templateCtrl,
              decoration: const InputDecoration(
                labelText: 'Pattern ({date}, {place}, {name}, {index})',
                border: OutlineInputBorder(),
                isDense: true,
              ),
              onChanged: (_) => _refreshPreview(),
            ),
            const SizedBox(height: 12),
            SizedBox(
              height: 220,
              child: _loading
                  ? const Center(child: CircularProgressIndicator())
                  : ListView.separated(
                      itemCount: _previews.length,
                      separatorBuilder: (_, __) => const Divider(height: 1),
                      itemBuilder: (_, idx) {
                        final p = _previews[idx];
                        return ListTile(
                          dense: true,
                          title: Text(p.oldFileName, overflow: TextOverflow.ellipsis),
                          subtitle: Text(
                            '→ ${p.newFileName}',
                            style: TextStyle(
                              color: p.conflict ? Colors.red : Colors.green,
                              fontWeight: FontWeight.w600,
                            ),
                            overflow: TextOverflow.ellipsis,
                          ),
                          trailing: p.conflict
                              ? const Icon(Icons.warning_amber, color: Colors.orange)
                              : const Icon(Icons.check, color: Colors.green, size: 18),
                        );
                      },
                    ),
            ),
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('Cancel'),
        ),
        FilledButton.icon(
          onPressed: _previews.isEmpty
              ? null
              : () async {
                  await ref
                      .read(galleryControllerProvider.notifier)
                      .applyBulkRename(widget.mediaIds, _templateCtrl.text);
                  if (context.mounted) Navigator.of(context).pop();
                },
          icon: const Icon(Icons.drive_file_rename_outline),
          label: const Text('Apply Rename'),
        ),
      ],
    );
  }
}

class CleanerDashboardView extends ConsumerWidget {
  const CleanerDashboardView({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final ui = ref.watch(galleryControllerProvider);
    final ctrl = ref.read(galleryControllerProvider.notifier);
    final stats = ui.stats;

    return Padding(
      padding: const EdgeInsets.all(14),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Card(
            child: Padding(
              padding: const EdgeInsets.all(14),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Wrap(
                    spacing: 8,
                    runSpacing: 8,
                    alignment: WrapAlignment.spaceBetween,
                    crossAxisAlignment: WrapCrossAlignment.center,
                    children: [
                      Text(
                        'Storage & Smart Cleanup',
                        style: Theme.of(context).textTheme.titleMedium,
                      ),
                      Chip(
                        avatar: const Icon(Icons.storage, size: 16),
                        label: Text(
                          '${stats.totalFiles} files · ${formatBytes(stats.totalBytes)}',
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 10),
                  Wrap(
                    spacing: 10,
                    runSpacing: 8,
                    children: [
                      _StatPill(
                        icon: Icons.movie,
                        label: 'Videos: ${stats.videoCount} (${formatBytes(stats.videoBytes)})',
                      ),
                      _StatPill(
                        icon: Icons.image,
                        label: 'Images: ${stats.imageCount} (${formatBytes(stats.imageBytes)})',
                      ),
                      _StatPill(
                        icon: Icons.music_note,
                        label: 'Audio: ${stats.audioCount} (${formatBytes(stats.audioBytes)})',
                      ),
                    ],
                  ),
                ],
              ),
            ),
          ),
          const SizedBox(height: 10),
          Wrap(
            spacing: 10,
            runSpacing: 10,
            children: [
              _CleanerActionCard(
                icon: Icons.content_copy,
                title: 'Duplicates',
                subtitle:
                    '${stats.duplicateGroups} group(s) · Save ${formatBytes(stats.duplicateExtraBytes)}',
                selected: ui.onlyDuplicates && !ui.trashedOnly,
                onTap: () => ctrl.openCleanerMode(duplicates: true),
              ),
              _CleanerActionCard(
                icon: Icons.data_usage,
                title: 'Large Files (>50 MB)',
                subtitle: '${stats.largeFilesCount} large file(s)',
                selected: ui.minSizeBytes != null && !ui.trashedOnly,
                onTap: () => ctrl.openCleanerMode(largeFiles: true),
              ),
              _CleanerActionCard(
                icon: Icons.delete_outline,
                title: 'Recycle Bin',
                subtitle: '${stats.trashedCount} item(s) · ${formatBytes(stats.trashedBytes)}',
                selected: ui.trashedOnly,
                onTap: () => ctrl.openCleanerMode(trashed: true),
              ),
            ],
          ),
          if (ui.onlyDuplicates && !ui.trashedOnly && stats.duplicateGroups > 0) ...[
            const SizedBox(height: 12),
            Wrap(
              spacing: 8,
              runSpacing: 8,
              children: [
                FilledButton.icon(
                  onPressed: () => ctrl.cleanAllDuplicates(keepNewest: true),
                  icon: const Icon(Icons.auto_fix_high),
                  label: Text(
                    'Auto-Clean Duplicates (Keep Newest · Save ${formatBytes(stats.duplicateExtraBytes)})',
                  ),
                ),
                OutlinedButton(
                  onPressed: () => ctrl.cleanAllDuplicates(keepNewest: false),
                  child: const Text('Keep Oldest'),
                ),
              ],
            ),
          ],
          if (ui.trashedOnly && ui.items.isNotEmpty) ...[
            const SizedBox(height: 12),
            FilledButton.icon(
              onPressed: () => ctrl.emptyTrash(),
              icon: const Icon(Icons.delete_forever),
              label: const Text('Empty Recycle Bin Permanently'),
            ),
          ],
        ],
      ),
    );
  }
}

class _StatPill extends StatelessWidget {
  const _StatPill({required this.icon, required this.label});

  final IconData icon;
  final String label;

  @override
  Widget build(BuildContext context) {
    return Chip(
      avatar: Icon(icon, size: 15),
      label: Text(label, style: const TextStyle(fontSize: 12)),
    );
  }
}

class _CleanerActionCard extends StatelessWidget {
  const _CleanerActionCard({
    required this.icon,
    required this.title,
    required this.subtitle,
    required this.selected,
    required this.onTap,
  });

  final IconData icon;
  final String title;
  final String subtitle;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    return SizedBox(
      width: 220,
      child: Card(
        color: selected ? colors.primaryContainer : null,
        child: InkWell(
          borderRadius: BorderRadius.circular(12),
          onTap: onTap,
          child: Padding(
            padding: const EdgeInsets.all(12),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Icon(icon, size: 24, color: colors.primary),
                const SizedBox(height: 8),
                Text(title, style: const TextStyle(fontWeight: FontWeight.w600)),
                const SizedBox(height: 4),
                Text(subtitle, style: const TextStyle(fontSize: 12)),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class DevicesBackupView extends ConsumerStatefulWidget {
  const DevicesBackupView({super.key});

  @override
  ConsumerState<DevicesBackupView> createState() => _DevicesBackupViewState();
}

class _DevicesBackupViewState extends ConsumerState<DevicesBackupView> {
  PairingSessionInfo? _session;
  final TextEditingController _pairCodeCtrl = TextEditingController();
  final TextEditingController _pairNameCtrl = TextEditingController();

  @override
  void dispose() {
    _pairCodeCtrl.dispose();
    _pairNameCtrl.dispose();
    super.dispose();
  }

  Future<void> _generateQr() async {
    final session = await ref.read(bridgeProvider).createPairingSession();
    if (mounted) setState(() => _session = session);
  }

  @override
  Widget build(BuildContext context) {
    final ui = ref.watch(galleryControllerProvider);
    final ctrl = ref.read(galleryControllerProvider.notifier);

    return Padding(
      padding: const EdgeInsets.all(14),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Card(
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    'Pair Phone & PC (Serverless P2P)',
                    style: Theme.of(context).textTheme.titleMedium,
                  ),
                  const SizedBox(height: 6),
                  const Text(
                    'Direct Wi-Fi LAN, USB, or hole-punched QUIC transfer — files never touch a cloud server.',
                  ),
                  const SizedBox(height: 12),
                  Wrap(
                    spacing: 12,
                    runSpacing: 12,
                    crossAxisAlignment: WrapCrossAlignment.center,
                    children: [
                      FilledButton.icon(
                        onPressed: _generateQr,
                        icon: const Icon(Icons.qr_code_2),
                        label: const Text('Show Pairing QR & 6-Digit PIN'),
                      ),
                      if (_session != null) ...[
                        Container(
                          padding: const EdgeInsets.all(8),
                          decoration: BoxDecoration(
                            color: Colors.white,
                            borderRadius: BorderRadius.circular(8),
                          ),
                          child: QrImageView(
                            data: _session!.qrPayload,
                            version: QrVersions.auto,
                            size: 130,
                          ),
                        ),
                        Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Chip(
                              avatar: const Icon(Icons.pin, size: 16),
                              label: Text(
                                'PIN: ${_session!.pin}',
                                style: const TextStyle(fontWeight: FontWeight.bold),
                              ),
                            ),
                            TextButton.icon(
                              onPressed: () {
                                Clipboard.setData(ClipboardData(text: _session!.qrPayload));
                              },
                              icon: const Icon(Icons.copy, size: 15),
                              label: const Text('Copy Pairing Payload'),
                            ),
                          ],
                        ),
                      ],
                    ],
                  ),
                  const Divider(height: 24),
                  Wrap(
                    spacing: 8,
                    runSpacing: 8,
                    crossAxisAlignment: WrapCrossAlignment.center,
                    children: [
                      SizedBox(
                        width: 220,
                        child: TextField(
                          controller: _pairCodeCtrl,
                          decoration: const InputDecoration(
                            labelText: 'Enter 6-digit PIN or QR payload',
                            border: OutlineInputBorder(),
                            isDense: true,
                          ),
                        ),
                      ),
                      SizedBox(
                        width: 170,
                        child: TextField(
                          controller: _pairNameCtrl,
                          decoration: const InputDecoration(
                            labelText: 'Device name (optional)',
                            border: OutlineInputBorder(),
                            isDense: true,
                          ),
                        ),
                      ),
                      FilledButton.tonalIcon(
                        onPressed: () async {
                          final code = _pairCodeCtrl.text.trim();
                          if (code.isEmpty) return;
                          await ctrl.pairWithDevice(code, peerName: _pairNameCtrl.text);
                          _pairCodeCtrl.clear();
                          _pairNameCtrl.clear();
                        },
                        icon: const Icon(Icons.link),
                        label: const Text('Pair Device'),
                      ),
                    ],
                  ),
                ],
              ),
            ),
          ),
          const SizedBox(height: 12),
          Card(
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text('Connected Devices & Quick Backup',
                      style: Theme.of(context).textTheme.titleMedium),
                  const SizedBox(height: 10),
                  for (final dev in ui.devices) ...[
                    ListTile(
                      leading: Icon(
                        dev.type == DeviceType.phone ? Icons.smartphone : Icons.computer,
                      ),
                      title: Text('${dev.name}${dev.isSelf ? " (This Device)" : ""}'),
                      subtitle: Text(
                        '${dev.isOnline ? "Online" : "Offline"} · ${dev.transports.map((t) => t.name).join(", ")}',
                      ),
                      trailing: dev.isSelf
                          ? const Chip(label: Text('Host'))
                          : Row(
                              mainAxisSize: MainAxisSize.min,
                              children: [
                                IconButton(
                                  tooltip: 'Backup Local Photos & Videos Now',
                                  icon: const Icon(Icons.backup),
                                  onPressed: () => ctrl.backupToDevice(dev.id),
                                ),
                                IconButton(
                                  tooltip: 'Forget device',
                                  icon: const Icon(Icons.link_off),
                                  onPressed: () async {
                                    await ref.read(bridgeProvider).forgetDevice(dev.id);
                                    await ctrl.refresh();
                                  },
                                ),
                              ],
                            ),
                    ),
                    if (!dev.isSelf)
                      Padding(
                        padding: const EdgeInsets.only(left: 16, right: 8, bottom: 6),
                        child: SwitchListTile(
                          dense: true,
                          title: Text('Auto-backup Camera & Pictures to ${dev.name}'),
                          value: ui.autoSyncRules
                              .any((r) => r.targetDeviceId == dev.id && r.enabled),
                          onChanged: (enabled) => ctrl.toggleAutoSyncRule(dev.id, enabled),
                        ),
                      ),
                  ],
                ],
              ),
            ),
          ),
          if (ui.transfers.isNotEmpty) ...[
            const SizedBox(height: 12),
            Card(
              child: Padding(
                padding: const EdgeInsets.all(16),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text('Transfers (${ui.transfers.length})',
                        style: Theme.of(context).textTheme.titleMedium),
                    const SizedBox(height: 8),
                    for (final tr in ui.transfers)
                      ListTile(
                        dense: true,
                        leading: const Icon(Icons.swap_horiz),
                        title: Text(tr.fileName, overflow: TextOverflow.ellipsis),
                        subtitle: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text(
                              '${tr.status} · ${tr.pathKind} · ${formatBytes(tr.bytesDone)} / ${formatBytes(tr.bytesTotal)}',
                            ),
                            LinearProgressIndicator(value: tr.progress),
                          ],
                        ),
                        trailing: Row(
                          mainAxisSize: MainAxisSize.min,
                          children: [
                            if (tr.status == 'transferring' || tr.status == 'queued')
                              IconButton(
                                tooltip: 'Pause',
                                icon: const Icon(Icons.pause),
                                onPressed: () => ctrl.pauseTransfer(tr.id),
                              ),
                            IconButton(
                              tooltip: 'Cancel',
                              icon: const Icon(Icons.close),
                              onPressed: () => ctrl.cancelTransfer(tr.id),
                            ),
                          ],
                        ),
                      ),
                  ],
                ),
              ),
            ),
          ],
        ],
      ),
    );
  }
}

class YouTubeDownloadView extends ConsumerStatefulWidget {
  const YouTubeDownloadView({super.key});

  @override
  ConsumerState<YouTubeDownloadView> createState() => _YouTubeDownloadViewState();
}

class _YouTubeDownloadViewState extends ConsumerState<YouTubeDownloadView> {
  final TextEditingController _urlCtrl = TextEditingController();
  String _format = 'mp4';
  String _quality = '1080p';
  bool _submitting = false;

  @override
  void dispose() {
    _urlCtrl.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final ui = ref.watch(galleryControllerProvider);

    return Padding(
      padding: const EdgeInsets.all(14),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Card(
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text('Download from YouTube', style: Theme.of(context).textTheme.titleMedium),
                  const SizedBox(height: 10),
                  TextField(
                    controller: _urlCtrl,
                    decoration: const InputDecoration(
                      labelText: 'Paste YouTube video or playlist URL',
                      border: OutlineInputBorder(),
                      isDense: true,
                    ),
                  ),
                  const SizedBox(height: 10),
                  Wrap(
                    spacing: 8,
                    runSpacing: 8,
                    crossAxisAlignment: WrapCrossAlignment.center,
                    children: [
                      ChoiceChip(
                        label: const Text('MP4 Video'),
                        selected: _format == 'mp4',
                        onSelected: (_) => setState(() {
                          _format = 'mp4';
                          _quality = '1080p';
                        }),
                      ),
                      ChoiceChip(
                        label: const Text('MP3 Audio'),
                        selected: _format == 'mp3',
                        onSelected: (_) => setState(() {
                          _format = 'mp3';
                          _quality = '320kbps';
                        }),
                      ),
                      DropdownButton<String>(
                        value: _quality,
                        items: (_format == 'mp4'
                                ? const ['1080p', '720p', '480p']
                                : const ['320kbps', '192kbps'])
                            .map((q) => DropdownMenuItem(value: q, child: Text(q)))
                            .toList(),
                        onChanged: (v) {
                          if (v != null) setState(() => _quality = v);
                        },
                      ),
                      FilledButton.icon(
                        onPressed: _submitting
                            ? null
                            : () async {
                                if (_urlCtrl.text.trim().isEmpty) return;
                                setState(() => _submitting = true);
                                try {
                                  await ref.read(bridgeProvider).enqueueYoutubeDownload(
                                        url: _urlCtrl.text.trim(),
                                        format: _format,
                                        quality: _quality,
                                        destinationFolder: _format == 'mp3'
                                            ? 'Music/YouTube'
                                            : 'Videos/YouTube',
                                      );
                                  _urlCtrl.clear();
                                  await ref.read(galleryControllerProvider.notifier).refresh();
                                } finally {
                                  if (mounted) setState(() => _submitting = false);
                                }
                              },
                        icon: const Icon(Icons.download),
                        label: Text(_submitting ? 'Downloading...' : 'Download'),
                      ),
                    ],
                  ),
                  const SizedBox(height: 6),
                  const Text(
                    'Download only content you have the right to download.',
                    style: TextStyle(fontSize: 12),
                  ),
                ],
              ),
            ),
          ),
          if (ui.downloadJobs.isNotEmpty) ...[
            const SizedBox(height: 12),
            Text('Download Queue (${ui.downloadJobs.length})',
                style: Theme.of(context).textTheme.titleSmall),
            const SizedBox(height: 6),
            for (final job in ui.downloadJobs)
              Card(
                child: ListTile(
                  leading: Icon(job.format == 'mp3' ? Icons.music_note : Icons.movie),
                  title: Text(job.title ?? job.url, overflow: TextOverflow.ellipsis),
                  subtitle: Text('${job.format.toUpperCase()} · ${job.quality} · ${job.status}'),
                  trailing: job.status == 'completed'
                      ? const Icon(Icons.check_circle, color: Colors.green)
                      : Text('${(job.progress * 100).round()}%'),
                ),
              ),
          ],
        ],
      ),
    );
  }
}

class SettingsView extends ConsumerStatefulWidget {
  const SettingsView({super.key});

  @override
  ConsumerState<SettingsView> createState() => _SettingsViewState();
}

class _SettingsViewState extends ConsumerState<SettingsView> {
  final TextEditingController _aliasTplCtrl = TextEditingController();
  bool _syncedInit = false;

  @override
  void dispose() {
    _aliasTplCtrl.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final ui = ref.watch(galleryControllerProvider);
    final ctrl = ref.read(galleryControllerProvider.notifier);
    if (!_syncedInit) {
      _aliasTplCtrl.text = ui.settings.aliasTemplate;
      _syncedInit = true;
    }

    return Padding(
      padding: const EdgeInsets.all(14),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Card(
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text('Library Folders & Exclusions',
                      style: Theme.of(context).textTheme.titleMedium),
                  const SizedBox(height: 8),
                  Wrap(
                    spacing: 8,
                    runSpacing: 8,
                    children: [
                      FilledButton.tonalIcon(
                        onPressed: () => pickAndScanFolder(context, ref),
                        icon: const Icon(Icons.create_new_folder),
                        label: const Text('Add Scan Folder'),
                      ),
                      OutlinedButton.icon(
                        onPressed: () async {
                          final dir = await FilePicker.platform
                              .getDirectoryPath(dialogTitle: 'Select folder to exclude');
                          if (dir != null && dir.isNotEmpty) {
                            await ctrl.updateSettings(
                              ui.settings.copyWith(
                                excludedFolders: {...ui.settings.excludedFolders, dir}.toList(),
                              ),
                            );
                          }
                        },
                        icon: const Icon(Icons.block),
                        label: const Text('Exclude Folder'),
                      ),
                    ],
                  ),
                  if (ui.settings.scanFolders.isNotEmpty) ...[
                    const SizedBox(height: 10),
                    const Text('Included Folders:', style: TextStyle(fontWeight: FontWeight.w600)),
                    Wrap(
                      spacing: 6,
                      children: [
                        for (final f in ui.settings.scanFolders)
                          InputChip(
                            label: Text(f),
                            onDeleted: () => ctrl.updateSettings(
                              ui.settings.copyWith(
                                scanFolders:
                                    ui.settings.scanFolders.where((e) => e != f).toList(),
                              ),
                            ),
                          ),
                      ],
                    ),
                  ],
                  if (ui.settings.excludedFolders.isNotEmpty) ...[
                    const SizedBox(height: 10),
                    const Text('Excluded Folders:', style: TextStyle(fontWeight: FontWeight.w600)),
                    Wrap(
                      spacing: 6,
                      children: [
                        for (final f in ui.settings.excludedFolders)
                          InputChip(
                            label: Text(f),
                            onDeleted: () => ctrl.updateSettings(
                              ui.settings.copyWith(
                                excludedFolders:
                                    ui.settings.excludedFolders.where((e) => e != f).toList(),
                              ),
                            ),
                          ),
                      ],
                    ),
                  ],
                ],
              ),
            ),
          ),
          const SizedBox(height: 12),
          Card(
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text('Alias, Theme & Network Transport',
                      style: Theme.of(context).textTheme.titleMedium),
                  const SizedBox(height: 10),
                  Row(
                    children: [
                      Expanded(
                        child: TextField(
                          controller: _aliasTplCtrl,
                          decoration: const InputDecoration(
                            labelText: 'Default Auto-Alias Template',
                            border: OutlineInputBorder(),
                            isDense: true,
                          ),
                        ),
                      ),
                      const SizedBox(width: 8),
                      FilledButton(
                        onPressed: () => ctrl.updateSettings(
                          ui.settings.copyWith(aliasTemplate: _aliasTplCtrl.text.trim()),
                        ),
                        child: const Text('Save'),
                      ),
                    ],
                  ),
                  const SizedBox(height: 10),
                  SwitchListTile(
                    title: const Text('Dark Theme'),
                    value: ui.settings.darkMode,
                    onChanged: (_) => ctrl.toggleTheme(),
                  ),
                  DropdownButtonFormField<TransportKind?>(
                    initialValue: ui.settings.manualTransportOverride,
                    decoration: const InputDecoration(
                      labelText: 'Transfer Path Preference',
                      border: OutlineInputBorder(),
                      isDense: true,
                    ),
                    items: [
                      const DropdownMenuItem<TransportKind?>(
                        value: null,
                        child: Text('Auto (LAN / Wired > Hole-Punch > Relay)'),
                      ),
                      for (final kind in TransportKind.values)
                        DropdownMenuItem<TransportKind?>(
                          value: kind,
                          child: Text(kind.name),
                        ),
                    ],
                    onChanged: (val) => ctrl.updateSettings(
                      val == null
                          ? ui.settings.copyWith(clearTransportOverride: true)
                          : ui.settings.copyWith(manualTransportOverride: val),
                    ),
                  ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class MiniPlayerBar extends ConsumerWidget {
  const MiniPlayerBar({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final ui = ref.watch(galleryControllerProvider);
    final ctrl = ref.read(galleryControllerProvider.notifier);
    final track = ui.playingTrack;
    if (track == null) return const SizedBox.shrink();

    final playback = MediaPlaybackService.instance;

    return Container(
      height: 68,
      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 4),
      decoration: BoxDecoration(
        color: Theme.of(context).colorScheme.secondaryContainer,
        border: Border(top: BorderSide(color: Theme.of(context).dividerColor)),
      ),
      child: Row(
        children: [
          SizedBox(
            width: 42,
            height: 42,
            child: MediaThumbnailWidget(item: track, cacheWidth: 120, borderRadius: 6),
          ),
          const SizedBox(width: 10),
          Expanded(
            child: Column(
              mainAxisAlignment: MainAxisAlignment.center,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  track.alias,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: const TextStyle(fontWeight: FontWeight.w600, fontSize: 13),
                ),
                ValueListenableBuilder<Duration>(
                  valueListenable: playback.position,
                  builder: (_, pos, __) {
                    final totalMs = playback.duration.value.inMilliseconds > 0
                        ? playback.duration.value.inMilliseconds
                        : track.durationMs;
                    final curMs = pos.inMilliseconds.clamp(0, totalMs > 0 ? totalMs : 1);
                    return Row(
                      children: [
                        Text(
                          formatDuration(curMs).isEmpty ? '0:00' : formatDuration(curMs),
                          style: const TextStyle(fontSize: 10),
                        ),
                        Expanded(
                          child: SliderTheme(
                            data: SliderTheme.of(context).copyWith(
                              trackHeight: 2,
                              thumbShape: const RoundSliderThumbShape(enabledThumbRadius: 5),
                            ),
                            child: Slider(
                              value: curMs.toDouble(),
                              max: (totalMs > 0 ? totalMs : 1).toDouble(),
                              onChanged: (v) =>
                                  playback.seek(Duration(milliseconds: v.round())),
                            ),
                          ),
                        ),
                        Text(
                          formatDuration(totalMs),
                          style: const TextStyle(fontSize: 10),
                        ),
                      ],
                    );
                  },
                ),
              ],
            ),
          ),
          IconButton(
            tooltip: 'Shuffle',
            icon: Icon(
              Icons.shuffle,
              size: 18,
              color: ui.shuffle ? Theme.of(context).colorScheme.primary : null,
            ),
            onPressed: ctrl.toggleShuffle,
          ),
          IconButton(
            tooltip: 'Previous',
            icon: const Icon(Icons.skip_previous),
            onPressed: () => ctrl.skipTrack(-1),
          ),
          IconButton(
            tooltip: ui.isPlaying ? 'Pause' : 'Play',
            icon: Icon(
              ui.isPlaying ? Icons.pause_circle_filled : Icons.play_circle_filled,
              size: 32,
            ),
            onPressed: ctrl.togglePlayback,
          ),
          IconButton(
            tooltip: 'Next',
            icon: const Icon(Icons.skip_next),
            onPressed: () => ctrl.skipTrack(1),
          ),
          IconButton(
            tooltip: 'Repeat one',
            icon: Icon(
              Icons.repeat_one,
              size: 18,
              color: ui.repeatOne ? Theme.of(context).colorScheme.primary : null,
            ),
            onPressed: ctrl.toggleRepeat,
          ),
          IconButton(
            tooltip: 'Close player',
            icon: const Icon(Icons.close, size: 18),
            onPressed: ctrl.stopPlayback,
          ),
        ],
      ),
    );
  }
}

Future<void> pickAndScanFolder(BuildContext context, WidgetRef ref) async {
  final dir = await FilePicker.platform.getDirectoryPath(dialogTitle: 'Select media folder to scan');
  if (dir != null && dir.isNotEmpty) {
    await ref.read(galleryControllerProvider.notifier).scanFolderPaths([dir]);
  }
}

Future<void> pickCustomCoverForMedia(
  BuildContext context,
  WidgetRef ref,
  MediaItem item,
) async {
  final res = await FilePicker.platform.pickFiles(
    type: FileType.image,
    dialogTitle: 'Select custom cover image',
  );
  final path = res?.files.singleOrNull?.path;
  if (path != null && path.isNotEmpty) {
    await ref.read(galleryControllerProvider.notifier).setCustomCover(
          item.id,
          path,
          item.cover?.aspect ?? AspectPreset.ratio16x9,
        );
  }
}
