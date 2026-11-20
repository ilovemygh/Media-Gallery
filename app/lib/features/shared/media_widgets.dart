import 'dart:io';
import 'dart:math' as math;
import 'package:file_picker/file_picker.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_ui/material_ui.dart';
import 'package:media_kit_video/media_kit_video.dart';
import 'package:qr_flutter/qr_flutter.dart';
import '../../core/models/media_models.dart';
import '../../core/services/media_playback_service.dart';
import '../../core/state/gallery_providers.dart';

/// Vector brand logo for Media Gallery v0.2.1 matching the native Windows/Android squircle icon.
class MediaGalleryLogo extends StatelessWidget {
  const MediaGalleryLogo({super.key, this.size = 28});

  final double size;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: size,
      height: size,
      child: CustomPaint(painter: _MediaGalleryLogoPainter()),
    );
  }
}

class _MediaGalleryLogoPainter extends CustomPainter {
  @override
  void paint(Canvas canvas, Size size) {
    final s = size.shortestSide;
    final rect = Rect.fromLTWH(0, 0, s, s);
    final rrect = RRect.fromRectAndRadius(rect, Radius.circular(s * 0.24));

    final bgPaint = Paint()
      ..shader = const LinearGradient(
        begin: Alignment.topLeft,
        end: Alignment.bottomRight,
        colors: [
          Color(0xFF17154E),
          Color(0xFF4F46E5),
          Color(0xFF06B6D4),
        ],
        stops: [0.0, 0.52, 1.0],
      ).createShader(rect);
    canvas.drawRRect(rrect, bgPaint);

    // Tilted translucent back gallery card
    canvas.save();
    canvas.translate(s * 0.5, s * 0.5);
    canvas.rotate(-0.17);
    final backRect = RRect.fromRectAndRadius(
      Rect.fromCenter(center: Offset.zero, width: s * 0.58, height: s * 0.46),
      Radius.circular(s * 0.09),
    );
    canvas.drawRRect(backRect, Paint()..color = Colors.white.withValues(alpha: 0.28));
    canvas.restore();

    // Front frosted media frame
    final frontRect = RRect.fromRectAndRadius(
      Rect.fromLTWH(s * 0.19, s * 0.23, s * 0.62, s * 0.52),
      Radius.circular(s * 0.10),
    );
    canvas.drawRRect(frontRect, Paint()..color = Colors.white.withValues(alpha: 0.95));

    // Inner dark-indigo viewport
    final vpRect = Rect.fromLTWH(s * 0.235, s * 0.275, s * 0.53, s * 0.43);
    final vpRRect = RRect.fromRectAndRadius(vpRect, Radius.circular(s * 0.065));
    canvas.drawRRect(
      vpRRect,
      Paint()
        ..shader = const LinearGradient(
          begin: Alignment.topLeft,
          end: Alignment.bottomRight,
          colors: [Color(0xFF18154A), Color(0xFF3730A3)],
        ).createShader(vpRect),
    );

    // Sound-wave spectrum bars along bottom of viewport
    const heights = [0.22, 0.42, 0.58, 0.35, 0.24, 0.40, 0.54, 0.38, 0.24];
    final barSpan = vpRect.width * 0.84;
    final startX = vpRect.left + (vpRect.width - barSpan) / 2;
    final step = barSpan / heights.length;
    final barW = step * 0.55;
    for (var i = 0; i < heights.length; i++) {
      final bh = vpRect.height * heights[i];
      final bx = startX + i * step + (step - barW) / 2;
      final by = vpRect.bottom - bh;
      final t = i / (heights.length - 1);
      final color = Color.lerp(const Color(0xFF22D3EE), const Color(0xFFE879F9), t)!;
      canvas.drawRRect(
        RRect.fromRectAndRadius(
          Rect.fromLTWH(bx, by, barW, bh),
          Radius.circular(barW / 2),
        ),
        Paint()..color = color,
      );
    }

    // Glowing sky lens dot
    canvas.drawCircle(
      Offset(vpRect.left + vpRect.width * 0.16, vpRect.top + vpRect.height * 0.22),
      s * 0.032,
      Paint()..color = const Color(0xFF38BDF8),
    );

    // Centered crisp white play triangle
    final cx = vpRect.left + vpRect.width * 0.53;
    final cy = vpRect.top + vpRect.height * 0.43;
    final tr = s * 0.10;
    final tri = Path()
      ..moveTo(cx - tr * 0.62, cy - tr * 0.82)
      ..lineTo(cx + tr * 0.95, cy)
      ..lineTo(cx - tr * 0.62, cy + tr * 0.82)
      ..close();
    canvas.drawPath(tri, Paint()..color = Colors.white);
  }

  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) => false;
}

