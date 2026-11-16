# Legacy Inventory & Migration Plan (`docs/MIGRATION.md`)

**Reference Directory:** `/legacy` (read-only snapshot of v0.1 build)  
**Target Architecture:** [`PRD-Media-Gallery-v0.2.md`](./PRD-Media-Gallery-v0.2.md) (Rust core workspace + Flutter UI via `flutter_rust_bridge` v2 + `iroh` QUIC + Kotlin Android plugins)

---

## 1. Legacy Stack, Build System & Dependencies

| Component | Legacy v0.1 Technology | Notes |
|---|---|---|
| Language | TypeScript `^5.8.3` + Kotlin (native Android stub) | Strict TypeScript (`ES2022`) |
| UI Framework | React `^19.1.0` + ReactDOM + CSS custom properties | Desktop layout + simulated Android mobile preview |
| Build / Bundler | Vite `^6.4.6` (`@vitejs/plugin-react`) | `npm --prefix legacy run build` |
| Desktop Wrapper | Electron (`legacy/electron/main.ts`, `preload.ts`) | Context-isolated IPC + `shell.trashItem` |
| Mobile Bridge | Capacitor (`legacy/android-bridge/capacitor.config.ts`) + Kotlin `MediaGalleryBridge.kt` | MediaStore cursor + `MediaMetadataRetriever` |
| Database | Node.js built-in `node:sqlite` (`DatabaseSync`) + SQLite FTS5 | WAL mode + `SAVEPOINT` nested transactions; browser fallback via `localStorage` |
| Networking | Node `ws` (`^8.18.2`), `node:https`, `node:dgram` (mDNS) | Self-signed TLS / WebSocket; no hole-punching or relay |
| Hashing | Node `node:crypto` SHA-256 | Full file & 64 KB start/mid/end partial hash |
| Test Runner | Vitest `^3.1.4` (`v3.2.7` resolved) | 8 test files, 22 tests (`npm --prefix legacy test`) |
| Icons | `lucide-react` `^0.511.0` | Vector icon set |

**How to run legacy side-by-side:**
```bash
npm --prefix legacy test
npm --prefix legacy run build
npm --prefix legacy run dev
```

---

## 2. Legacy Module Map

