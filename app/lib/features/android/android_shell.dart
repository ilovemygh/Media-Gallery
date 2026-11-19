import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_ui/material_ui.dart';
import '../../core/models/media_models.dart';
import '../../core/state/gallery_providers.dart';
import '../shared/media_widgets.dart';

/// Android mobile layout implementing PRD Section 10.8 + Google Files ergonomics:
/// - BottomNavigationBar: Home (Recent & Quick Actions), Library (Videos/Images/Audio),
///   Clean (Duplicates, Large Files, Recycle Bin), Devices (QR Pair, Backup & YouTube)
/// - Top bar: search, Scan/Refresh, YouTube, Connect chip, Google account
/// - Details bottom sheet (Rename, Alias, Cover Crop, Custom Cover, Play, Send, Trash) + Mini Player
class AndroidShell extends ConsumerStatefulWidget {
  const AndroidShell({super.key});

  @override
  ConsumerState<AndroidShell> createState() => _AndroidShellState();
}

class _AndroidShellState extends ConsumerState<AndroidShell> {
  int _navIndex = 0;

  @override
  Widget build(BuildContext context) {
    final ui = ref.watch(galleryControllerProvider);
    final ctrl = ref.read(galleryControllerProvider.notifier);

    return Scaffold(
      appBar: AppBar(
        title: TextField(
          decoration: const InputDecoration(
            hintText: 'Search files, dates, places...',
            prefixIcon: Icon(Icons.search, size: 18),
            isDense: true,
            border: OutlineInputBorder(
              borderRadius: BorderRadius.all(Radius.circular(24)),
            ),
          ),
          onChanged: ctrl.setSearchQuery,
        ),
        actions: [
          IconButton(
            tooltip: 'Scan Phone Media',
            icon: Icon(ui.isScanning ? Icons.hourglass_top : Icons.refresh),
            onPressed: ui.isScanning ? null : ctrl.runInitialScan,
          ),
          IconButton(
            tooltip: 'YouTube Downloader',
            icon: const Icon(Icons.smart_display, color: Colors.red),
            onPressed: () {
              setState(() => _navIndex = 3);
              ctrl.selectSection(NavSection.youtube);
            },
          ),
          IconButton(
            tooltip: 'Google Account',
            icon: Icon(
              Icons.account_circle,
              color: ui.googleEmail != null ? Theme.of(context).colorScheme.primary : null,
            ),
            onPressed: ctrl.toggleGoogleSignIn,
          ),
        ],
      ),
      body: Column(
        children: [
          if (ui.statusBanner != null)
            MaterialBanner(
              content: Text(ui.statusBanner!),
              actions: [
                if (ui.lastBulkBatchId != null)
                  TextButton(
                    onPressed: ctrl.undoLastBulkRename,
                    child: const Text('Undo'),
                  ),
                TextButton(
                  onPressed: ctrl.clearStatusBanner,
                  child: const Text('OK'),
                ),
              ],
            ),
          Expanded(
            child: _buildBody(context, ui, ctrl),
          ),
          const MiniPlayerBar(),
        ],
      ),
      bottomNavigationBar: NavigationBar(
        selectedIndex: _navIndex,
        onDestinationSelected: (idx) {
          setState(() => _navIndex = idx);
          if (idx == 0) ctrl.selectSection(NavSection.home);
          if (idx == 1) ctrl.selectSection(NavSection.images);
          if (idx == 2) ctrl.openCleanerMode(duplicates: true);
          if (idx == 3) ctrl.selectSection(NavSection.devices);
        },
        destinations: const [
          NavigationDestination(icon: Icon(Icons.home_outlined), selectedIcon: Icon(Icons.home), label: 'Home'),
          NavigationDestination(
              icon: Icon(Icons.photo_library_outlined), selectedIcon: Icon(Icons.photo_library), label: 'Library'),
          NavigationDestination(
              icon: Icon(Icons.cleaning_services_outlined), selectedIcon: Icon(Icons.cleaning_services), label: 'Clean'),
          NavigationDestination(icon: Icon(Icons.devices_outlined), selectedIcon: Icon(Icons.devices), label: 'Devices'),
        ],
      ),
    );
  }