/// Circular profile avatar button with account popover menu (replaces raw "Google" text button).
class ProfileAvatarButton extends ConsumerWidget {
  const ProfileAvatarButton({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final ui = ref.watch(galleryControllerProvider);
    final ctrl = ref.read(galleryControllerProvider.notifier);
    final email = ui.googleEmail;
    final signedIn = email != null && email.isNotEmpty;
    final colors = Theme.of(context).colorScheme;
    final initial = signedIn ? email.trim()[0].toUpperCase() : null;

    return PopupMenuButton<String>(
      key: const Key('profile_avatar_button'),
      tooltip: signedIn ? 'Profile ($email)' : 'Profile',
      onSelected: (action) {
        if (action == 'toggle_auth') {
          ctrl.toggleGoogleSignIn();
        } else if (action == 'settings') {
          ctrl.selectSection(NavSection.settings);
        }
      },
      itemBuilder: (_) => [
        PopupMenuItem<String>(
          enabled: false,
          child: Row(
            children: [
              CircleAvatar(
                radius: 16,
                backgroundColor: signedIn ? colors.primary : colors.surfaceContainerHighest,
                child: signedIn
                    ? Text(
                        initial!,
                        style: TextStyle(
                          color: colors.onPrimary,
                          fontWeight: FontWeight.bold,
                        ),
                      )
                    : Icon(Icons.person, size: 18, color: colors.onSurfaceVariant),
              ),
              const SizedBox(width: 10),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Text(
                      signedIn ? email : 'Local Profile',
                      style: const TextStyle(fontWeight: FontWeight.w600, fontSize: 13),
                      overflow: TextOverflow.ellipsis,
                    ),
                    Text(
                      signedIn ? 'Google Account Linked' : 'Offline-first mode',
                      style: const TextStyle(fontSize: 11),
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
        const PopupMenuDivider(),
        PopupMenuItem<String>(
          value: 'toggle_auth',
          child: Row(
            children: [
              Icon(signedIn ? Icons.logout : Icons.login, size: 18),
              const SizedBox(width: 8),
              Text(signedIn ? 'Sign out' : 'Sign in with Google'),
            ],
          ),
        ),
        const PopupMenuItem<String>(
          value: 'settings',
          child: Row(
            children: [
              Icon(Icons.settings_outlined, size: 18),
              SizedBox(width: 8),
              Text('Profile & App Settings'),
            ],
          ),
        ),
      ],
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 4),
        child: Stack(
          clipBehavior: Clip.none,
          children: [
            CircleAvatar(
              radius: 16,
              backgroundColor: signedIn ? colors.primary : colors.surfaceContainerHighest,
              child: signedIn
                  ? Text(
                      initial!,
                      style: TextStyle(
                        color: colors.onPrimary,
                        fontWeight: FontWeight.bold,
                        fontSize: 13,
                      ),
                    )
                  : Icon(Icons.person_rounded, size: 19, color: colors.onSurfaceVariant),
            ),
            if (signedIn)
              Positioned(
                right: -1,
                bottom: -1,
                child: Container(
                  width: 10,
                  height: 10,
                  decoration: BoxDecoration(
                    color: Colors.greenAccent.shade700,
                    shape: BoxShape.circle,
                    border: Border.all(color: colors.surface, width: 1.5),
                  ),
                ),
              ),
          ],
        ),
      ),
    );
  }
}

/// Compact dropdown selector for grid size and detail list view.
class GridSizeViewDropdown extends StatelessWidget {
  const GridSizeViewDropdown({
    super.key,
    required this.value,
    required this.onChanged,
  });

  final GridDensity value;
  final ValueChanged<GridDensity> onChanged;

  String _label(GridDensity d) {
    switch (d) {
      case GridDensity.small:
        return 'Size: Small Grid';
      case GridDensity.medium:
        return 'Size: Medium Grid';
      case GridDensity.large:
        return 'Size: Large Grid';
      case GridDensity.list:
        return 'View: Details List';
    }
  }

  IconData _icon(GridDensity d) {
    switch (d) {
      case GridDensity.small:
        return Icons.grid_on;
      case GridDensity.medium:
        return Icons.grid_view;
      case GridDensity.large:
        return Icons.crop_square;
      case GridDensity.list:
        return Icons.view_list;
    }
  }

  @override
  Widget build(BuildContext context) {
    return DropdownButton<GridDensity>(
      key: const Key('grid_size_dropdown'),
      value: value,
      isDense: true,
      underline: const SizedBox.shrink(),
      items: GridDensity.values
          .map(
            (d) => DropdownMenuItem<GridDensity>(
              value: d,
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Icon(_icon(d), size: 16),
                  const SizedBox(width: 6),
                  Text(_label(d)),
                ],
              ),
            ),
          )
          .toList(),
      onChanged: (next) {
        if (next != null) onChanged(next);
      },
    );
  }
}

/// Interactive audio sound-wave visualizer with real file amplitude sampling and seek scrubbing.
class AudioWaveformVisualizer extends StatefulWidget {
  const AudioWaveformVisualizer({
    super.key,
    required this.item,
    required this.isPlaying,
    this.barCount = 56,
    this.height = 110,
  });

