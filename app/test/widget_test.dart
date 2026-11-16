import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:media_gallery/core/bridge/mock_core_bridge.dart';
import 'package:media_gallery/core/state/gallery_providers.dart';
import 'package:media_gallery/features/migration/import_wizard_dialog.dart';
import 'package:media_gallery/features/shell/desktop_shell.dart';

void main() {
  testWidgets('DesktopShell collapses left sidebar, searches "2026 beach", and shows details sidebar',
      (WidgetTester tester) async {
    tester.view.physicalSize = const Size(1400, 900);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.resetPhysicalSize);

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          bridgeProvider.overrideWithValue(MockCoreBridge()),
        ],
        child: const MaterialApp(home: DesktopShell()),
      ),
    );
    await tester.pumpAndSettle();

    // Left sidebar shows labels initially and collapses on toggle
    expect(find.text('Videos'), findsOneWidget);
    await tester.tap(find.byKey(const Key('sidebar_collapse_toggle')));
    await tester.pumpAndSettle();
    expect(find.text('Videos'), findsNothing);

    // Searching "2026 beach" filters down to Bali & Bondi beach items
    await tester.enterText(find.byKey(const Key('top_search_field')), '2026 beach');
    await tester.pumpAndSettle();
    expect(find.text('2026-03-14 Kuta Beach Bali'), findsOneWidget);

    // Selecting an item opens the right details sidebar with aspect crop presets
    await tester.tap(find.text('2026-03-14 Kuta Beach Bali'));
    await tester.pumpAndSettle();
    expect(find.text('DETAILS'), findsOneWidget);
    expect(find.text('Crop aspect presets'), findsOneWidget);
  });

  testWidgets('ImportWizardDialog imports legacy data and displays re-login notice',
      (WidgetTester tester) async {
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          bridgeProvider.overrideWithValue(MockCoreBridge(simulateLegacyPresent: true)),
        ],
        child: const MaterialApp(home: Scaffold(body: ImportWizardDialog())),
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.text('Import Library'));
    await tester.pumpAndSettle();

    expect(find.text('Imported media records: 6'), findsOneWidget);
    expect(find.textContaining('Legacy auth tokens were not copied'), findsOneWidget);
  });
}