  Widget _buildBody(BuildContext context, GalleryUiState ui, GalleryController ctrl) {
    if (ui.section == NavSection.youtube) {
      return const SingleChildScrollView(child: YouTubeDownloadView());
    }
    if (_navIndex == 3 || ui.section == NavSection.devices) {
      return const SingleChildScrollView(child: DevicesBackupView());
    }

    return CustomScrollView(
      slivers: [
        if (_navIndex == 0)
          SliverToBoxAdapter(
            child: _AndroidHomeHeader(
              ui: ui,
              onOpenClean: () {
                setState(() => _navIndex = 2);
                ctrl.openCleanerMode(duplicates: true);
              },
              onOpenBackup: () {
                setState(() => _navIndex = 3);
                ctrl.selectSection(NavSection.devices);
              },
              onTapMedia: (m) => _showDetailsSheet(context, m),
            ),
          ),
        if (_navIndex == 1)
          SliverToBoxAdapter(
            child: Padding(
              padding: const EdgeInsets.fromLTRB(12, 8, 12, 4),
              child: Wrap(
                spacing: 8,
                runSpacing: 6,
                crossAxisAlignment: WrapCrossAlignment.center,
                children: [
                  ChoiceChip(
                    label: const Text('All'),
                    selected: ui.section == NavSection.home,
                    onSelected: (_) => ctrl.selectSection(NavSection.home),
                  ),
                  ChoiceChip(
                    label: const Text('Images'),
                    selected: ui.section == NavSection.images,
                    onSelected: (_) => ctrl.selectSection(NavSection.images),
                  ),
                  ChoiceChip(
                    label: const Text('Videos'),
                    selected: ui.section == NavSection.videos,
                    onSelected: (_) => ctrl.selectSection(NavSection.videos),
                  ),
                  ChoiceChip(
                    label: const Text('Audio'),
                    selected: ui.section == NavSection.audio,
                    onSelected: (_) => ctrl.selectSection(NavSection.audio),
                  ),
                  FilterChip(
                    label: const Text('Select'),
                    selected: ui.selectionMode,
                    onSelected: (v) => ctrl.toggleSelectionMode(v),
                  ),
                  if (ui.selectionMode && ui.selectedIds.isNotEmpty)
                    FilledButton.tonal(
                      onPressed: () =>
                          showBulkRenameDialog(context, ref, ui.selectedIds.toList()),
                      child: Text('Rename (${ui.selectedIds.length})'),
                    ),
                ],
              ),
            ),
          ),
        if (_navIndex == 2 || ui.section == NavSection.clean)
          const SliverToBoxAdapter(child: CleanerDashboardView()),
        SliverPadding(
          padding: const EdgeInsets.all(10),
          sliver: SliverGrid(
            gridDelegate: const SliverGridDelegateWithFixedCrossAxisCount(
              crossAxisCount: 2,
              mainAxisSpacing: 8,
              crossAxisSpacing: 8,
              childAspectRatio: 0.9,
            ),
            delegate: SliverChildBuilderDelegate(
              (context, idx) {
                final item = ui.items[idx];
                final isChecked = ui.selectedIds.contains(item.id);
                return Card(
                  shape: RoundedRectangleBorder(
                    side: BorderSide(
                      color: isChecked
                          ? Theme.of(context).colorScheme.primary
                          : Colors.transparent,
                      width: 2,
                    ),
                    borderRadius: BorderRadius.circular(10),
                  ),
                  child: InkWell(
                    onTap: () => ui.selectionMode
                        ? ctrl.toggleItemSelected(item.id)
                        : _showDetailsSheet(context, item),
                    onLongPress: () => ctrl.toggleItemSelected(item.id),
                    child: Padding(
                      padding: const EdgeInsets.all(8),
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Expanded(child: MediaThumbnailWidget(item: item, cacheWidth: 320)),
                          const SizedBox(height: 6),
                          Text(
                            item.alias,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: const TextStyle(fontWeight: FontWeight.w600),
                          ),
                          Text(
                            '${formatBytes(item.size)} · ${item.syncState.name}',
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: const TextStyle(fontSize: 11),
                          ),
                        ],
                      ),
                    ),
                  ),
                );
              },
              childCount: ui.items.length,
            ),
          ),
        ),
      ],
    );
  }

  void _showDetailsSheet(BuildContext context, MediaItem item) {
    final aliasCtrl = TextEditingController(text: item.alias);
    showModalBottomSheet<void>(
      context: context,
      isScrollControlled: true,
      builder: (sheetCtx) => Padding(
        padding: EdgeInsets.fromLTRB(
          16,
          16,
          16,
          MediaQuery.of(sheetCtx).viewInsets.bottom + 16,
        ),
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              SizedBox(
                height: 160,
                width: double.infinity,
                child: MediaThumbnailWidget(item: item, cacheWidth: 500),
              ),
              const SizedBox(height: 10),
              Text(item.alias, style: Theme.of(sheetCtx).textTheme.titleMedium),
              Text('${item.fileName} · ${formatBytes(item.size)} · ${item.syncState.name}'),
              const SizedBox(height: 10),
              TextField(
                controller: aliasCtrl,
                decoration: const InputDecoration(
                  labelText: 'Alias / Rename',
                  border: OutlineInputBorder(),
                  isDense: true,
                ),
              ),
              const SizedBox(height: 10),
              Wrap(
                spacing: 8,
                runSpacing: 8,
                children: [
                  FilledButton.icon(
                    onPressed: () {
                      if (item.type == MediaType.image) {
                        Navigator.of(sheetCtx).pop();
                        showMediaViewerDialog(context, ref, item);
                      } else {
                        ref.read(galleryControllerProvider.notifier).playTrack(item);
                        Navigator.of(sheetCtx).pop();
                      }
                    },
                    icon: Icon(item.type == MediaType.image ? Icons.zoom_in : Icons.play_arrow),
                    label: Text(item.type == MediaType.image ? 'View' : 'Play'),
                  ),
                  OutlinedButton(
                    onPressed: () async {
                      await ref
                          .read(galleryControllerProvider.notifier)
                          .saveAlias(item.id, aliasCtrl.text);
                      if (sheetCtx.mounted) Navigator.of(sheetCtx).pop();
                    },
                    child: const Text('Save Alias'),
                  ),
                  OutlinedButton(
                    onPressed: () async {
                      await ref
                          .read(galleryControllerProvider.notifier)
                          .applyRealRename(item.id, aliasCtrl.text);
                      if (sheetCtx.mounted) Navigator.of(sheetCtx).pop();
                    },
                    child: const Text('Rename File'),
                  ),
                  OutlinedButton.icon(
                    onPressed: () async {
                      await ref.read(galleryControllerProvider.notifier).moveToTrash(item.id);
                      if (sheetCtx.mounted) Navigator.of(sheetCtx).pop();
                    },
                    icon: const Icon(Icons.delete_outline),
                    label: const Text('Trash'),
                  ),
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _AndroidHomeHeader extends StatelessWidget {
  const _AndroidHomeHeader({
    required this.ui,
    required this.onOpenClean,
    required this.onOpenBackup,
    required this.onTapMedia,
  });

  final GalleryUiState ui;
  final VoidCallback onOpenClean;
  final VoidCallback onOpenBackup;
  final void Function(MediaItem) onTapMedia;

  @override
  Widget build(BuildContext context) {
    final stats = ui.stats;
    return Padding(
      padding: const EdgeInsets.fromLTRB(12, 10, 12, 4),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Wrap(
            spacing: 8,
            runSpacing: 8,
            children: [
              ActionChip(
                avatar: const Icon(Icons.cleaning_services, size: 16),
                label: Text('Clean (${stats.duplicateGroups} dups)'),
                onPressed: onOpenClean,
              ),
              ActionChip(
                avatar: const Icon(Icons.backup, size: 16),
                label: const Text('Backup to PC'),
                onPressed: onOpenBackup,
              ),
              Chip(
                label: Text('${stats.totalFiles} files · ${formatBytes(stats.totalBytes)}'),
              ),
            ],
          ),
          if (ui.recentItems.isNotEmpty) ...[
            const SizedBox(height: 10),
            Text('Recent Media', style: Theme.of(context).textTheme.titleSmall),
            const SizedBox(height: 6),
            SizedBox(
              height: 112,
              child: ListView.separated(
                scrollDirection: Axis.horizontal,
                itemCount: ui.recentItems.length,
                separatorBuilder: (_, __) => const SizedBox(width: 8),
                itemBuilder: (_, idx) {
                  final item = ui.recentItems[idx];
                  return SizedBox(
                    width: 120,
                    child: Card(
                      clipBehavior: Clip.antiAlias,
                      child: InkWell(
                        onTap: () => onTapMedia(item),
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Expanded(
                              child: MediaThumbnailWidget(
                                item: item,
                                cacheWidth: 220,
                                borderRadius: 0,
                              ),
                            ),
                            Padding(
                              padding: const EdgeInsets.all(4),
                              child: Text(
                                item.alias,
                                maxLines: 1,
                                overflow: TextOverflow.ellipsis,
                                style: const TextStyle(fontSize: 11),
                              ),
                            ),
                          ],
                        ),
                      ),
                    ),
                  );
                },
              ),
            ),
          ],
        ],
      ),
    );
  }
}