  final MediaItem item;
  final bool isPlaying;
  final int barCount;
  final double height;

  @override
  State<AudioWaveformVisualizer> createState() => _AudioWaveformVisualizerState();
}

class _AudioWaveformVisualizerState extends State<AudioWaveformVisualizer> {
  List<double> _peaks = const [];

  @override
  void initState() {
    super.initState();
    _peaks = _computePeaks();
  }

  @override
  void didUpdateWidget(covariant AudioWaveformVisualizer oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.item.id != widget.item.id || oldWidget.barCount != widget.barCount) {
      _peaks = _computePeaks();
    }
  }

  List<double> _computePeaks() {
    final count = widget.barCount.clamp(16, 120);
    // Sample real audio bytes from disk when available so each file has an authentic waveform contour.
    try {
      if (widget.item.path.isNotEmpty && !widget.item.path.startsWith('content://')) {
        final file = File(widget.item.path);
        if (file.existsSync()) {
          final len = file.lengthSync();
          if (len > 2048) {
            final raf = file.openSync(mode: FileMode.read);
            try {
              final headerSkip = math.min(1024, len ~/ 10);
              final usable = len - headerSkip;
              final step = math.max(1, usable ~/ count);
              final out = <double>[];
              final buf = List<int>.filled(256, 0);
              for (var i = 0; i < count; i++) {
                final pos = headerSkip + (i * step);
                raf.setPositionSync(pos.clamp(0, len - 1));
                final n = raf.readIntoSync(buf, 0, math.min(buf.length, len - pos));
                if (n <= 0) {
                  out.add(0.35);
                  continue;
                }
                var sum = 0.0;
                for (var j = 0; j < n; j++) {
                  sum += (buf[j] - 128).abs() / 128.0;
                }
                final avg = (sum / n).clamp(0.08, 1.0);
                final shaped = (0.18 + 0.82 * math.pow(avg, 0.75)).clamp(0.14, 1.0);
                out.add(shaped);
              }
              return out;
            } finally {
              raf.closeSync();
            }
          }
        }
      }
    } catch (_) {
      // Fallback to deterministic musical contour below.
    }

    final seed = '${widget.item.id}:${widget.item.alias}:${widget.item.size}'.codeUnits;
    var acc = 2166136261;
    for (final c in seed) {
      acc = ((acc ^ c) * 16777619) & 0x7fffffff;
    }
    return List<double>.generate(count, (i) {
      final phase1 = math.sin(i * 0.29 + (acc % 97) * 0.07).abs();
      final phase2 = math.cos(i * 0.13 + (acc % 53) * 0.11).abs();
      final envelope = math.sin((i / count) * math.pi).clamp(0.25, 1.0);
      return (0.16 + 0.82 * (0.55 * phase1 + 0.45 * phase2) * envelope).clamp(0.14, 1.0);
    });
  }

  void _seekToLocalDx(double dx, double width, int totalMs) {
    if (width <= 0 || totalMs <= 0) return;
    final ratio = (dx / width).clamp(0.0, 1.0);
    MediaPlaybackService.instance.seek(Duration(milliseconds: (totalMs * ratio).round()));
  }

