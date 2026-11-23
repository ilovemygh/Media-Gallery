import 'dart:io';
import 'package:flutter/services.dart';

const MethodChannel _androidOsChannel = MethodChannel(
  'io.mediagallery.app/mediastore',
);

/// Opens an external HTTP(S) URL in the OS default browser.
Future<bool> openExternalUrl(String url) async {
  try {
    if (Platform.isWindows) {
      await Process.start('cmd', ['/c', 'start', '', url], runInShell: false);
      return true;
    }
    if (Platform.isLinux) {
      await Process.start('xdg-open', [url]);
      return true;
    }
    if (Platform.isMacOS) {
      await Process.start('open', [url]);
      return true;
    }
    if (Platform.isAndroid) {
      await _androidOsChannel.invokeMethod<void>('openUrl', {'url': url});
      return true;
    }
  } catch (_) {
    return false;
  }
  return false;
}

/// Opens the containing folder and highlights the file in the OS file manager when supported.
Future<bool> revealFileInOsManager(String filePath) async {
  if (filePath.isEmpty || filePath.startsWith('content://')) return false;
  try {
    final normalized = Platform.isWindows
        ? filePath.replaceAll('/', '\\')
        : filePath;
    if (Platform.isWindows) {
      await Process.start('explorer.exe', ['/select,$normalized']);
      return true;
    }
    if (Platform.isMacOS) {
      await Process.start('open', ['-R', normalized]);
      return true;
    }
    if (Platform.isLinux) {
      final target = File(normalized).existsSync()
          ? File(normalized).parent.path
          : normalized;
      await Process.start('xdg-open', [target]);
      return true;
    }
  } catch (_) {
    return false;
  }
  return false;
}
