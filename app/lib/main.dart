import 'dart:io';
import 'package:material_ui/material_ui.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'core/bridge/core_bridge.dart';
import 'core/bridge/live_core_bridge.dart';
import 'core/bridge/mock_core_bridge.dart';
import 'core/services/media_playback_service.dart';
import 'core/state/gallery_providers.dart';
import 'features/android/android_shell.dart';
import 'features/migration/import_wizard_dialog.dart';
import 'features/shell/desktop_shell.dart';

const _galleryAccent = Color(0xFF456A7A);

ColorScheme _galleryColorScheme(bool darkMode) {
  final isDark = darkMode;
  return ColorScheme.fromSeed(
    seedColor: _galleryAccent,
    brightness: isDark ? Brightness.dark : Brightness.light,
  ).copyWith(
    surface: isDark ? const Color(0xFF17191B) : const Color(0xFFFCFAF7),
    surfaceContainerLowest:
        isDark ? const Color(0xFF111315) : const Color(0xFFFFFDFC),
    surfaceContainerLow:
        isDark ? const Color(0xFF1E2123) : const Color(0xFFF4F2EE),
    surfaceContainer:
        isDark ? const Color(0xFF25292B) : const Color(0xFFEFEEE9),
    surfaceContainerHigh:
        isDark ? const Color(0xFF2A2E31) : const Color(0xFFE9E7E2),
    surfaceContainerHighest:
        isDark ? const Color(0xFF303538) : const Color(0xFFE3E0DA),
    outlineVariant: isDark ? const Color(0xFF42484B) : const Color(0xFFDEDAD3),
  );
}

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  MediaPlaybackService.instance.ensureInitialized();
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
    final settings =
        ref.watch(galleryControllerProvider.select((s) => s.settings));
    final isMobile = Platform.isAndroid;
    final colors = _galleryColorScheme(settings.darkMode);

    return MaterialApp(
      title: 'Media Gallery',
      debugShowCheckedModeBanner: false,
      theme: ThemeData(
        colorScheme: colors,
        scaffoldBackgroundColor: colors.surface,
        useMaterial3: true,
        appBarTheme: AppBarTheme(
          backgroundColor: colors.surface,
          surfaceTintColor: Colors.transparent,
          elevation: 0,
          scrolledUnderElevation: 0,
        ),
        dividerTheme: DividerThemeData(color: colors.outlineVariant),
        inputDecorationTheme: InputDecorationTheme(
          filled: true,
          fillColor: colors.surfaceContainerLowest,
          border: OutlineInputBorder(
            borderRadius: BorderRadius.circular(10),
            borderSide: BorderSide(color: colors.outlineVariant),
          ),
          enabledBorder: OutlineInputBorder(
            borderRadius: BorderRadius.circular(10),
            borderSide: BorderSide(color: colors.outlineVariant),
          ),
          focusedBorder: OutlineInputBorder(
            borderRadius: BorderRadius.circular(10),
            borderSide: BorderSide(color: colors.primary, width: 1.5),
          ),
        ),
        navigationBarTheme: NavigationBarThemeData(
          backgroundColor: colors.surface,
          surfaceTintColor: Colors.transparent,
          indicatorColor: colors.secondaryContainer,
          elevation: 0,
        ),
      ),
      home: isMobile ? const AndroidShell() : const DesktopShell(),
    );
  }
}
