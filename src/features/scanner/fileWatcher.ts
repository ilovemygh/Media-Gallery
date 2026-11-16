import { watch, type FSWatcher } from 'node:fs';
import path from 'node:path';
import type { MediaRepository } from '../../core/database/repository.ts';
import { ScannerService, type ScanOptions } from './scannerService.ts';

/**
 * Incremental filesystem watcher using native `fs.watch` with per-path debouncing (PRD A2).
 */
export class LibraryFileWatcher {
  private readonly watchers: FSWatcher[] = [];
  private readonly pendingTimers = new Map<string, NodeJS.Timeout>();
  private readonly scanner: ScannerService;

  constructor(
    repo: MediaRepository,
    private readonly options: Omit<ScanOptions, 'roots'>,
    private readonly debounceMs = 150,
  ) {
    this.scanner = new ScannerService(repo);
  }

  start(roots: string[]): void {
    this.stop();
    for (const root of roots) {
      try {
        const watcher = watch(root, { recursive: true }, (_eventType, filename) => {
          if (!filename) return;
          const fullPath = path.join(root, filename.toString());
          const existingTimer = this.pendingTimers.get(fullPath);
          if (existingTimer) clearTimeout(existingTimer);
          const timer = setTimeout(() => {
            this.pendingTimers.delete(fullPath);
            void this.scanner.indexSingleFile(fullPath, this.options);
          }, this.debounceMs);
          this.pendingTimers.set(fullPath, timer);
        });
        this.watchers.push(watcher);
      } catch {
        // Ignore inaccessible roots cleanly.
      }
    }
  }

  stop(): void {
    for (const timer of this.pendingTimers.values()) {
      clearTimeout(timer);
    }
    this.pendingTimers.clear();
    for (const watcher of this.watchers) {
      watcher.close();
    }
    this.watchers.length = 0;
  }
}