| Legacy File / Folder | Purpose |
|---|---|
| `legacy/src/core/types/domain.ts` | Domain types (`DeviceRecord`, `MediaRecord`, `CoverRecord`, `TransferRecord`, `PinnedFolderRecord`, `RenameLogRecord`, `DownloadJobRecord`, `SavedSearchRecord`, `AutoSyncRuleRecord`, `AppSettings`) |
| `legacy/src/core/database/schema.ts` | SQLite DDL (10 tables + `media_fts` FTS5 virtual table + `ai`/`au`/`ad` triggers) |
| `legacy/src/core/database/nodeSqliteAdapter.ts` | Synchronous `node:sqlite` wrapper with WAL + `SAVEPOINT` nesting |
| `legacy/src/core/database/repository.ts` | CRUD, FTS5 query builder (`buildFtsMatchQuery`), paged queries, folder 2×2 preview summaries, `exportDeltaSince`, `applyRemoteDelta` |
| `legacy/src/core/utils/hash.ts` | Streaming SHA-256 and 3×64 KB partial hash |
| `legacy/src/features/scanner/` | `mediaFormats.ts`, `scannerService.ts` (chunked `setImmediate` scan), `fileWatcher.ts` (`fs.watch`), `duplicateDetector.ts` |
| `legacy/src/features/alias/` | `aliasEngine.ts` (Sec 9 cleanup/templates), `offlineGeocoder.ts` (12-region Haversine gazetteer), `renameService.ts` (alias, undoable disk rename, bulk rename, folder move, `.media-gallery-trash`) |
| `legacy/src/features/thumbnails/` | `thumbnailService.ts` (first-third frame capture via `ffmpeg` CLI or deterministic SVG fallback + byte-limited LRU eviction), `coverEditorService.ts` (crop presets + audio tag art confirmation) |
| `legacy/src/features/search/searchService.ts` | FTS5 wrapper + `saved_search` CRUD |
| `legacy/src/features/connectivity/` | `transportInterface.ts`, `pairingService.ts` (6-digit PIN + QR + Ed25519 key + AES-256-GCM memory store), `wifiLanTransport.ts` (mDNS + WS), `usbTransport.ts` (ADB forward), `bluetoothTransport.ts`, `transportManager.ts`, `transferEngine.ts` (256 KB offset-safe chunk resume) |
| `legacy/src/features/sync/` | `indexSyncService.ts` (delta + base64 thumb blobs, offline check, LAN stream URL), `googleCloudSync.ts` (OAuth session + Drive `appDataFolder` interface + H2 wipe) |
| `legacy/src/features/player/musicPlayerService.ts` | De-duplicated PC+Phone music library, artist/album/folder/device grouping, queue/shuffle/repeat |
| `legacy/src/features/youtube/` | `youtubeConstants.ts`, `youtubeService.ts` (`yt-dlp` arg builder, spawn queue, `-U` updater, auto-ingest) |
| `legacy/src/ui/` | Desktop React shell (`LeftSidebar`, `TopBar`, `MainGalleryArea`, `RightDetailsSidebar`, `BottomShortcutBar`, `MusicPlayerBar`, `Modals`, `FeaturePanels`, `AndroidMobileShell`, `browserEngine.ts`) |
| `legacy/electron/` | Electron `main.ts`, `preload.ts`, ambient `electron-env.d.ts` |
| `legacy/android-bridge/` | Capacitor config, MediaStore TS mapper, Kotlin `MediaGalleryBridge.kt` |

---

## 3. Feature Parity Matrix

