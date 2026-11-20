import 'dart:async';
import 'package:desktop_drop/desktop_drop.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_ui/material_ui.dart';
import '../../core/models/media_models.dart';
import '../../core/state/gallery_providers.dart';
import '../shared/media_widgets.dart';

/// Desktop layout implementing PRD Section 10.1-10.7 + Google Files / Explorer upgrades:
/// - Collapsible & drag-resizable left sidebar with spring-loaded folder drop targets
/// - Top bar beside sidebar (Connect chip, FTS5 search, + Scan Folder, YouTube dropdown, Google)
/// - Home Recent strip, Storage Cleaner (Duplicates, Large files, Trash), Devices/QR, YouTube queue
/// - Virtualized S/M/L grids & List view with fast Image.file thumbnails + 2x2 folder mosaics
/// - Multi-select + Bulk Rename with preview/undo + Built-in Media Viewer + Mini Music Player
/// - Right details sidebar with cover preview, crop presets, custom cover, frame scrubber, undo rename
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
  String? _lastSelectedIdForAlias;

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

  double _maxExtentForDensity(GridDensity density) {
    switch (density) {
      case GridDensity.small:
        return 175;
      case GridDensity.medium:
        return 235;
      case GridDensity.large:
        return 320;
      case GridDensity.list:
        return 240;
    }
  }

  @override
  Widget build(BuildContext context) {
    final ui = ref.watch(galleryControllerProvider);
    final ctrl = ref.read(galleryControllerProvider.notifier);
    final selected = ui.selectedMedia;

    if (selected != null && selected.id != _lastSelectedIdForAlias) {
      _lastSelectedIdForAlias = selected.id;
      _aliasCtrl.text = selected.alias;
    }

    final connectedPeers = ui.devices.where((d) => !d.isSelf && d.isOnline).toList();

    return Scaffold(
      body: DropTarget(
        onDragDone: (details) async {
          final paths = details.files.map((f) => f.path).where((p) => p.isNotEmpty).toList();
          if (paths.isNotEmpty) {
            await ctrl.scanFolderPaths(paths);
          }
        },
        child: Column(
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
                                if (!ui.settings.sidebarCollapsed) ...[
                                  const MediaGalleryLogo(size: 26),
                                  const SizedBox(width: 8),
                                  const Expanded(
                                    child: Text(
                                      'Media Gallery',
                                      style: TextStyle(fontWeight: FontWeight.bold, fontSize: 15),
                                      overflow: TextOverflow.ellipsis,
                                    ),
                                  ),
                                ],
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
                                _SidebarNavTile(
                                  icon: Icons.schedule,
                                  label: 'Recent',
                                  collapsed: ui.settings.sidebarCollapsed,
                                  selected: ui.section == NavSection.recent,
                                  onTap: () => ctrl.selectSection(NavSection.recent),
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
                                  icon: Icons.cleaning_services,
                                  label: 'Clean & Storage',
                                  collapsed: ui.settings.sidebarCollapsed,
                                  selected: ui.section == NavSection.clean,
                                  onTap: () => ctrl.openCleanerMode(duplicates: true),
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
                                  label: 'Devices & Backup',
                                  collapsed: ui.settings.sidebarCollapsed,
                                  selected: ui.section == NavSection.devices,
                                  onTap: () => ctrl.selectSection(NavSection.devices),
                                ),
                                _SidebarNavTile(
                                  icon: Icons.settings,
                                  label: 'Settings',
                                  collapsed: ui.settings.sidebarCollapsed,
                                  selected: ui.section == NavSection.settings,
                                  onTap: () => ctrl.selectSection(NavSection.settings),
                                ),
                                if (ui.savedSearches.isNotEmpty) ...[
                                  const Divider(),
                                  if (!ui.settings.sidebarCollapsed)
                                    const Padding(
                                      padding: EdgeInsets.fromLTRB(10, 4, 10, 4),
                                      child: Text('SAVED SEARCHES', style: TextStyle(fontSize: 11)),
                                    ),
                                  for (final s in ui.savedSearches)
                                    _SidebarNavTile(
                                      icon: Icons.bookmark_outline,
                                      label: s.name,
                                      collapsed: ui.settings.sidebarCollapsed,
                                      selected: false,
                                      onTap: () {
                                        _searchCtrl.text = s.query;
                                        ctrl.applySavedSearch(s);
                                      },
                                    ),
                                ],
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
                                    decoration: InputDecoration(
                                      prefixIcon: const Icon(Icons.search, size: 18),
                                      suffixIcon: ui.searchQuery.isNotEmpty
                                          ? Row(
                                              mainAxisSize: MainAxisSize.min,
                                              children: [
                                                IconButton(
                                                  tooltip: 'Save smart search',
                                                  icon: const Icon(Icons.bookmark_add_outlined,
                                                      size: 16),
                                                  onPressed: () =>
                                                      ctrl.saveCurrentSearch(ui.searchQuery),
                                                ),
                                                IconButton(
                                                  icon: const Icon(Icons.clear, size: 16),
                                                  onPressed: () {
                                                    _searchCtrl.clear();
                                                    ctrl.setSearchQuery('');
                                                  },
                                                ),
                                              ],
                                            )
                                          : null,
                                      hintText: 'Search ("2026 beach", date, folder, device)...',
                                      isDense: true,
                                      border: const OutlineInputBorder(
                                        borderRadius: BorderRadius.all(Radius.circular(24)),
                                      ),
                                    ),
                                    onChanged: ctrl.setSearchQuery,
                                  ),
                                ),
                              const SizedBox(width: 8),
                              FilledButton.tonalIcon(
                                onPressed: ui.isScanning
                                    ? null
                                    : () => pickAndScanFolder(context, ref),
                                icon: Icon(
                                  ui.isScanning ? Icons.hourglass_top : Icons.create_new_folder,
                                  size: 17,
                                ),
                                label: Text(ui.isScanning ? 'Scanning...' : 'Scan Folder'),
                              ),
                              const SizedBox(width: 8),
                              PopupMenuButton<String>(
                                tooltip: 'YouTube',
                                onSelected: (action) {
                                  if (action == 'download') {
                                    _showYouTubeModal(context);
                                  } else if (action == 'queue') {
                                    ctrl.selectSection(NavSection.youtube);
                                  }
                                },
                                itemBuilder: (_) => const [
                                  PopupMenuItem(value: 'upload', child: Text('Upload')),
                                  PopupMenuItem(
                                    value: 'download',
                                    child: Text('Download from YouTube'),
                                  ),
                                  PopupMenuItem(
                                    value: 'queue',
                                    child: Text('YouTube Downloader & Queue'),
                                  ),
                                  PopupMenuItem(value: 'channel', child: Text('Your channel')),
                                  PopupMenuItem(value: 'studio', child: Text('YouTube Studio')),
                                ],
                                child: const Chip(
                                  avatar: Icon(Icons.play_circle_fill, color: Colors.red, size: 18),
                                  label: Text('YouTube'),
                                ),
                              ),
                              const SizedBox(width: 6),
                              IconButton(
                                tooltip: 'Toggle theme',
                                icon: const Icon(Icons.brightness_6),
                                onPressed: ctrl.toggleTheme,
                              ),
                              const ProfileAvatarButton(),
                            ],
                          ),
                        ),

                        if (ui.statusBanner != null)
                          MaterialBanner(
                            content: Text(ui.statusBanner!),
                            actions: [
                              if (ui.lastBulkBatchId != null)
                                TextButton(
                                  onPressed: ctrl.undoLastBulkRename,
                                  child: const Text('Undo Rename'),
                                ),
                              TextButton(
                                onPressed: ctrl.clearStatusBanner,
                                child: const Text('Dismiss'),
                              ),
                            ],
                          ),

                        // Workspace Split: Main Gallery + Right Details Sidebar
                        Expanded(
                          child: Row(
                            children: [
                              Expanded(
                                child: CustomScrollView(
                                  slivers: [
                                    if (ui.section == NavSection.devices)
                                      const SliverToBoxAdapter(child: DevicesBackupView()),
                                    if (ui.section == NavSection.youtube)
                                      const SliverToBoxAdapter(child: YouTubeDownloadView()),
                                    if (ui.section == NavSection.clean)
                                      const SliverToBoxAdapter(child: CleanerDashboardView()),
                                    if (ui.section == NavSection.settings)
                                      const SliverToBoxAdapter(child: SettingsView()),

                                    // Unified Music Library Grouping Chips
                                    if (ui.section == NavSection.audio)
                                      SliverToBoxAdapter(
                                        child: Padding(
                                          padding: const EdgeInsets.fromLTRB(14, 10, 14, 2),
                                          child: Wrap(
                                            spacing: 8,
                                            runSpacing: 6,
                                            crossAxisAlignment: WrapCrossAlignment.center,
                                            children: [
                                              const Text('Group Music:',
                                                  style: TextStyle(fontWeight: FontWeight.w600)),
                                              for (final gb in MusicGroupBy.values)
                                                ChoiceChip(
                                                  label: Text(gb.name.toUpperCase()),
                                                  selected: ui.musicGroupBy == gb,
                                                  onSelected: (_) => ctrl.setMusicGroupBy(gb),
                                                ),
                                            ],
                                          ),
                                        ),
                                      ),

                                    // Home Overview: Recent strip + Quick Cleanup summary
                                    if (ui.section == NavSection.home &&
                                        ui.activeFolder == null &&
                                        ui.searchQuery.isEmpty)
                                      SliverToBoxAdapter(
                                        child: _HomeOverviewSection(ui: ui),
                                      ),

                                    // Explorer Toolbar (Header, Density S/M/L/List, Sort, Multi-select)
                                    if (ui.section != NavSection.devices &&
                                        ui.section != NavSection.youtube &&
                                        ui.section != NavSection.settings)
                                      SliverToBoxAdapter(
                                        child: Padding(
                                          padding: const EdgeInsets.fromLTRB(14, 12, 14, 6),
                                          child: Wrap(
                                            spacing: 8,
                                            runSpacing: 8,
                                            crossAxisAlignment: WrapCrossAlignment.center,
                                            children: [
                                              Text(
                                                ui.activeFolder ?? ui.section.name.toUpperCase(),
                                                style: Theme.of(context).textTheme.titleMedium,
                                              ),
                                              Chip(label: Text('${ui.items.length} items')),
                                              if (ui.activeFolder != null)
                                                OutlinedButton(
                                                  onPressed: () => ctrl.openFolder(null),
                                                  child: const Text('All folders'),
                                                ),
                                              const SizedBox(width: 12),
                                              GridSizeViewDropdown(
                                                value: ui.gridDensity,
                                                onChanged: ctrl.setGridDensity,
                                              ),
                                              DropdownButton<SortField>(
                                                value: ui.sortField,
                                                items: SortField.values
                                                    .map((f) => DropdownMenuItem(
                                                          value: f,
                                                          child: Text('Sort: ${f.name}'),
                                                        ))
                                                    .toList(),
                                                onChanged: (f) {
                                                  if (f != null) ctrl.setSort(f, ui.sortDir);
                                                },
                                              ),
                                              IconButton(
                                                tooltip: ui.sortDir == SortDir.desc
                                                    ? 'Descending'
                                                    : 'Ascending',
                                                icon: Icon(ui.sortDir == SortDir.desc
                                                    ? Icons.arrow_downward
                                                    : Icons.arrow_upward),
                                                onPressed: () => ctrl.setSort(
                                                  ui.sortField,
                                                  ui.sortDir == SortDir.desc
                                                      ? SortDir.asc
                                                      : SortDir.desc,
                                                ),
                                              ),
                                              FilterChip(
                                                label: const Text('Select'),
                                                selected: ui.selectionMode,
                                                onSelected: (v) => ctrl.toggleSelectionMode(v),
                                              ),
                                              if (ui.selectionMode && ui.selectedIds.isNotEmpty) ...[
                                                FilledButton.tonalIcon(
                                                  onPressed: () => showBulkRenameDialog(
                                                    context,
                                                    ref,
                                                    ui.selectedIds.toList(),
                                                  ),
                                                  icon: const Icon(Icons.drive_file_rename_outline,
                                                      size: 16),
                                                  label: Text(
                                                      'Bulk Rename (${ui.selectedIds.length})'),
                                                ),
                                                OutlinedButton.icon(
                                                  onPressed: ctrl.trashSelected,
                                                  icon: const Icon(Icons.delete_outline, size: 16),
                                                  label: const Text('Trash'),
                                                ),
                                              ],
                                            ],
                                          ),
                                        ),
                                      ),

                                    // Folder Cards with 2x2 mosaic + spring-open drag target
                                    if (ui.section != NavSection.devices &&
                                        ui.section != NavSection.youtube &&
                                        ui.section != NavSection.clean &&
                                        ui.section != NavSection.settings &&
                                        ui.activeFolder == null &&
                                        ui.folders.isNotEmpty)
                                      SliverPadding(
                                        padding: const EdgeInsets.symmetric(horizontal: 14),
                                        sliver: SliverGrid(
                                          gridDelegate:
                                              const SliverGridDelegateWithMaxCrossAxisExtent(
                                            maxCrossAxisExtent: 230,
                                            mainAxisSpacing: 10,
                                            crossAxisSpacing: 10,
                                            childAspectRatio: 1.5,
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
                                                          FolderMosaicPreview(folder: folder),
                                                          Text(
                                                            '${folder.name} (${folder.itemCount})',
                                                            maxLines: 1,
                                                            overflow: TextOverflow.ellipsis,
                                                            style: const TextStyle(
                                                                fontWeight: FontWeight.w500),
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

                                    // Main Media Grid or List
                                    if (ui.section != NavSection.devices &&
                                        ui.section != NavSection.youtube &&
                                        ui.section != NavSection.settings)
                                      if (ui.gridDensity == GridDensity.list)
                                        SliverList(
                                          delegate: SliverChildBuilderDelegate(
                                            (context, idx) {
                                              final item = ui.items[idx];
                                              final isChecked = ui.selectedIds.contains(item.id);
                                              return ListTile(
                                                leading: SizedBox(
                                                  width: 48,
                                                  height: 48,
                                                  child: MediaThumbnailWidget(
                                                      item: item, cacheWidth: 140),
                                                ),
                                                title: Text(item.alias,
                                                    maxLines: 1, overflow: TextOverflow.ellipsis),
                                                subtitle: Text(
                                                  '${item.fileName} · ${formatBytes(item.size)} · ${item.deviceName}',
                                                  maxLines: 1,
                                                  overflow: TextOverflow.ellipsis,
                                                ),
                                                selected: selected?.id == item.id || isChecked,
                                                trailing: ui.selectionMode
                                                    ? Checkbox(
                                                        value: isChecked,
                                                        onChanged: (_) =>
                                                            ctrl.toggleItemSelected(item.id),
                                                      )
                                                    : (ui.trashedOnly
                                                        ? TextButton(
                                                            onPressed: () =>
                                                                ctrl.restoreFromTrash(item.id),
                                                            child: const Text('Restore'),
                                                          )
                                                        : IconButton(
                                                            icon: const Icon(Icons.open_in_new,
                                                                size: 18),
                                                            onPressed: () => showMediaViewerDialog(
                                                                context, ref, item),
                                                          )),
                                                onTap: () => ui.selectionMode
                                                    ? ctrl.toggleItemSelected(item.id)
                                                    : ctrl.selectMedia(item.id),
                                              );
                                            },
                                            childCount: ui.items.length,
                                          ),
                                        )
                                      else
                                        SliverPadding(
                                          padding: const EdgeInsets.all(14),
                                          sliver: SliverGrid(
                                            gridDelegate:
                                                SliverGridDelegateWithMaxCrossAxisExtent(
                                              maxCrossAxisExtent:
                                                  _maxExtentForDensity(ui.gridDensity),
                                              mainAxisSpacing: 12,
                                              crossAxisSpacing: 12,
                                              childAspectRatio: 0.92,
                                            ),
                                            delegate: SliverChildBuilderDelegate(
                                              (context, idx) {
                                                final item = ui.items[idx];
                                                final isSelected = selected?.id == item.id;
                                                final isChecked = ui.selectedIds.contains(item.id);
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
                                                        color: (isSelected || isChecked)
                                                            ? Theme.of(context).colorScheme.primary
                                                            : Colors.transparent,
                                                        width: 2,
                                                      ),
                                                      borderRadius: BorderRadius.circular(10),
                                                    ),
                                                    child: InkWell(
                                                      onTap: () => ui.selectionMode
                                                          ? ctrl.toggleItemSelected(item.id)
                                                          : ctrl.selectMedia(item.id),
                                                      child: Padding(
                                                        padding: const EdgeInsets.all(8),
                                                        child: Column(
                                                          crossAxisAlignment:
                                                              CrossAxisAlignment.start,
                                                          children: [
                                                            Expanded(
                                                              child: Stack(
                                                                children: [
                                                                  MediaThumbnailWidget(item: item),
                                                                  if (ui.selectionMode)
                                                                    Positioned(
                                                                      right: 4,
                                                                      top: 4,
                                                                      child: Checkbox(
                                                                        value: isChecked,
                                                                        onChanged: (_) => ctrl
                                                                            .toggleItemSelected(
                                                                                item.id),
                                                                      ),
                                                                    ),
                                                                ],
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
                                                              '${item.fileName} · ${formatBytes(item.size)}',
                                                              maxLines: 1,
                                                              overflow: TextOverflow.ellipsis,
                                                              style: const TextStyle(fontSize: 11),
                                                            ),
                                                            if (ui.trashedOnly)
                                                              Padding(
                                                                padding:
                                                                    const EdgeInsets.only(top: 4),
                                                                child: FilledButton.tonal(
                                                                  onPressed: () => ctrl
                                                                      .restoreFromTrash(item.id),
                                                                  child: const Text('Restore'),
                                                                ),
                                                              )
                                                            else if (item.syncState ==
                                                                SyncState.remote)
                                                              Padding(
                                                                padding:
                                                                    const EdgeInsets.only(top: 4),
                                                                child: FilledButton.tonalIcon(
                                                                  onPressed: item.deviceOnline
                                                                      ? () => ctrl
                                                                          .downloadRemote(item.id)
                                                                      : null,
                                                                  icon: const Icon(
                                                                      Icons.cloud_download,
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
                                  width: 320,
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
                                      SizedBox(
                                        height: 150,
                                        child: MediaThumbnailWidget(
                                            item: selected, cacheWidth: 480),
                                      ),
                                      const SizedBox(height: 10),
                                      Text(selected.fileName,
                                          style: const TextStyle(fontWeight: FontWeight.w600)),
                                      Text(
                                        '${selected.type.name.toUpperCase()} · ${formatBytes(selected.size)} · ${selected.deviceName}',
                                        style: const TextStyle(fontSize: 12),
                                      ),
                                      const SizedBox(height: 8),
                                      FilledButton.tonalIcon(
                                        onPressed: () => showMediaViewerDialog(
                                            context, ref, selected),
                                        icon: const Icon(Icons.open_in_full, size: 16),
                                        label: const Text('Open Viewer / Player'),
                                      ),
                                      const SizedBox(height: 6),

                                      // Collapsible Dropdown 1: Details & Size
                                      ExpansionTile(
                                        tilePadding: EdgeInsets.zero,
                                        title: const Text(
                                          'Details & Size',
                                          style: TextStyle(fontSize: 13, fontWeight: FontWeight.w600),
                                        ),
                                        subtitle: Text(
                                          '${formatBytes(selected.size)} · ${selected.type.name.toUpperCase()}',
                                          style: const TextStyle(fontSize: 11),
                                        ),
                                        childrenPadding: const EdgeInsets.only(bottom: 8),
                                        expandedCrossAxisAlignment: CrossAxisAlignment.start,
                                        children: [
                                          Text('File: ${selected.fileName}',
                                              style: const TextStyle(fontSize: 12)),
                                          Text('Size: ${formatBytes(selected.size)} (${selected.size} bytes)',
                                              style: const TextStyle(fontSize: 12)),
                                          if (selected.width > 0 && selected.height > 0)
                                            Text('Resolution: ${selected.width} × ${selected.height}',
                                                style: const TextStyle(fontSize: 12)),
                                          if (selected.durationMs > 0)
                                            Text('Duration: ${formatDuration(selected.durationMs)}',
                                                style: const TextStyle(fontSize: 12)),
                                          Text('Device: ${selected.deviceName} (${selected.syncState.name})',
                                              style: const TextStyle(fontSize: 12)),
                                          Text('Folder: ${selected.folderPath}',
                                              style: const TextStyle(fontSize: 12)),
                                          if (selected.place != null)
                                            Text('Location: ${selected.place}',
                                                style: const TextStyle(fontSize: 12)),
                                          if (selected.artist != null)
                                            Text('Artist: ${selected.artist}',
                                                style: const TextStyle(fontSize: 12)),
                                        ],
                                      ),

                                      // Collapsible Dropdown 2: Rename & Alias
                                      ExpansionTile(
                                        tilePadding: EdgeInsets.zero,
                                        title: const Text(
                                          'Rename & Alias',
                                          style: TextStyle(fontSize: 13, fontWeight: FontWeight.w600),
                                        ),
                                        childrenPadding: const EdgeInsets.only(bottom: 8),
                                        children: [
                                          TextField(
                                            controller: _aliasCtrl,
                                            decoration: const InputDecoration(
                                              labelText: 'Alias / New Name',
                                              isDense: true,
                                              border: OutlineInputBorder(),
                                            ),
                                          ),
                                          const SizedBox(height: 8),
                                          Wrap(
                                            spacing: 6,
                                            runSpacing: 6,
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
                                              TextButton.icon(
                                                onPressed: () => ctrl.undoLastRename(selected.id),
                                                icon: const Icon(Icons.undo, size: 15),
                                                label: const Text('Undo Rename'),
                                              ),
                                            ],
                                          ),
                                        ],
                                      ),

                                      // Collapsible Dropdown 3: Crop aspect presets & Cover
                                      ExpansionTile(
                                        tilePadding: EdgeInsets.zero,
                                        title: const Text(
                                          'Crop aspect presets',
                                          style: TextStyle(fontSize: 13, fontWeight: FontWeight.w600),
                                        ),
                                        childrenPadding: const EdgeInsets.only(bottom: 8),
                                        expandedCrossAxisAlignment: CrossAxisAlignment.start,
                                        children: [
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
                                          const SizedBox(height: 10),
                                          OutlinedButton.icon(
                                            onPressed: () =>
                                                pickCustomCoverForMedia(context, ref, selected),
                                            icon: const Icon(Icons.photo_library_outlined, size: 16),
                                            label: const Text('Change Cover Image'),
                                          ),
                                          if (selected.type == MediaType.video) ...[
                                            const SizedBox(height: 8),
                                            OutlinedButton.icon(
                                              onPressed: () => ctrl.rerollVideoThumbnail(selected.id),
                                              icon: const Icon(Icons.refresh, size: 16),
                                              label: const Text('Re-roll First-Third Frame'),
                                            ),
                                            if (selected.durationMs > 0) ...[
                                              const SizedBox(height: 8),
                                              Text(
                                                'Frame time: ${formatDuration(selected.cover?.frameTimeMs ?? 0)}',
                                                style: const TextStyle(fontSize: 12),
                                              ),
                                              Slider(
                                                value: ((selected.cover?.frameTimeMs ?? 0)
                                                        .clamp(0, selected.durationMs))
                                                    .toDouble(),
                                                max: selected.durationMs.toDouble(),
                                                onChanged: (v) =>
                                                    ctrl.pickVideoFrame(selected.id, v.round()),
                                              ),
                                            ],
                                          ],
                                        ],
                                      ),
                                      if (ui.devices.any((d) => !d.isSelf)) ...[
                                        const SizedBox(height: 8),
                                        PopupMenuButton<String>(
                                          tooltip: 'Send directly to paired device',
                                          onSelected: (devId) =>
                                              ctrl.sendToDevice(selected.id, devId),
                                          itemBuilder: (_) => [
                                            for (final d in ui.devices.where((d) => !d.isSelf))
                                              PopupMenuItem(
                                                value: d.id,
                                                child: Text('Send to ${d.name}'),
                                              ),
                                          ],
                                          child: const IgnorePointer(
                                            child: OutlinedButton(
                                              onPressed: null,
                                              child: Row(
                                                mainAxisSize: MainAxisSize.min,
                                                children: [
                                                  Icon(Icons.send, size: 16),
                                                  SizedBox(width: 6),
                                                  Text('Send to Device'),
                                                ],
                                              ),
                                            ),
                                          ),
                                        ),
                                      ],
                                      const SizedBox(height: 8),
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

            // Openable Completed YouTube Download Bar at bottom
            const CompletedDownloadBar(),

            // Persistent Mini Music Player Bar when playing
            const MiniPlayerBar(),

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
                        builder: (context, _, __) => Row(
                          mainAxisSize: MainAxisSize.min,
                          children: [
                            InputChip(
                              avatar: const Icon(Icons.folder, size: 16),
                              label: Text(pin.label),
                              selected: ui.activeFolder == pin.path,
                              onPressed: () => ctrl.openFolder(pin.path),
                              onDeleted: () => ctrl.removePinnedFolder(pin.id),
                            ),
                            PopupMenuButton<String>(
                              tooltip: 'Manage shortcut',
                              icon: const Icon(Icons.more_vert, size: 14),
                              onSelected: (act) {
                                if (act == 'left') ctrl.movePinnedFolder(pin.id, -1);
                                if (act == 'right') ctrl.movePinnedFolder(pin.id, 1);
                                if (act == 'rename') _showRenamePinDialog(context, pin);
                              },
                              itemBuilder: (_) => const [
                                PopupMenuItem(value: 'rename', child: Text('Rename')),
                                PopupMenuItem(value: 'left', child: Text('Move Left')),
                                PopupMenuItem(value: 'right', child: Text('Move Right')),
                              ],
                            ),
                          ],
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
      ),
    );
  }

  void _showYouTubeModal(BuildContext context) {
    final s = ref.read(galleryControllerProvider).settings;
    final urlCtrl = TextEditingController();
    String format = s.youtubeDefaultFormat == 'mp3' ? 'mp3' : 'mp4';
    String quality = s.youtubeDefaultQuality.isNotEmpty
        ? s.youtubeDefaultQuality
        : (format == 'mp3' ? '320kbps' : '1080p');
    showDialog<void>(
      context: context,
      builder: (_) => StatefulBuilder(
        builder: (ctx, setModalState) => AlertDialog(
          title: const Text('Download from YouTube'),
          content: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              TextField(
                controller: urlCtrl,
                decoration: const InputDecoration(labelText: 'YouTube URL'),
              ),
              const SizedBox(height: 10),
              Row(
                children: [
                  ChoiceChip(
                    label: const Text('MP4'),
                    selected: format == 'mp4',
                    onSelected: (_) => setModalState(() {
                      format = 'mp4';
                      quality = '1080p';
                    }),
                  ),
                  const SizedBox(width: 8),
                  ChoiceChip(
                    label: const Text('MP3'),
                    selected: format == 'mp3',
                    onSelected: (_) => setModalState(() {
                      format = 'mp3';
                      quality = '320kbps';
                    }),
                  ),
                ],
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
                await ref.read(galleryControllerProvider.notifier).startYoutubeDownload(
                      url: urlCtrl.text,
                      format: format,
                      quality: quality,
                    );
                if (ctx.mounted) Navigator.of(ctx).pop();
              },
              child: const Text('Download'),
            ),
          ],
        ),
      ),
    );
  }

  void _showRenamePinDialog(BuildContext context, PinnedFolderItem pin) {
    final nameCtrl = TextEditingController(text: pin.label);
    showDialog<void>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('Rename Shortcut'),
        content: TextField(
          controller: nameCtrl,
          decoration: const InputDecoration(labelText: 'Shortcut Label'),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(ctx).pop(),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () async {
              await ref
                  .read(galleryControllerProvider.notifier)
                  .renamePinnedFolder(pin.id, nameCtrl.text);
              if (ctx.mounted) Navigator.of(ctx).pop();
            },
            child: const Text('Save'),
          ),
        ],
      ),
    );
  }
}

class _HomeOverviewSection extends ConsumerWidget {
  const _HomeOverviewSection({required this.ui});

  final GalleryUiState ui;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final ctrl = ref.read(galleryControllerProvider.notifier);
    final stats = ui.stats;

    return Padding(
      padding: const EdgeInsets.fromLTRB(14, 14, 14, 4),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Wrap(
            spacing: 8,
            runSpacing: 8,
            children: [
              ActionChip(
                avatar: const Icon(Icons.cleaning_services, size: 16),
                label: Text(
                  stats.duplicateGroups > 0
                      ? 'Clean ${stats.duplicateGroups} Duplicate Group(s) (${formatBytes(stats.duplicateExtraBytes)})'
                      : 'Storage Clean (${formatBytes(stats.totalBytes)})',
                ),
                onPressed: () => ctrl.openCleanerMode(duplicates: true),
              ),
              ActionChip(
                avatar: const Icon(Icons.data_usage, size: 16),
                label: Text('Large Files (${stats.largeFilesCount})'),
                onPressed: () => ctrl.openCleanerMode(largeFiles: true),
              ),
              ActionChip(
                avatar: const Icon(Icons.backup, size: 16),
                label: const Text('Pair & Backup'),
                onPressed: () => ctrl.selectSection(NavSection.devices),
              ),
            ],
          ),
          if (ui.recentItems.isNotEmpty) ...[
            const SizedBox(height: 12),
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Text('Recent Media', style: Theme.of(context).textTheme.titleSmall),
                TextButton(
                  onPressed: () => ctrl.selectSection(NavSection.recent),
                  child: const Text('See all'),
                ),
              ],
            ),
            SizedBox(
              height: 124,
              child: ListView.separated(
                scrollDirection: Axis.horizontal,
                itemCount: ui.recentItems.length,
                separatorBuilder: (_, __) => const SizedBox(width: 10),
                itemBuilder: (_, idx) {
                  final item = ui.recentItems[idx];
                  return SizedBox(
                    width: 150,
                    child: Card(
                      clipBehavior: Clip.antiAlias,
                      child: InkWell(
                        onTap: () => ctrl.selectMedia(item.id),
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Expanded(
                              child: MediaThumbnailWidget(
                                item: item,
                                cacheWidth: 260,
                                borderRadius: 0,
                              ),
                            ),
                            Padding(
                              padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 4),
                              child: Text(
                                item.alias,
                                maxLines: 1,
                                overflow: TextOverflow.ellipsis,
                                style: const TextStyle(fontSize: 11, fontWeight: FontWeight.w600),
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
    if (collapsed) {
      return Tooltip(
        message: label,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
          child: InkWell(
            borderRadius: BorderRadius.circular(8),
            onTap: onTap,
            child: Container(
              height: 44,
              alignment: Alignment.center,
              decoration: selected
                  ? BoxDecoration(
                      color: Theme.of(context).colorScheme.primaryContainer,
                      borderRadius: BorderRadius.circular(8),
                    )
                  : null,
              child: Icon(
                icon,
                color: selected ? Theme.of(context).colorScheme.primary : null,
              ),
            ),
          ),
        ),
      );
    }
    return ListTile(
      dense: true,
      selected: selected,
      leading: Icon(icon),
      title: Text(label, overflow: TextOverflow.ellipsis),
      onTap: onTap,
    );
  }
}
