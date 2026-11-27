import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';
import 'package:media_gallery/core/bridge/mock_core_bridge.dart';
import 'package:media_gallery/core/state/gallery_providers.dart';
import 'package:media_gallery/features/android/android_shell.dart';
import 'package:media_gallery/features/migration/import_wizard_dialog.dart';
import 'package:media_gallery/features/shell/desktop_shell.dart';

void main() {
  testWidgets(
    'DesktopShell collapses left sidebar, searches "2026 beach", and shows details sidebar',
    (WidgetTester tester) async {
      tester.view.physicalSize = const Size(1400, 900);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.resetPhysicalSize);

      await tester.pumpWidget(
        ProviderScope(
          overrides: [bridgeProvider.overrideWithValue(MockCoreBridge())],
          child: const MaterialApp(home: DesktopShell()),
        ),
      );
      await tester.pumpAndSettle();

      // Home shows Recent Media strip initially
      expect(find.text('Recent Media'), findsOneWidget);

      // Left sidebar shows labels initially and collapses on toggle
      expect(find.text('Videos'), findsOneWidget);
      await tester.tap(find.byKey(const Key('sidebar_collapse_toggle')));
      await tester.pumpAndSettle();
      expect(find.text('Videos'), findsNothing);

      // Searching "2026 beach" filters down to Bali & Bondi beach items
      await tester.enterText(
        find.byKey(const Key('top_search_field')),
        '2026 beach',
      );
      await tester.pumpAndSettle();
      expect(find.text('2026-03-14 Kuta Beach Bali'), findsOneWidget);

      // Selecting an item opens the right details sidebar with aspect crop presets
      await tester.tap(find.text('2026-03-14 Kuta Beach Bali'));
      await tester.pumpAndSettle();
      expect(find.text('DETAILS'), findsOneWidget);
      expect(find.text('Crop aspect presets'), findsOneWidget);
    },
  );

  testWidgets(
    'DesktopShell navigates Clean & Storage, Devices QR Pairing, and Bulk Rename',
    (WidgetTester tester) async {
      tester.view.physicalSize = const Size(1400, 900);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.resetPhysicalSize);

      await tester.pumpWidget(
        ProviderScope(
          overrides: [bridgeProvider.overrideWithValue(MockCoreBridge())],
          child: const MaterialApp(home: DesktopShell()),
        ),
      );
      await tester.pumpAndSettle();

      // Open Clean & Storage
      await tester.tap(find.text('Clean & Storage'));
      await tester.pumpAndSettle();
      expect(find.text('Storage & Smart Cleanup'), findsOneWidget);
      expect(find.text('Duplicates'), findsOneWidget);
      expect(find.text('Large Files (>50 MB)'), findsOneWidget);
      expect(find.text('Recycle Bin'), findsOneWidget);

      // Open Devices & Backup and generate QR + PIN
      await tester.tap(find.text('Devices & Backup'));
      await tester.pumpAndSettle();
      expect(find.text('Pair Phone & PC (Serverless P2P)'), findsOneWidget);
      await tester.tap(find.text('Show Pairing QR & 6-Digit PIN'));
      await tester.pumpAndSettle();
      expect(find.text('PIN: 482910'), findsOneWidget);

      // Open Settings view
      await tester.tap(find.text('Settings'));
      await tester.pumpAndSettle();
      expect(find.text('Library Folders & Exclusions'), findsOneWidget);
      expect(find.text('Open at Startup'), findsOneWidget);
      expect(find.text('Auto-Scan in Background on Open'), findsOneWidget);
      expect(find.text('Media Formats & YouTube Defaults'), findsOneWidget);

      // Open Audio view with Unified Music grouping chips, Sound Wave Visualizer & MiniPlayerBar
      await tester.tap(find.text('Audio'));
      await tester.pumpAndSettle();
      expect(find.text('Group Music:'), findsOneWidget);
      expect(find.byKey(const Key('grid_size_dropdown')), findsOneWidget);
      expect(find.byKey(const Key('profile_avatar_button')), findsOneWidget);
      await tester.tap(find.text('Nova Drift - Midnight Echoes').first);
      await tester.pumpAndSettle();
      expect(find.text('DETAILS'), findsOneWidget);
      expect(find.text('Details & Size'), findsOneWidget);

      // Grouping by ARTIST switches into Unified Music group view cleanly
      await tester.tap(find.text('ARTIST'));
      await tester.pumpAndSettle();
      expect(find.textContaining('Nova Drift ('), findsOneWidget);
      await tester.tap(find.text('ALL'));
      await tester.pumpAndSettle();

      // Switching to Recent & Sort dropdown works smoothly
      await tester.tap(find.text('Recent'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Sort: Recent / Date'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Sort: Size').last);
      await tester.pumpAndSettle();
      expect(find.text('Sort: Size'), findsOneWidget);

      await tester.tap(find.text('Open Viewer / Player'));
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key('audio_waveform_visualizer')),
        findsOneWidget,
      );
      expect(find.byTooltip('Playback Speed'), findsWidgets);
      expect(find.byTooltip('Slideshow (3s)'), findsOneWidget);
      await tester.tap(find.byTooltip('Close').last);
      await tester.pumpAndSettle();
      expect(find.byTooltip('Shuffle'), findsOneWidget);

      // Magic Filter chip (4K+) filters to 3840x2160 video item
      await tester.tap(find.text('Home'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('4K+'));
      await tester.pumpAndSettle();
      expect(find.text('2026-03-14 Kuta Beach Sunset'), findsWidgets);
    },
  );

  testWidgets('AndroidShell renders Home, Clean, and Devices tabs cleanly', (
    WidgetTester tester,
  ) async {
    tester.view.physicalSize = const Size(420, 840);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.resetPhysicalSize);

    await tester.pumpWidget(
      ProviderScope(
        overrides: [bridgeProvider.overrideWithValue(MockCoreBridge())],
        child: const MaterialApp(home: AndroidShell()),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('Recent Media'), findsOneWidget);

    await tester.tap(find.text('Clean'));
    await tester.pumpAndSettle();
    expect(find.text('Storage & Smart Cleanup'), findsOneWidget);

    await tester.tap(find.text('Devices'));
    await tester.pumpAndSettle();
    expect(find.text('Pair Phone & PC (Serverless P2P)'), findsOneWidget);
  });

  testWidgets(
    'ImportWizardDialog imports legacy data and displays re-login notice',
    (WidgetTester tester) async {
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            bridgeProvider.overrideWithValue(
              MockCoreBridge(simulateLegacyPresent: true),
            ),
          ],
          child: const MaterialApp(home: Scaffold(body: ImportWizardDialog())),
        ),
      );
      await tester.pumpAndSettle();

      await tester.tap(find.text('Import Library'));
      await tester.pumpAndSettle();

      expect(find.text('Imported media records: 6'), findsOneWidget);
      expect(
        find.textContaining('Legacy auth tokens were not copied'),
        findsOneWidget,
      );
    },
  );
}