| PRD ID | Feature Description | Legacy Status | Legacy Gap / Note | Target Rust Crate / Flutter Feature |
|---|---|---|---|---|
| **A1** | Scan all 16 media formats (`mp4..heic`) | **Works** | Metadata probing (`exif`/`ffprobe`) is via pluggable hook rather than built-in parser | `core-index::scanner`, `core-media::metadata` |
| **A2** | Incremental rescan (`notify` / MediaStore) | **Works** | Uses `fs.watch` on desktop; Kotlin `MediaGalleryBridge` has query helper only | `core-platform::watcher`, `core-index`, `android-plugins/mediastore` |
| **A3** | Non-blocking background scan + progress | **Works** | Single-threaded JS `setImmediate` batching | `core-index` (`tokio` + `rayon` + FRB progress stream) |
| **A4** | Include / exclude scan folders | **Works** | Path prefix filter in `scannerService.ts` | `core-index::scanner` |
| **A5** | Duplicate detection (size + partial hash, flag only) | **Works** | Uses SHA-256 partial hash; v0.2 upgrades to **BLAKE3** | `core-index::duplicates` (`blake3`) |
| **B1–B5** | Videos/Images/Audio views, 2×2 folder mosaics, S/M/L/list, sort, group | **Works** | React DOM virtualizes only via CSS grid (no true sliver virtualization for 100k UI) | `app/lib/features/gallery/` (Flutter `CustomScrollView` + `SliverGrid`) |
| **B6** | Built-in image viewer, video player, audio player | **Partial** | Browser `<video>` stub without local file codec / `libmpv` | `app/lib/features/viewer/` (`media_kit` + image viewer) |
| **C1** | Instant FTS5 search (`name`, `alias`, `date`, `place`, `device`) | **Works** | Real SQLite FTS5 in Node; browser preview uses JS string filter | `core-index::search` (always real `rusqlite` FTS5) |
| **C2** | Auto-alias (`{date} {place} {name}`, offline GPS, audio/YT tags) | **Works** | 12-entry offline gazetteer | `core-media::alias` + offline city dataset |
| **C3–C4** | Non-destructive alias, undoable real rename, bulk pattern rename | **Works** | Tested on disk via `RenameService` | `core-media::rename` |
| **C5–C6** | Faceted filters + saved searches | **Works** | Implemented in SQL + `saved_search` table | `core-index::search`, `app/lib/features/search/` |
| **D1–D3** | QR/PIN pairing, LAN discovery, bidirectional send | **Works** | Local mDNS + WS; Electron `main.ts` chunk handler is a stub | `core-net` (`iroh` + mDNS + `ctrl/1` + `files/1`) |
| **D4** | Wired USB mode (ADB reverse / tethering) | **Partial** | `adb forward` helper only; no USB tethering interface probe | `core-net::usb`, `android-plugins/usb` |
| **D5** | Bluetooth fallback + slow warning | **Partial** | Interface + UI warning implemented; requires OS RFCOMM bridge | `core-net::bluetooth`, `android-plugins/bluetooth` |
| **D6** | Auto transport priority + manual override | **Works** | `TransportManager` selects `wired > wifi_lan > wifi_direct > bluetooth` | `core-net::path_selector` (adds hole-punch & relay tiers) |
| **D7** | Chunked, resumable, hash-verified transfer queue | **Works** | SHA-256 verified chunked WS transfer; v0.2 mandates **BLAKE3** | `core-net::transfer` (`blake3`) |
| **D8** | Connection status chip in top bar | **Works** | Implemented in `TopBar.tsx` | `app/lib/features/shell/top_bar.dart` |
| **D9** | Cross-network hole punching & relay fallback | **Missing** | Explicitly out of scope in v0.1 | `core-net` (`iroh` QUIC hole punching + relay fallback) |
| **E1–E6** | Serverless index sync, cloud badge, on-demand pull, LAN stream, offline guard, auto-sync | **Works** | Protocol & DB sync implemented; streaming endpoint URL constructed without HTTP range server | `core-sync`, `core-net` (`index/1`, `files/1`) |
| **F1–F5** | Music player, de-duplicated PC+Phone tracks, grouping, media keys, YT auto-add | **Partial** | State machine & UI complete; browser lacks real multi-format `<audio>` decoder pipeline & Android `audio_service` | `core-api::player`, `app/lib/features/player/` (`media_kit` + `audio_service`) |
| **G1–G6** | YouTube `yt-dlp` popup, queue, playlists, updater, dropdown, Upload entry | **Partial** | `yt-dlp` runner works on desktop; Android `youtubedl-android` only an interface stub | `core-ytdl`, `android-plugins/ytdlp` |
| **H1–H2** | Optional Google Sign-In & logout token/cache wipe | **Partial** | OAuth token store + Drive `appDataFolder` trait interface; real HTTP Google OAuth client not wired | `core-sync::google`, `app/lib/features/auth/` |
| **I1–I4** | Fixed random first-third video frame, re-roll/pick, audio cover edit, `4:3`/`9:16`/`16:9`/`1:1`/`custom` crop | **Works** | Audio `ffmpeg` embed ignores `tempOut` rename-back on completion | `core-media::thumbnails`, `core-media::covers` (`lofty`) |
| **J1–J4** | Drag & drop (in-app + OS), ~700 ms spring-loaded folders, bottom pinned bar, drop-on-device | **Works** | Implemented across `LeftSidebar`, `MainGalleryArea`, `BottomShortcutBar`, `FeaturePanels` | `app/lib/features/shell/` (`DragTarget` + `desktop_drop`) |

---

## 4. Legacy Data Inventory

1. **SQLite Schema (`legacy/src/core/database/schema.ts`):**
   - Tables: `device`, `media`, `media_fts` (FTS5 `unicode61`), `cover`, `transfer`, `pinned_folder`, `rename_log`, `download_job`, `settings`, `saved_search`, `auto_sync_rule`.
   - Desktop DB location in `electron/main.ts`: `<app.getPath('userData')>/media-gallery.db` (WAL mode).
   - Thumbnail cache location: `<app.getPath('userData')>/thumbnails/<mediaId>-<frameTimeMs>.{jpg,svg}` (and custom covers `<mediaId>-custom-<ts>.<ext>`).
   - Trash staging directory (headless fallback): `<folder>/.media-gallery-trash/<ts>-<fileName>`.