  @override
  Widget build(BuildContext context) {
    final playback = MediaPlaybackService.instance;
    final colors = Theme.of(context).colorScheme;

    return ValueListenableBuilder<Duration>(
      valueListenable: playback.position,
      builder: (context, pos, _) {
        final totalMs = playback.duration.value.inMilliseconds > 0
            ? playback.duration.value.inMilliseconds
            : (widget.item.durationMs > 0 ? widget.item.durationMs : 180000);
        final curMs = pos.inMilliseconds.clamp(0, totalMs);
        final progress = totalMs > 0 ? (curMs / totalMs).clamp(0.0, 1.0) : 0.0;
        final phase = curMs / 160.0;

        return LayoutBuilder(
          builder: (context, constraints) {
            final width = constraints.maxWidth > 0 ? constraints.maxWidth : 360.0;
            return GestureDetector(
              key: const Key('audio_waveform_visualizer'),
              behavior: HitTestBehavior.opaque,
              onTapDown: (d) => _seekToLocalDx(d.localPosition.dx, width, totalMs),
              onHorizontalDragUpdate: (d) => _seekToLocalDx(d.localPosition.dx, width, totalMs),
              child: SizedBox(
                width: double.infinity,
                height: widget.height,
                child: CustomPaint(
                  painter: _AudioWaveformPainter(
                    peaks: _peaks,
                    progress: progress,
                    isPlaying: widget.isPlaying,
                    phase: phase,
                    playedColor: colors.primary,
                    playedAccent: Colors.cyanAccent,
                    unplayedColor: Colors.white.withValues(alpha: 0.24),
                  ),
                ),
              ),
            );
          },
        );
      },
    );
  }
}

class _AudioWaveformPainter extends CustomPainter {
  _AudioWaveformPainter({
    required this.peaks,
    required this.progress,
    required this.isPlaying,
    required this.phase,
    required this.playedColor,
    required this.playedAccent,
    required this.unplayedColor,
  });

  final List<double> peaks;
  final double progress;
  final bool isPlaying;
  final double phase;
  final Color playedColor;
  final Color playedAccent;
  final Color unplayedColor;

  @override
  void paint(Canvas canvas, Size size) {
    if (peaks.isEmpty) return;
    final n = peaks.length;
    final step = size.width / n;
    final barW = (step * 0.62).clamp(2.0, 10.0);
    final midY = size.height / 2;

    for (var i = 0; i < n; i++) {
      final ratio = i / n;
      var amp = peaks[i];
      if (isPlaying) {
        final distToHead = (ratio - progress).abs();
        final boost = distToHead < 0.12
            ? 0.22 * math.sin(phase + i * 0.75)
            : 0.08 * math.sin(phase * 0.6 + i * 0.45);
        amp = (amp + boost).clamp(0.12, 1.0);
      }
      final barH = (amp * size.height * 0.88).clamp(6.0, size.height);
      final x = i * step + (step - barW) / 2;
      final rect = RRect.fromRectAndRadius(
        Rect.fromCenter(center: Offset(x + barW / 2, midY), width: barW, height: barH),
        Radius.circular(barW / 2),
      );
      final isPlayed = ratio <= progress;
      final paint = Paint()
        ..color = isPlayed
            ? Color.lerp(playedAccent, playedColor, ratio.clamp(0.0, 1.0))!
            : unplayedColor;
      canvas.drawRRect(rect, paint);
    }

    // Playhead indicator line
    final headX = (progress * size.width).clamp(1.0, size.width - 1.0);
    canvas.drawLine(
      Offset(headX, 4),
      Offset(headX, size.height - 4),
      Paint()
        ..color = Colors.white.withValues(alpha: 0.85)
        ..strokeWidth = 2,
    );
  }

  @override
  bool shouldRepaint(covariant _AudioWaveformPainter old) =>
      old.progress != progress ||
      old.isPlaying != isPlaying ||
      old.phase != phase ||
      old.peaks != peaks;
}

/// Compact mini sound-wave bars shown inside MiniPlayerBar.
class MiniSoundWaveIndicator extends StatelessWidget {
  const MiniSoundWaveIndicator({
    super.key,
    required this.isPlaying,
    required this.seed,
  });

  final bool isPlaying;
  final String seed;

  @override
  Widget build(BuildContext context) {
    final playback = MediaPlaybackService.instance;
    final color = Theme.of(context).colorScheme.primary;
    return ValueListenableBuilder<Duration>(
      valueListenable: playback.position,
      builder: (_, pos, __) {
        final t = pos.inMilliseconds / 180.0;
        const base = [0.45, 0.85, 0.55, 0.95, 0.60];
        return SizedBox(
          width: 22,
          height: 18,
          child: Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            crossAxisAlignment: CrossAxisAlignment.end,
            children: List.generate(base.length, (i) {
              final wave = isPlaying ? 0.25 * math.sin(t + i * 1.1) : 0.0;
              final h = ((base[i] + wave).clamp(0.22, 1.0)) * 16.0;
              return Container(
                width: 3,
                height: h,
                decoration: BoxDecoration(
                  color: color,
                  borderRadius: BorderRadius.circular(2),
                ),
              );
            }),
          ),
        );
      },
    );
  }
}

