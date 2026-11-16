import 'dart:async';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../../core/models/media_models.dart';
import '../../core/state/gallery_providers.dart';

/// Desktop layout implementing PRD Section 10.1-10.7:
/// - Left sidebar (only collapsible sidebar, top-left toggle, drag-resizable width, spring-loaded drop targets)
/// - Top bar beside sidebar (Connect chip, instant FTS5 search, YouTube dropdown, Google button)
/// - Main area with 2x2 folder mosaic cards & virtualized SliverGrid
/// - Right details sidebar with cover crop presets (4:3, 9:16, 16:9, 1:1, custom)
/// - Bottom bar with pinned folder shortcuts
class DesktopShell extends ConsumerStatefulWidget {
  const DesktopShell({super.key});

  @override
  ConsumerState<DesktopShell> createState() => _DesktopShellState();
}

class _DesktopShellState extends ConsumerState<DesktopShell> {
  final TextEditingController _searchCtrl = TextEditingController();
  final TextEditingController _aliasCtrl = TextEditingController();
  Timer? _springTimer;
  String? _hoveredFolderPath;

  @override
  void dispose() {
    _springTimer?.cancel();
    _searchCtrl.dispose();
    _aliasCtrl.dispose();
    super.dispose();
  }

  void _startSpringOpen(String folderPath) {
    setState(() => _hoveredFolderPath = folderPath);
    _springTimer?.cancel();
    _springTimer = Timer(const Duration(milliseconds: 700), () {
      if (mounted) {
        ref.read(galleryControllerProvider.notifier).openFolder(folderPath);
      }
    });
  }

  void _cancelSpringOpen() {
    _springTimer?.cancel();
    if (_hoveredFolderPath != null) {
      setState(() => _hoveredFolderPath = null);
    }
  }