2. **Browser / Preview Storage (`legacy/src/ui/state/browserEngine.ts`):**
   - `window.localStorage['media_gallery_state_v1']` storing JSON `{ devices, media, covers, transfers, pinnedFolders, renameLogs, downloadJobs, savedSearches, autoSyncRules, settings, googleAccountEmail, activeTransport }`.
3. **Auth & Pairing Tokens (`legacy/src/features/connectivity/pairingService.ts`):**
   - Stored in `EncryptedMemoryTokenStore` (AES-256-GCM ephemeral key in memory) using keys `peer:<deviceId>:auth`, `peer:<deviceId>:pubkey`, `google:oauth:session`. Because ephemeral in-memory keys do not survive process restarts in Node headless mode, any legacy token entries require re-authentication during migration (never copied in plain text).

---

## 5. Undocumented Legacy Behaviors (Preserved for Parity)

1. `saved_search` and `auto_sync_rule` exist as first-class SQLite tables rather than settings JSON blobs.
2. Soft-delete tracks `is_trashed INTEGER` and `trashed_path TEXT` separately from `sync_state` (`Local`/`Remote`/`Downloading`/`Synced`/`Error`) so trashed files retain restore metadata without polluting `SyncState`.
3. `rename_log` includes a `batch_id TEXT` column enabling one-click atomic undo of an entire bulk rename operation (`undoBatchRename`).
4. `ThumbnailService` falls back to generating a deterministic `<mediaId>-<frameTimeMs>.svg` visual placeholder when `ffmpeg` is absent so pipelines never fail on headless systems.
5. `DuplicateDetector` skips hidden dot-directories and `.media-gallery-trash` during scans.

---

## 6. Known Legacy Bugs & Performance Problems

1. **Bug — Browser/Electron disconnect:** The Vite web preview reads `localStorage` (`browserEngine.ts`) instead of querying the Electron `ipcMain` SQLite DB via `window.electronAPI`; Electron's `WifiLanTransport.startServer` `onChunkReceived` handler is a no-op stub.
2. **Bug — Audio cover embedding overwrite:** `coverEditorService.ts` writes `ffmpeg` output to `${media.path}.tmp${ext}` without atomically moving `tempOut` back onto `media.path`.
3. **Bug — Name sort tie-breaking:** When `cleanFileNameForAlias` strips everything (e.g., `IMG_01.jpg`, `IMG_02.jpg`) on same-day photos without GPS, `auto_alias` collapses to `{date}`, making `SORT_COLUMNS.name` tie on date unless secondary sort falls back to `file_name`.
4. **Bug — In-memory ephemeral secret store:** `EncryptedMemoryTokenStore` generates a random AES key per run, losing paired tokens across Electron restarts instead of persisting in OS Credential Manager / Keystore.
5. **Performance — Single-threaded JS scanning & no real virtualization in React list/grid:** Scanning hashes sequentially in JS (`~870 ms` per 1,000 tiny files); 100k items in DOM would bloat layout without virtualized sliver rendering.
6. **Baseline Legacy Numbers (measured on host via `npm --prefix legacy test` & benchmark runner):**
   - **Scan 1,000 fixture files** (partial SHA-256 + auto-alias + SQLite FTS5): `869.76 ms`
   - **Batch insert 10,000 mixed records + FTS5 triggers:** `1,701.13 ms` (full test case `1,950 ms`)
   - **FTS5 `"2026 beach"` query (over 11,000 rows):** `1.38 ms`
   - **Time to first thumbnail (SVG fallback):** `11.18 ms`
   - **Peak benchmark RSS memory:** `82.8 MB`