/// Opens a completed YouTube download job directly in the built-in viewer/player.
Future<void> openDownloadedJob(
  BuildContext context,
  WidgetRef ref,
  DownloadJobItem job,
) async {
  final ctrl = ref.read(galleryControllerProvider.notifier);
  final media = await ctrl.resolveMediaForDownloadJob(job);
  if (!context.mounted) return;
  if (media != null) {
    ctrl.selectMedia(media.id);
    await showMediaViewerDialog(context, ref, media);
  } else {
    await ctrl.openFolder(job.destinationFolder);
  }
}

/// Bottom notification strip for newly completed YouTube downloads, clickable to open immediately.
class CompletedDownloadBar extends ConsumerWidget {
  const CompletedDownloadBar({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final job = ref.watch(galleryControllerProvider.select((s) => s.lastCompletedDownload));
    if (job == null) return const SizedBox.shrink();
    final ctrl = ref.read(galleryControllerProvider.notifier);
    final colors = Theme.of(context).colorScheme;

    return Material(
      color: colors.tertiaryContainer,
      child: InkWell(
        key: const Key('completed_download_bar'),
        onTap: () => openDownloadedJob(context, ref, job),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 6),
          child: Row(
            children: [
              Icon(
                job.format == 'mp3' ? Icons.music_note : Icons.play_circle_fill,
                size: 20,
                color: colors.onTertiaryContainer,
              ),
              const SizedBox(width: 10),
              Expanded(
                child: Text(
                  'Downloaded: ${job.title ?? job.url} (${job.format.toUpperCase()} · ${job.quality})',
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: TextStyle(
                    fontWeight: FontWeight.w600,
                    fontSize: 13,
                    color: colors.onTertiaryContainer,
                  ),
                ),
              ),
              FilledButton.tonalIcon(
                onPressed: () => openDownloadedJob(context, ref, job),
                icon: const Icon(Icons.open_in_new, size: 16),
                label: const Text('Open'),
              ),
              const SizedBox(width: 4),
              IconButton(
                tooltip: 'Dismiss download notice',
                icon: const Icon(Icons.close, size: 18),
                onPressed: ctrl.clearCompletedDownload,
              ),
            ],
          ),
        ),
      ),
    );
  }
}

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
                    : item.type == MediaType.audio
                        ? Padding(
                            padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 16),
                            child: Column(
                              mainAxisAlignment: MainAxisAlignment.center,
                              children: [
                                if (diskImage != null)
                                  ClipRRect(
                                    borderRadius: BorderRadius.circular(14),
                                    child: Image.file(
                                      File(diskImage),
                                      width: 140,
                                      height: 140,
                                      fit: BoxFit.cover,
                                    ),
                                  )
                                else
                                  const Icon(Icons.graphic_eq, size: 72, color: Colors.cyanAccent),
                                const SizedBox(height: 12),
                                Text(
                                  item.alias,
                                  style: const TextStyle(
                                    color: Colors.white,
                                    fontSize: 18,
                                    fontWeight: FontWeight.bold,
                                  ),
                                  textAlign: TextAlign.center,
                                  maxLines: 1,
                                  overflow: TextOverflow.ellipsis,
                                ),
                                Text(
                                  '${item.artist ?? "Unknown Artist"} · ${item.album ?? item.fileName}',
                                  style: const TextStyle(color: Colors.white70, fontSize: 13),
                                  textAlign: TextAlign.center,
                                  maxLines: 1,
                                  overflow: TextOverflow.ellipsis,
                                ),
                                const SizedBox(height: 18),
                                AudioWaveformVisualizer(
                                  item: item,
                                  isPlaying: ui.isPlaying && ui.playingTrack?.id == item.id,
                                  height: 112,
                                ),
                                const SizedBox(height: 12),
                                Wrap(
                                  spacing: 12,
                                  crossAxisAlignment: WrapCrossAlignment.center,
                                  children: [
                                    IconButton(
                                      icon: const Icon(Icons.skip_previous, color: Colors.white),
                                      onPressed: () => _step(-1, ui.items),
                                    ),
                                    FilledButton.icon(
                                      onPressed: () => ui.playingTrack?.id == item.id
                                          ? ctrl.togglePlayback()
                                          : ctrl.playTrack(item),
                                      icon: Icon(
                                        ui.isPlaying && ui.playingTrack?.id == item.id
                                            ? Icons.pause
                                            : Icons.play_arrow,
                                      ),
                                      label: Text(
                                        ui.isPlaying && ui.playingTrack?.id == item.id
                                            ? 'Pause'
                                            : 'Play',
                                      ),
                                    ),
                                    IconButton(
                                      icon: const Icon(Icons.skip_next, color: Colors.white),
                                      onPressed: () => _step(1, ui.items),
                                    ),
                                  ],
                                ),
                              ],
                            ),
                          )
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
                                      item.type == MediaType.video ? Icons.movie : Icons.image,
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
  bool _initializedDefaults = false;

  @override
  void dispose() {
    _urlCtrl.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final ui = ref.watch(galleryControllerProvider);
    final ctrl = ref.read(galleryControllerProvider.notifier);
    if (!_initializedDefaults) {
      _initializedDefaults = true;
      _format = ui.settings.youtubeDefaultFormat == 'mp3' ? 'mp3' : 'mp4';
      final validQualities = _format == 'mp4'
          ? const ['1080p', '720p', '480p']
          : const ['320kbps', '192kbps'];
      _quality = validQualities.contains(ui.settings.youtubeDefaultQuality)
          ? ui.settings.youtubeDefaultQuality
          : validQualities.first;
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
                                  await ctrl.startYoutubeDownload(
                                    url: _urlCtrl.text.trim(),
                                    format: _format,
                                    quality: _quality,
                                  );
                                  _urlCtrl.clear();
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
                  onTap: job.status == 'completed'
                      ? () => openDownloadedJob(context, ref, job)
                      : null,
                  trailing: job.status == 'completed'
                      ? FilledButton.tonalIcon(
                          onPressed: () => openDownloadedJob(context, ref, job),
                          icon: const Icon(Icons.open_in_new, size: 16),
                          label: const Text('Open'),
                        )
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

  void _toggleExtension(AppSettings settings, GalleryController ctrl, String ext) {
    final current = settings.effectiveExtensions;
    final next = <String>{...current};
    if (next.contains(ext)) {
      if (next.length > 1) next.remove(ext);
    } else {
      next.add(ext);
    }
    final isAll = next.length == AppSettings.allSupportedExtensions.length;
    ctrl.updateSettings(
      settings.copyWith(enabledExtensions: isAll ? const [] : next.toList()),
    );
  }

  @override
  Widget build(BuildContext context) {
    final ui = ref.watch(galleryControllerProvider);
    final ctrl = ref.read(galleryControllerProvider.notifier);
    final s = ui.settings;
    if (!_syncedInit) {
      _aliasTplCtrl.text = s.aliasTemplate;
      _syncedInit = true;
    }
    final activeExts = s.effectiveExtensions;
    final ytQualities = s.youtubeDefaultFormat == 'mp3'
        ? const ['320kbps', '192kbps']
        : const ['1080p', '720p', '480p'];
    final curYtQuality = ytQualities.contains(s.youtubeDefaultQuality)
        ? s.youtubeDefaultQuality
        : ytQualities.first;

    return Padding(
      padding: const EdgeInsets.all(14),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // 1. Startup, Background Auto-Scan & Deduplication
          Card(
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      const MediaGalleryLogo(size: 32),
                      const SizedBox(width: 10),
                      Expanded(
                        child: Text(
                          'Startup & Automatic Background Scan (v0.2.1)',
                          style: Theme.of(context).textTheme.titleMedium,
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 8),
                  SwitchListTile(
                    dense: true,
                    title: const Text('Open at Startup'),
                    subtitle: const Text(
                      'Launch Media Gallery automatically when you sign in to your device',
                    ),
                    value: s.openAtStartup,
                    onChanged: (v) => ctrl.updateSettings(s.copyWith(openAtStartup: v)),
                  ),
                  SwitchListTile(
                    dense: true,
                    title: const Text('Auto-Scan in Background on Open'),
                    subtitle: const Text(
                      'Keep your persisted library loaded and silently index new files in the background',
                    ),
                    value: s.autoScanOnOpen,
                    onChanged: (v) => ctrl.updateSettings(s.copyWith(autoScanOnOpen: v)),
                  ),
                  SwitchListTile(
                    dense: true,
                    title: const Text('Hide Duplicate Files in Gallery Views'),
                    subtitle: const Text(
                      'Show each unique item only once while keeping all copies in Clean & Storage',
                    ),
                    value: s.hideDuplicatesInGallery,
                    onChanged: (v) => ctrl.updateSettings(s.copyWith(hideDuplicatesInGallery: v)),
                  ),
                  SwitchListTile(
                    dense: true,
                    title: const Text('Skip System & OS Cache Folders'),
                    subtitle: const Text(
                      'Prevents crashes on full-drive (C:\\) scans by skipping Windows, Program Files, AppData, and junctions',
                    ),
                    value: s.skipSystemFolders,
                    onChanged: (v) => ctrl.updateSettings(s.copyWith(skipSystemFolders: v)),
                  ),
                ],
              ),
            ),
          ),
          const SizedBox(height: 12),

          // 2. What to Scan (Library Folders, Media Types & Exclusions)
          Card(
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text('Library Folders & Exclusions',
                      style: Theme.of(context).textTheme.titleMedium),
                  const SizedBox(height: 8),
                  const Text('What to Scan (Media Categories):',
                      style: TextStyle(fontWeight: FontWeight.w600, fontSize: 13)),
                  const SizedBox(height: 6),
                  Wrap(
                    spacing: 8,
                    runSpacing: 6,
                    children: [
                      FilterChip(
                        avatar: const Icon(Icons.movie_outlined, size: 16),
                        label: const Text('Scan Videos'),
                        selected: s.scanVideos,
                        onSelected: (v) => ctrl.updateSettings(s.copyWith(scanVideos: v)),
                      ),
                      FilterChip(
                        avatar: const Icon(Icons.image_outlined, size: 16),
                        label: const Text('Scan Images'),
                        selected: s.scanImages,
                        onSelected: (v) => ctrl.updateSettings(s.copyWith(scanImages: v)),
                      ),
                      FilterChip(
                        avatar: const Icon(Icons.music_note_outlined, size: 16),
                        label: const Text('Scan Audio'),
                        selected: s.scanAudio,
                        onSelected: (v) => ctrl.updateSettings(s.copyWith(scanAudio: v)),
                      ),
                    ],
                  ),
                  const SizedBox(height: 12),
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
                              s.copyWith(
                                excludedFolders: {...s.excludedFolders, dir}.toList(),
                              ),
                            );
                          }
                        },
                        icon: const Icon(Icons.block),
                        label: const Text('Exclude Folder'),
                      ),
                      OutlinedButton.icon(
                        onPressed: ui.isScanning ? null : ctrl.runInitialScan,
                        icon: const Icon(Icons.sync),
                        label: const Text('Scan All Now'),
                      ),
                    ],
                  ),
                  if (s.scanFolders.isNotEmpty) ...[
                    const SizedBox(height: 10),
                    const Text('Included Folders:', style: TextStyle(fontWeight: FontWeight.w600)),
                    Wrap(
                      spacing: 6,
                      children: [
                        for (final f in s.scanFolders)
                          InputChip(
                            label: Text(f),
                            onDeleted: () => ctrl.updateSettings(
                              s.copyWith(
                                scanFolders: s.scanFolders.where((e) => e != f).toList(),
                              ),
                            ),
                          ),
                      ],
                    ),
                  ],
                  if (s.excludedFolders.isNotEmpty) ...[
                    const SizedBox(height: 10),
                    const Text('Excluded Folders:', style: TextStyle(fontWeight: FontWeight.w600)),
                    Wrap(
                      spacing: 6,
                      children: [
                        for (final f in s.excludedFolders)
                          InputChip(
                            label: Text(f),
                            onDeleted: () => ctrl.updateSettings(
                              s.copyWith(
                                excludedFolders: s.excludedFolders.where((e) => e != f).toList(),
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

          // 3. Formats & YouTube Defaults
          Card(
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    mainAxisAlignment: MainAxisAlignment.spaceBetween,
                    children: [
                      Text('Media Formats & YouTube Defaults',
                          style: Theme.of(context).textTheme.titleMedium),
                      if (s.enabledExtensions.isNotEmpty)
                        TextButton(
                          onPressed: () =>
                              ctrl.updateSettings(s.copyWith(enabledExtensions: const [])),
                          child: const Text('Enable All Formats'),
                        ),
                    ],
                  ),
                  const SizedBox(height: 6),
                  const Text(
                    'Active File Formats to Index:',
                    style: TextStyle(fontWeight: FontWeight.w600, fontSize: 13),
                  ),
                  const SizedBox(height: 6),
                  Wrap(
                    spacing: 6,
                    runSpacing: 6,
                    children: [
                      for (final ext in AppSettings.allSupportedExtensions)
                        FilterChip(
                          label: Text('.${ext.toUpperCase()}'),
                          selected: activeExts.contains(ext),
                          onSelected: (_) => _toggleExtension(s, ctrl, ext),
                        ),
                    ],
                  ),
                  const Divider(height: 22),
                  Wrap(
                    spacing: 12,
                    runSpacing: 10,
                    crossAxisAlignment: WrapCrossAlignment.center,
                    children: [
                      const Text(
                        'Default YouTube Format:',
                        style: TextStyle(fontWeight: FontWeight.w600),
                      ),
                      DropdownButton<String>(
                        value: s.youtubeDefaultFormat == 'mp3' ? 'mp3' : 'mp4',
                        items: const [
                          DropdownMenuItem(value: 'mp4', child: Text('MP4 (Video)')),
                          DropdownMenuItem(value: 'mp3', child: Text('MP3 (Audio)')),
                        ],
                        onChanged: (fmt) {
                          if (fmt == null) return;
                          ctrl.updateSettings(
                            s.copyWith(
                              youtubeDefaultFormat: fmt,
                              youtubeDefaultQuality: fmt == 'mp3' ? '320kbps' : '1080p',
                            ),
                          );
                        },
                      ),
                      const SizedBox(width: 8),
                      const Text(
                        'Default Quality:',
                        style: TextStyle(fontWeight: FontWeight.w600),
                      ),
                      DropdownButton<String>(
                        value: curYtQuality,
                        items: ytQualities
                            .map((q) => DropdownMenuItem(value: q, child: Text(q)))
                            .toList(),
                        onChanged: (q) {
                          if (q != null) {
                            ctrl.updateSettings(s.copyWith(youtubeDefaultQuality: q));
                          }
                        },
                      ),
                    ],
                  ),
                ],
              ),
            ),
          ),
          const SizedBox(height: 12),

          // 4. Alias, Theme & Network Transport
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
                          s.copyWith(aliasTemplate: _aliasTplCtrl.text.trim()),
                        ),
                        child: const Text('Save'),
                      ),
                    ],
                  ),
                  const SizedBox(height: 10),
                  SwitchListTile(
                    title: const Text('Dark Theme'),
                    value: s.darkMode,
                    onChanged: (_) => ctrl.toggleTheme(),
                  ),
                  DropdownButtonFormField<TransportKind?>(
                    initialValue: s.manualTransportOverride,
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
                          ? s.copyWith(clearTransportOverride: true)
                          : s.copyWith(manualTransportOverride: val),
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
      height: 74,
      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 6),
      decoration: BoxDecoration(
        color: Theme.of(context).colorScheme.secondaryContainer,
        border: Border(top: BorderSide(color: Theme.of(context).dividerColor)),
      ),
      child: Row(
        children: [
          InkWell(
            onTap: () => showMediaViewerDialog(context, ref, track),
            child: SizedBox(
              width: 42,
              height: 42,
              child: MediaThumbnailWidget(item: track, cacheWidth: 120, borderRadius: 6),
            ),
          ),
          const SizedBox(width: 8),
          MiniSoundWaveIndicator(isPlaying: ui.isPlaying, seed: track.id),
          const SizedBox(width: 8),
          Expanded(
            child: Column(
              mainAxisAlignment: MainAxisAlignment.center,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                InkWell(
                  onTap: () => showMediaViewerDialog(context, ref, track),
                  child: Text(
                    track.alias,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: const TextStyle(fontWeight: FontWeight.w600, fontSize: 13),
                  ),
                ),
                const SizedBox(height: 2),
                ValueListenableBuilder<Duration>(
                  valueListenable: playback.position,
                  builder: (_, pos, __) {
                    final totalMs = playback.duration.value.inMilliseconds > 0
                        ? playback.duration.value.inMilliseconds
                        : track.durationMs;
                    final curMs = pos.inMilliseconds.clamp(0, totalMs > 0 ? totalMs : 1);
                    return SizedBox(
                      height: 24,
                      child: Row(
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
                                overlayShape: const RoundSliderOverlayShape(overlayRadius: 10),
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
                      ),
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
