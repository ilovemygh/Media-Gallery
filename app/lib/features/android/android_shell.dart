import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../../core/models/media_models.dart';
import '../../core/state/gallery_providers.dart';

/// PRD Section 10.8 Android mobile layout:
/// - BottomNavigationBar: Home, Library (Videos/Images/Audio tabs), YouTube, Devices
/// - Top bar: search, Connect status chip, YouTube menu, account avatar
/// - Details panel as bottom sheet + mini player
class AndroidShell extends ConsumerStatefulWidget {
  const AndroidShell({super.key});

  @override
  ConsumerState<AndroidShell> createState() => _AndroidShellState();
}

class _AndroidShellState extends ConsumerState<AndroidShell> {
  int _navIndex = 1;

  @override
  Widget build(BuildContext context) {
    final ui = ref.watch(galleryControllerProvider);
    final ctrl = ref.read(galleryControllerProvider.notifier);

    return Scaffold(
      appBar: AppBar(
        title: TextField(
          decoration: const InputDecoration(
            hintText: 'Search...',
            prefixIcon: Icon(Icons.search, size: 18),
            isDense: true,
          ),
          onChanged: ctrl.setSearchQuery,
        ),
        actions: [
          IconButton(
            icon: const Icon(Icons.wifi),
            onPressed: () => setState(() => _navIndex = 3),
          ),
          IconButton(
            icon: const Icon(Icons.account_circle),
            onPressed: ctrl.toggleGoogleSignIn,
          ),
        ],
      ),
      body: GridView.builder(
        padding: const EdgeInsets.all(10),
        gridDelegate: const SliverGridDelegateWithFixedCrossAxisCount(
          crossAxisCount: 2,
          mainAxisSpacing: 8,
          crossAxisSpacing: 8,
        ),
        itemCount: ui.items.length,
        itemBuilder: (context, idx) {
          final item = ui.items[idx];
          return Card(
            child: InkWell(
              onTap: () => _showDetailsSheet(context, item),
              child: Padding(
                padding: const EdgeInsets.all(8),
                child: Column(
                  mainAxisAlignment: MainAxisAlignment.center,
                  children: [
                    Icon(
                      item.type == MediaType.video
                          ? Icons.movie
                          : item.type == MediaType.audio
                              ? Icons.music_note
                              : Icons.image,
                      size: 36,
                    ),
                    const SizedBox(height: 8),
                    Text(item.alias, maxLines: 1, overflow: TextOverflow.ellipsis),
                  ],
                ),
              ),
            ),
          );
        },
      ),
      bottomNavigationBar: NavigationBar(
        selectedIndex: _navIndex,
        onDestinationSelected: (idx) {
          setState(() => _navIndex = idx);
          if (idx == 0) ctrl.selectSection(NavSection.home);
          if (idx == 1) ctrl.selectSection(NavSection.images);
          if (idx == 2) ctrl.selectSection(NavSection.youtube);
          if (idx == 3) ctrl.selectSection(NavSection.devices);
        },
        destinations: const [
          NavigationDestination(icon: Icon(Icons.home), label: 'Home'),
          NavigationDestination(icon: Icon(Icons.photo_library), label: 'Library'),
          NavigationDestination(icon: Icon(Icons.smart_display), label: 'YouTube'),
          NavigationDestination(icon: Icon(Icons.devices), label: 'Devices'),
        ],
      ),
    );
  }

  void _showDetailsSheet(BuildContext context, MediaItem item) {
    showModalBottomSheet<void>(
      context: context,
      builder: (_) => Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(item.alias, style: Theme.of(context).textTheme.titleMedium),
            Text('${item.fileName} · ${item.syncState.name}'),
            const SizedBox(height: 12),
            ElevatedButton.icon(
              onPressed: () {
                ref.read(galleryControllerProvider.notifier).playTrack(item);
                Navigator.of(context).pop();
              },
              icon: const Icon(Icons.share),
              label: const Text('Send with Media Gallery'),
            ),
          ],
        ),
      ),
    );
  }
}
