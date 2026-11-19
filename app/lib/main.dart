import 'dart:io';
import 'package:material_ui/material_ui.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'core/bridge/core_bridge.dart';
import 'core/bridge/live_core_bridge.dart';
import 'core/bridge/mock_core_bridge.dart';
import 'core/state/gallery_providers.dart';
import 'features/android/android_shell.dart';
import 'features/migration/import_wizard_dialog.dart';
import 'features/shell/desktop_shell.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  CoreBridge bridge;
  try {
    bridge = await LiveCoreBridge.initialize();
  } catch (_) {
    bridge = MockCoreBridge(simulateLegacyPresent: false);
  }
  runApp(
    ProviderScope(
      overrides: [bridgeProvider.overrideWithValue(bridge)],
      child: const MediaGalleryApp(),
    ),
  );
}

class MediaGalleryApp extends ConsumerStatefulWidget {
  const MediaGalleryApp({super.key});

  @override
  ConsumerState<MediaGalleryApp> createState() => _MediaGalleryAppState();
}

class _MediaGalleryAppState extends ConsumerState<MediaGalleryApp> {
  bool _checkedMigration = false;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (!_checkedMigration) {
      _checkedMigration = true;
      WidgetsBinding.instance.addPostFrameCallback((_) async {
        final detected = await ref.read(bridgeProvider).detectLegacyDatabase();
        if (detected && mounted) {
          await showDialog<void>(
            context: context,
            builder: (_) => const ImportWizardDialog(),
          );
        }
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final settings = ref.watch(galleryControllerProvider.select((s) => s.settings));
    final isMobile = Platform.isAndroid;

    return MaterialApp(
      title: 'Media Gallery',
      debugShowCheckedModeBanner: false,
      theme: ThemeData(
        colorSchemeSeed: Colors.blue,
        brightness: settings.darkMode ? Brightness.dark : Brightness.light,
        useMaterial3: true,
      ),
      home: isMobile ? const AndroidShell() : const DesktopShell(),
    );
  }
}
