import 'dart:async';
import 'package:flutter/foundation.dart';
import 'package:media_kit/media_kit.dart';
import 'package:media_kit_video/media_kit_video.dart';
import '../models/media_models.dart';

/// Wraps `media_kit` (`libmpv`) for real audio & video playback across Windows and Android,
/// while degrading gracefully in headless widget tests where native `libmpv` is absent.
class MediaPlaybackService {
  MediaPlaybackService._();

  static final MediaPlaybackService instance = MediaPlaybackService._();

  bool _initialized = false;
  Player? _player;
  VideoController? _videoController;

  final ValueNotifier<Duration> position = ValueNotifier(Duration.zero);
  final ValueNotifier<Duration> duration = ValueNotifier(Duration.zero);
  final ValueNotifier<bool> playing = ValueNotifier(false);
  final ValueNotifier<double> volume = ValueNotifier(100.0);

  StreamSubscription<Duration>? _posSub;
  StreamSubscription<Duration>? _durSub;
  StreamSubscription<bool>? _playSub;
  StreamSubscription<bool>? _doneSub;

  VoidCallback? onTrackCompleted;

  bool get isNativeReady => _player != null;
  VideoController? get videoController => _videoController;

  void ensureInitialized() {
    if (_initialized) return;
    _initialized = true;
    try {
      MediaKit.ensureInitialized();
      final p = Player();
      _player = p;
      _videoController = VideoController(p);
      _posSub = p.stream.position.listen((d) => position.value = d);
      _durSub = p.stream.duration.listen((d) => duration.value = d);
      _playSub = p.stream.playing.listen((v) => playing.value = v);
      _doneSub = p.stream.completed.listen((done) {
        if (done) {
          onTrackCompleted?.call();
        }
      });
    } catch (_) {
      // Headless widget tests or missing native libmpv: stay in safe UI-state mode.
      _player = null;
      _videoController = null;
    }
  }

  Future<void> openAndPlay(MediaItem item) async {
    ensureInitialized();
    position.value = Duration.zero;
    duration.value = Duration(milliseconds: item.durationMs);
    playing.value = true;

    final player = _player;
    if (player == null || item.path.isEmpty) return;
    try {
      await player.open(Media(item.path), play: true);
    } catch (_) {
      // Ignore invalid mock paths in non-file scenarios.
    }
  }

  Future<void> togglePlayPause() async {
    final player = _player;
    if (player != null) {
      await player.playOrPause();
    } else {
      playing.value = !playing.value;
    }
  }

  Future<void> seek(Duration target) async {
    position.value = target;
    final player = _player;
    if (player != null) {
      await player.seek(target);
    }
  }

  Future<void> setVolume(double value0To100) async {
    final clamped = value0To100.clamp(0.0, 100.0);
    volume.value = clamped;
    final player = _player;
    if (player != null) {
      await player.setVolume(clamped);
    }
  }

  Future<void> stop() async {
    playing.value = false;
    position.value = Duration.zero;
    final player = _player;
    if (player != null) {
      await player.stop();
    }
  }

  Future<void> dispose() async {
    await _posSub?.cancel();
    await _durSub?.cancel();
    await _playSub?.cancel();
    await _doneSub?.cancel();
    await _player?.dispose();
    _player = null;
    _videoController = null;
  }
}
