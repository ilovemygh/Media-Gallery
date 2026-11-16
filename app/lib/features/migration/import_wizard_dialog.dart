import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../../core/models/media_models.dart';
import '../../core/state/gallery_providers.dart';

/// First-launch wizard shown when legacy v0.1 data is detected (PRD Sec 16.6).
class ImportWizardDialog extends ConsumerStatefulWidget {
  const ImportWizardDialog({super.key});

  @override
  ConsumerState<ImportWizardDialog> createState() => _ImportWizardDialogState();
}

class _ImportWizardDialogState extends ConsumerState<ImportWizardDialog> {
  bool _importing = false;
  LegacyImportReport? _report;

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('Import Legacy Media Gallery Library'),
      content: SizedBox(
        width: 440,
        child: _report == null
            ? const Text(
                'An existing Media Gallery v0.1 library was detected. '
                'Import your media index, aliases, fixed video thumbnails, pinned folders, '
                'and settings without modifying legacy files.',
              )
            : Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text('Imported devices: ${_report!.importedDevices}'),
                  Text('Imported media records: ${_report!.importedMedia}'),
                  Text('Imported covers (frames preserved): ${_report!.importedCovers}'),
                  Text('Imported pinned folders: ${_report!.importedPinnedFolders}'),
                  Text('Skipped duplicates: ${_report!.skippedItems}'),
                  if (_report!.authReloginRequired)
                    const Padding(
                      padding: EdgeInsets.only(top: 10),
                      child: Text(
                        'Security notice: Legacy auth tokens were not copied in plain text. '
                        'Please re-pair or re-login if using cloud sync.',
                        style: TextStyle(fontWeight: FontWeight.w600),
                      ),
                    ),
                ],
              ),
      ),
      actions: [
        if (_report == null)
          TextButton(
            onPressed: _importing ? null : () => Navigator.of(context).pop(),
            child: const Text('Skip'),
          ),
        ElevatedButton(
          onPressed: _importing
              ? null
              : () async {
                  if (_report != null) {
                    if (context.mounted) Navigator.of(context).pop();
                    return;
                  }
                  setState(() => _importing = true);
                  final res = await ref.read(bridgeProvider).importLegacyDatabase();
                  await ref.read(galleryControllerProvider.notifier).refresh();
                  if (mounted) {
                    setState(() {
                      _importing = false;
                      _report = res;
                    });
                  }
                },
          child: Text(_report == null ? (_importing ? 'Importing...' : 'Import Library') : 'Done'),
        ),
      ],
    );
  }
}