  @override
  Widget build(BuildContext context) {
    final ui = ref.watch(galleryControllerProvider);
    final ctrl = ref.read(galleryControllerProvider.notifier);
    final selected = ui.selectedMedia;

    if (selected != null && _aliasCtrl.text != selected.alias) {
      _aliasCtrl.text = selected.alias;
    }

    final connectedPeers = ui.devices.where((d) => !d.isSelf && d.isOnline).toList();

    return Scaffold(
      body: Column(
        children: [
          Expanded(
            child: Row(
              children: [
                // 10.2 Left Sidebar (ONLY collapsible panel, top-left toggle, resizable)
                SizedBox(
                  width: ui.settings.sidebarCollapsed ? 72 : ui.settings.sidebarWidth,
                  child: Material(
                    color: Theme.of(context).colorScheme.surfaceContainerLow,
                    child: Column(
                      children: [
                        SizedBox(
                          height: 56,
                          child: Row(
                            children: [
                              const SizedBox(width: 10),
                              IconButton(
                                key: const Key('sidebar_collapse_toggle'),
                                icon: const Icon(Icons.menu),
                                tooltip: 'Collapse or expand sidebar',
                                onPressed: ctrl.toggleSidebarCollapsed,
                              ),
                              if (!ui.settings.sidebarCollapsed)
                                const Expanded(
                                  child: Text(
                                    'Media Gallery',
                                    style: TextStyle(fontWeight: FontWeight.bold, fontSize: 15),
                                    overflow: TextOverflow.ellipsis,
                                  ),
                                ),
                            ],
                          ),
                        ),
                        const Divider(height: 1),
                        Expanded(
                          child: ListView(
                            padding: const EdgeInsets.all(8),
                            children: [
                              _SidebarNavTile(
                                icon: Icons.home,
                                label: 'Home',
                                collapsed: ui.settings.sidebarCollapsed,
                                selected: ui.section == NavSection.home && ui.activeFolder == null,
                                onTap: () => ctrl.selectSection(NavSection.home),
                              ),
                              if (!ui.settings.sidebarCollapsed)
                                const Padding(
                                  padding: EdgeInsets.fromLTRB(10, 12, 10, 4),
                                  child: Text('LIBRARY', style: TextStyle(fontSize: 11)),
                                ),
                              _SidebarNavTile(
                                icon: Icons.movie,
                                label: 'Videos',
                                collapsed: ui.settings.sidebarCollapsed,
                                selected: ui.section == NavSection.videos,
                                onTap: () => ctrl.selectSection(NavSection.videos),
                              ),
                              _SidebarNavTile(
                                icon: Icons.image,
                                label: 'Images',
                                collapsed: ui.settings.sidebarCollapsed,
                                selected: ui.section == NavSection.images,
                                onTap: () => ctrl.selectSection(NavSection.images),
                              ),
                              _SidebarNavTile(
                                icon: Icons.music_note,
                                label: 'Audio',
                                collapsed: ui.settings.sidebarCollapsed,
                                selected: ui.section == NavSection.audio,
                                onTap: () => ctrl.selectSection(NavSection.audio),
                              ),
                              _SidebarNavTile(
                                icon: Icons.smart_display,
                                label: 'YouTube',
                                collapsed: ui.settings.sidebarCollapsed,
                                selected: ui.section == NavSection.youtube,
                                onTap: () => ctrl.selectSection(NavSection.youtube),
                              ),
                              _SidebarNavTile(
                                icon: Icons.devices,
                                label: 'Devices & Transfers',
                                collapsed: ui.settings.sidebarCollapsed,
                                selected: ui.section == NavSection.devices,
                                onTap: () => ctrl.selectSection(NavSection.devices),
                              ),
                              if (ui.pinnedFolders.isNotEmpty) ...[
                                const Divider(),
                                for (final folder in ui.pinnedFolders)
                                  DragTarget<String>(
                                    onWillAcceptWithDetails: (_) {
                                      _startSpringOpen(folder.path);
                                      return true;
                                    },
                                    onLeave: (_) => _cancelSpringOpen(),
                                    onAcceptWithDetails: (details) {
                                      _cancelSpringOpen();
                                      ctrl.moveToFolder(details.data, folder.path);
                                    },
                                    builder: (context, candidates, _) => _SidebarNavTile(
                                      icon: Icons.folder,
                                      label: folder.label,
                                      collapsed: ui.settings.sidebarCollapsed,
                                      selected: ui.activeFolder == folder.path ||
                                          candidates.isNotEmpty,
                                      onTap: () => ctrl.openFolder(folder.path),
                                    ),
                                  ),
                              ],
                            ],
                          ),
                        ),
                      ],
                    ),
                  ),
                ),

                // Resizable sidebar divider
                if (!ui.settings.sidebarCollapsed)
                  MouseRegion(
                    cursor: SystemMouseCursors.resizeColumn,
                    child: GestureDetector(
                      onHorizontalDragUpdate: (d) =>
                          ctrl.resizeSidebar(ui.settings.sidebarWidth + d.delta.dx),
                      child: const VerticalDivider(width: 4, thickness: 1),
                    ),
                  ),

                // Right Column: TopBar BESIDE sidebar + Main Content + Right Details Sidebar
                Expanded(
                  child: Column(
                    children: [
                      // 10.3 Top Bar
                      Container(
                        height: 56,
                        padding: const EdgeInsets.symmetric(horizontal: 14),
                        decoration: BoxDecoration(
                          color: Theme.of(context).colorScheme.surfaceContainerLow,
                          border: Border(
                            bottom: BorderSide(color: Theme.of(context).dividerColor),
                          ),
                        ),
                        child: Row(
                          children: [
                            ActionChip(
                              avatar: Icon(
                                connectedPeers.isNotEmpty ? Icons.wifi : Icons.link,
                                size: 16,
                              ),
                              label: Text(
                                connectedPeers.isNotEmpty
                                    ? connectedPeers.first.name
                                    : 'Connect',
                              ),
                              onPressed: () => ctrl.selectSection(NavSection.devices),
                            ),
                            const SizedBox(width: 10),
                            Expanded(
                              child: TextField(
                                key: const Key('top_search_field'),
                                controller: _searchCtrl,
                                decoration: const InputDecoration(
                                  prefixIcon: Icon(Icons.search, size: 18),
                                  hintText: 'Search ("2026 beach")...',
                                  isDense: true,
                                  border: OutlineInputBorder(
                                    borderRadius: BorderRadius.all(Radius.circular(24)),
                                  ),
                                ),
                                onChanged: ctrl.setSearchQuery,
                              ),
                            ),
                            const SizedBox(width: 10),
                            PopupMenuButton<String>(
                              tooltip: 'YouTube',
                              onSelected: (action) {
                                if (action == 'download') {
                                  _showYouTubeModal(context);
                                }
                              },
                              itemBuilder: (_) => const [
                                PopupMenuItem(value: 'upload', child: Text('Upload')),
                                PopupMenuItem(
                                  value: 'download',
                                  child: Text('Download from YouTube'),
                                ),
                                PopupMenuItem(value: 'channel', child: Text('Your channel')),
                                PopupMenuItem(value: 'studio', child: Text('YouTube Studio')),
                              ],
                              child: const Chip(
                                avatar: Icon(Icons.play_circle_fill, color: Colors.red, size: 18),
                                label: Text('YouTube'),
                              ),
                            ),
                            const SizedBox(width: 8),
                            IconButton(
                              tooltip: 'Toggle theme',
                              icon: const Icon(Icons.brightness_6),
                              onPressed: ctrl.toggleTheme,
                            ),
                            ActionChip(
                              avatar: const Icon(Icons.account_circle, size: 18),
                              label: Text(ui.googleEmail ?? 'Google'),
                              onPressed: ctrl.toggleGoogleSignIn,
                            ),
                          ],
                        ),
                      ),

                      // Workspace Split: Main Gallery + Right Details Sidebar
                      Expanded(
                        child: Row(
                          children: [
                            Expanded(
                              child: CustomScrollView(
                                slivers: [
                                  SliverToBoxAdapter(
                                    child: Padding(
                                      padding: const EdgeInsets.all(14),
                                      child: Row(
                                        children: [
                                          Text(
                                            ui.activeFolder ?? ui.section.name.toUpperCase(),
                                            style: Theme.of(context).textTheme.titleMedium,
                                          ),
                                          const SizedBox(width: 8),
                                          Chip(label: Text('${ui.items.length} items')),
                                          if (ui.activeFolder != null) ...[
                                            const SizedBox(width: 8),
                                            OutlinedButton(
                                              onPressed: () => ctrl.openFolder(null),
                                              child: const Text('All folders'),
                                            ),
                                          ],
                                        ],
                                      ),
                                    ),
                                  ),
                                  if (ui.activeFolder == null && ui.folders.isNotEmpty)
                                    SliverPadding(
                                      padding: const EdgeInsets.symmetric(horizontal: 14),
                                      sliver: SliverGrid(
                                        gridDelegate:
                                            const SliverGridDelegateWithMaxCrossAxisExtent(
                                          maxCrossAxisExtent: 220,
                                          mainAxisSpacing: 10,
                                          crossAxisSpacing: 10,
                                          childAspectRatio: 1.25,
                                        ),
                                        delegate: SliverChildBuilderDelegate(
                                          (context, idx) {
                                            final folder = ui.folders[idx];
                                            return DragTarget<String>(
                                              onWillAcceptWithDetails: (_) {
                                                _startSpringOpen(folder.path);
                                                return true;
                                              },
                                              onLeave: (_) => _cancelSpringOpen(),
                                              onAcceptWithDetails: (d) {
                                                _cancelSpringOpen();
                                                ctrl.moveToFolder(d.data, folder.path);
                                              },
                                              builder: (context, candidates, _) => Card(
                                                color: candidates.isNotEmpty
                                                    ? Colors.green.withValues(alpha: 0.2)
                                                    : null,
                                                child: InkWell(
                                                  onTap: () => ctrl.openFolder(folder.path),
                                                  child: Padding(
                                                    padding: const EdgeInsets.all(10),
                                                    child: Column(
                                                      mainAxisAlignment:
                                                          MainAxisAlignment.spaceBetween,
                                                      crossAxisAlignment:
                                                          CrossAxisAlignment.start,
                                                      children: [
                                                        const Icon(Icons.folder, size: 36),
                                                        Text(
                                                          '${folder.name} (${folder.itemCount})',
                                                          overflow: TextOverflow.ellipsis,
                                                        ),
                                                      ],
                                                    ),
                                                  ),
                                                ),
                                              ),
                                            );
                                          },
                                          childCount: ui.folders.length,
                                        ),
                                      ),
                                    ),
                                  SliverPadding(
                                    padding: const EdgeInsets.all(14),
                                    sliver: SliverGrid(
                                      gridDelegate:
                                          const SliverGridDelegateWithMaxCrossAxisExtent(
                                        maxCrossAxisExtent: 240,
                                        mainAxisSpacing: 12,
                                        crossAxisSpacing: 12,
                                        childAspectRatio: 0.95,
                                      ),
                                      delegate: SliverChildBuilderDelegate(
                                        (context, idx) {
                                          final item = ui.items[idx];
                                          final isSelected = selected?.id == item.id;
                                          return Draggable<String>(
                                            data: item.id,
                                            feedback: Material(
                                              elevation: 6,
                                              child: Padding(
                                                padding: const EdgeInsets.all(8),
                                                child: Text(item.alias),
                                              ),
                                            ),
                                            child: Card(
                                              shape: RoundedRectangleBorder(
                                                side: BorderSide(
                                                  color: isSelected
                                                      ? Theme.of(context).colorScheme.primary
                                                      : Colors.transparent,
                                                  width: 2,
                                                ),
                                                borderRadius: BorderRadius.circular(10),
                                              ),
                                              child: InkWell(
                                                onTap: () => ctrl.selectMedia(item.id),
                                                child: Padding(
                                                  padding: const EdgeInsets.all(10),
                                                  child: Column(
                                                    crossAxisAlignment:
                                                        CrossAxisAlignment.start,
                                                    children: [
                                                      Expanded(
                                                        child: Container(
                                                          width: double.infinity,
                                                          decoration: BoxDecoration(
                                                            color: Theme.of(context)
                                                                .colorScheme
                                                                .surfaceContainerHighest,
                                                            borderRadius:
                                                                BorderRadius.circular(8),
                                                          ),
                                                          child: Center(
                                                            child: Icon(
                                                              item.type == MediaType.video
                                                                  ? Icons.movie
                                                                  : item.type == MediaType.audio
                                                                      ? Icons.music_note
                                                                      : Icons.image,
                                                              size: 40,
                                                            ),
                                                          ),
                                                        ),
                                                      ),
                                                      const SizedBox(height: 8),
                                                      Text(
                                                        item.alias,
                                                        maxLines: 1,
                                                        overflow: TextOverflow.ellipsis,
                                                        style: const TextStyle(
                                                          fontWeight: FontWeight.w600,
                                                        ),
                                                      ),
                                                      Text(
                                                        '${item.fileName} · ${item.syncState.name}',
                                                        maxLines: 1,
                                                        overflow: TextOverflow.ellipsis,
                                                        style: const TextStyle(fontSize: 11),
                                                      ),
                                                      if (item.syncState == SyncState.remote)
                                                        Padding(
                                                          padding: const EdgeInsets.only(top: 4),
                                                          child: FilledButton.tonalIcon(
                                                            onPressed: item.deviceOnline
                                                                ? () => ctrl.downloadRemote(item.id)
                                                                : null,
                                                            icon: const Icon(Icons.cloud_download,
                                                                size: 14),
                                                            label: Text(
                                                              item.deviceOnline
                                                                  ? 'Download'
                                                                  : 'Offline',
                                                            ),
                                                          ),
                                                        ),
                                                    ],
                                                  ),
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
                              ),
                            ),

                            // 10.5 Right Details Sidebar
                            if (selected != null)
                              Container(
                                width: 310,
                                padding: const EdgeInsets.all(14),
                                decoration: BoxDecoration(
                                  color: Theme.of(context).colorScheme.surfaceContainerLow,
                                  border: Border(
                                    left: BorderSide(color: Theme.of(context).dividerColor),
                                  ),
                                ),
                                child: ListView(
                                  children: [
                                    Row(
                                      mainAxisAlignment: MainAxisAlignment.spaceBetween,
                                      children: [
                                        const Text(
                                          'DETAILS',
                                          style: TextStyle(fontWeight: FontWeight.bold),
                                        ),
                                        IconButton(
                                          icon: const Icon(Icons.close, size: 18),
                                          onPressed: ctrl.closeDetailsForSession,
                                        ),
                                      ],
                                    ),
                                    const SizedBox(height: 8),
                                    Text(selected.fileName,
                                        style: const TextStyle(fontWeight: FontWeight.w600)),
                                    const SizedBox(height: 8),
                                    TextField(
                                      controller: _aliasCtrl,
                                      decoration: const InputDecoration(
                                        labelText: 'Alias (non-destructive)',
                                        isDense: true,
                                      ),
                                    ),
                                    const SizedBox(height: 8),
                                    Wrap(
                                      spacing: 6,
                                      children: [
                                        FilledButton(
                                          onPressed: () =>
                                              ctrl.saveAlias(selected.id, _aliasCtrl.text),
                                          child: const Text('Save Alias'),
                                        ),
                                        OutlinedButton(
                                          onPressed: () =>
                                              ctrl.applyRealRename(selected.id, _aliasCtrl.text),
                                          child: const Text('Apply Real Rename'),
                                        ),
                                      ],
                                    ),
                                    const Divider(height: 22),
                                    const Text('Crop aspect presets'),
                                    const SizedBox(height: 6),
                                    Wrap(
                                      spacing: 6,
                                      children: [
                                        for (final preset in AspectPreset.values)
                                          ChoiceChip(
                                            label: Text(preset.name),
                                            selected: selected.cover?.aspect == preset,
                                            onSelected: (_) =>
                                                ctrl.updateCoverAspect(selected.id, preset),
                                          ),
                                      ],
                                    ),
                                    const SizedBox(height: 12),
                                    if (selected.type == MediaType.video)
                                      OutlinedButton.icon(
                                        onPressed: () => ctrl.rerollVideoThumbnail(selected.id),
                                        icon: const Icon(Icons.refresh, size: 16),
                                        label: const Text('Re-roll First-Third Frame'),
                                      ),
                                    const SizedBox(height: 10),
                                    OutlinedButton.icon(
                                      onPressed: () => ctrl.moveToTrash(selected.id),
                                      icon: const Icon(Icons.delete_outline, size: 16),
                                      label: const Text('Move to Recycle Bin'),
                                    ),
                                  ],
                                ),
                              ),
                          ],
                        ),
                      ),
                    ],
                  ),
                ),
              ],
            ),
          ),

          // 10.6 Bottom Bar: Pinned Folder Shortcuts
          Container(
            height: 46,
            padding: const EdgeInsets.symmetric(horizontal: 12),
            decoration: BoxDecoration(
              color: Theme.of(context).colorScheme.surfaceContainerLow,
              border: Border(top: BorderSide(color: Theme.of(context).dividerColor)),
            ),
            child: ListView(
              scrollDirection: Axis.horizontal,
              children: [
                for (final pin in ui.pinnedFolders)
                  Padding(
                    padding: const EdgeInsets.only(right: 8),
                    child: DragTarget<String>(
                      onWillAcceptWithDetails: (_) {
                        _startSpringOpen(pin.path);
                        return true;
                      },
                      onLeave: (_) => _cancelSpringOpen(),
                      onAcceptWithDetails: (d) {
                        _cancelSpringOpen();
                        ctrl.moveToFolder(d.data, pin.path);
                      },
                      builder: (context, _, __) => InputChip(
                        avatar: const Icon(Icons.folder, size: 16),
                        label: Text(pin.label),
                        selected: ui.activeFolder == pin.path,
                        onPressed: () => ctrl.openFolder(pin.path),
                        onDeleted: () => ctrl.removePinnedFolder(pin.id),
                      ),
                    ),
                  ),
                ActionChip(
                  avatar: const Icon(Icons.add, size: 16),
                  label: const Text('Pin folder'),
                  onPressed: () =>
                      ctrl.addPinnedFolder(ui.activeFolder ?? 'C:/Media/Favorites'),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  void _showYouTubeModal(BuildContext context) {
    final urlCtrl = TextEditingController();
    showDialog<void>(
      context: context,
      builder: (_) => AlertDialog(
        title: const Text('Download from YouTube'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            TextField(
              controller: urlCtrl,
              decoration: const InputDecoration(labelText: 'YouTube URL'),
            ),
            const SizedBox(height: 8),
            const Text(
              'Download only content you have the right to download.',
              style: TextStyle(fontSize: 12),
            ),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () async {
              await ref.read(bridgeProvider).enqueueYoutubeDownload(
                    url: urlCtrl.text,
                    format: 'mp4',
                    quality: '1080p',
                    destinationFolder: 'C:/Media/YouTube',
                  );
              await ref.read(galleryControllerProvider.notifier).refresh();
              if (context.mounted) Navigator.of(context).pop();
            },
            child: const Text('Download'),
          ),
        ],
      ),
    );
  }
}

class _SidebarNavTile extends StatelessWidget {
  const _SidebarNavTile({
    required this.icon,
    required this.label,
    required this.collapsed,
    required this.selected,
    required this.onTap,
  });

  final IconData icon;
  final String label;
  final bool collapsed;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return ListTile(
      dense: true,
      selected: selected,
      leading: Icon(icon),
      title: collapsed ? null : Text(label, overflow: TextOverflow.ellipsis),
      onTap: onTap,
    );
  }
}
