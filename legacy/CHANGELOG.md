# Changelog & PRD Requirement Checklist (`CHANGELOG.md`)

All milestones (`M0`–`M7`) from [`PRD-Media-Gallery.md`](./PRD-Media-Gallery.md) have been implemented and verified.

---

## Milestones

| Milestone | Scope | Status |
|---|---|---|
| **M0** | Project setup, SQLite FTS5 schema & repository, Desktop UI shell (sidebar, top bar, bottom bar, dark/light themes) | **Done** |
| **M1** | Background scanner, incremental watcher, duplicate detection, FTS5 search, auto-alias + offline geocoding, undoable rename & trash, fixed first-third video thumbnails, cover aspect crop editor, 700ms spring-loaded drag-and-drop | **Done** |
| **M2** | Unified PC + Phone de-duplicated music player, Android MediaStore & `MediaMetadataRetriever` native bridge, Android mobile layout (bottom nav, tabs, bottom-sheet details, mini/full player, share-sheet hook) | **Done** |
| **M3** | QR & 6-digit PIN pairing, Ed25519 key pinning, encrypted token store, mDNS + HTTP/TLS WebSocket LAN transport, resumable SHA-256 verified chunked transfers | **Done** |
| **M4** | Serverless index delta sync (`Remote`/`Local`/`Synced`), cloud badge + on-demand download, offline-owner disabled download state, LAN preview URL, optional Google Drive `appDataFolder` sync & H2 logout wipe | **Done** |
| **M5** | YouTube downloader (`yt-dlp` + `ffmpeg`, MP4/MP3 quality, playlists, updater, auto-library & music ingestion, dropdown actions, compliance notice) | **Done** |
| **M6** | Wired USB (ADB port-forward / tethering + MTP note), Bluetooth fallback with slow-speed warning, automatic transport selection (`wired > wifi_lan > wifi_direct > bluetooth`) + manual override | **Done** |
| **M7** | Bulk pattern rename + batch undo, auto-sync rules UI, duplicate review UI, Electron desktop wrapper, 10,000-record indexing/search benchmark, `CHANGELOG.md`, `README.md` | **Done** |

---

## PRD Requirement Matrix

| ID | Priority | Requirement Summary | Status | Primary Implementation |
|---|---|---|---|---|
| **A1** | P0 | Scan all media formats (`mp4, mkv, webm, mov, avi, mp3, m4a, flac, wav, ogg, png, jpg, jpeg, gif, webp, heic`) | **Done** | `src/features/scanner/mediaFormats.ts`, `scannerService.ts` |
| **A2** | P0 | Incremental rescan via file watchers / MediaStore change notifications | **Done** | `src/features/scanner/fileWatcher.ts`, `android-bridge/MediaGalleryPlugin.ts` |
| **A3** | P0 | Background non-blocking scan with progress updates | **Done** | `src/features/scanner/scannerService.ts` |
| **A4** | P1 | Include / exclude scan folders | **Done** | `src/features/scanner/scannerService.ts`, `SettingsView` |
| **A5** | P1 | Duplicate detection by size + partial hash (flag only) | **Done** | `src/features/scanner/duplicateDetector.ts`, `DuplicatesView` |
| **B1** | P0 | Three main views: Videos, Images, Audio | **Done** | `src/ui/components/LeftSidebar.tsx`, `MainGalleryArea.tsx` |
| **B2** | P0 | Folder-aware visual browsing (2x2 thumbnail mosaics) | **Done** | `MainGalleryArea.tsx`, `repository.ts` |
| **B3** | P0 | Grid sizes (small / medium / large) + list view | **Done** | `MainGalleryArea.tsx` |
| **B4** | P0 | Sort by name, date, size, duration, type | **Done** | `repository.ts`, `MainGalleryArea.tsx` |
| **B5** | P1 | Group by date, folder, or device | **Done** | `MainGalleryArea.tsx` |
| **B6** | P0 | Built-in image, video, and audio viewers/players | **Done** | `Modals.tsx`, `MusicPlayerBar.tsx` |
| **C1** | P0 | Instant search by name, alias, date/time, type, folder, device | **Done** | `schema.ts` (`media_fts` FTS5), `searchService.ts` |
| **C2** | P0 | Auto-alias generation (filename cleanup, EXIF/date, offline GPS, tags) | **Done** | `src/features/alias/aliasEngine.ts`, `offlineGeocoder.ts` |
| **C3** | P0 | Single-file alias edit & undoable real rename | **Done** | `src/features/alias/renameService.ts`, `RightDetailsSidebar.tsx` |
| **C4** | P1 | Bulk pattern rename with live preview and undo | **Done** | `renameService.ts`, `BulkRenameModal` |
| **C5** | P1 | Multi-faceted filters (type, date, time of day, size, duration, device, sync state) | **Done** | `repository.ts`, `App.tsx` |
| **C6** | P1 | Saved searches / smart folders | **Done** | `searchService.ts`, `repository.ts` |
| **D1** | P0 | QR code or 6-digit PIN one-time pairing | **Done** | `pairingService.ts`, `ConnectPairModal` |
| **D2** | P0 | Auto-discover paired devices on LAN (mDNS) | **Done** | `wifiLanTransport.ts` |
| **D3** | P0 | Bidirectional PC <-> Phone transfer (drag/drop & share) | **Done** | `DevicesAndTransfersView`, `AndroidMobileShell.tsx` |
| **D4** | P1 | Wired USB mode (ADB port-forward / tethering + MTP fallback) | **Done** | `src/features/connectivity/usbTransport.ts` |
| **D5** | P1 | Bluetooth fallback mode with UI slow-speed warning | **Done** | `src/features/connectivity/bluetoothTransport.ts`, `Modals.tsx` |
| **D6** | P0 | Automatic transport choice + manual override | **Done** | `src/features/connectivity/transportManager.ts` |
| **D7** | P0 | Chunked, resumable, SHA-256 verified transfer queue | **Done** | `src/features/connectivity/transferEngine.ts` |
| **D8** | P1 | Top bar Connect button / live status chip | **Done** | `src/ui/components/TopBar.tsx` |
| **E1** | P0 | Serverless index sync (metadata + thumbnails only) | **Done** | `src/features/sync/indexSyncService.ts` |
| **E2** | P0 | Remote items show cloud badge + Download/Sync on demand | **Done** | `MainGalleryArea.tsx`, `RightDetailsSidebar.tsx` |
| **E3** | P1 | Auto-sync rules | **Done** | `indexSyncService.ts`, `DevicesAndTransfersView` |
| **E4** | P1 | Stream/preview remote files over LAN without saving | **Done** | `indexSyncService.ts` (`getLanStreamPreviewUrl`) |
| **E5** | P0 | Works fully on LAN without Google login | **Done** | `wifiLanTransport.ts`, `indexSyncService.ts` |
| **E6** | P0 | Offline owner browsing via cached thumbnails with disabled Download | **Done** | `indexSyncService.ts`, `MainGalleryArea.tsx` |
| **F1** | P0 | Full audio playback controls & queue | **Done** | `musicPlayerService.ts`, `MusicPlayerBar.tsx` |
| **F2** | P0 | Unified de-duplicated PC + Phone music library | **Done** | `musicPlayerService.ts`, `UnifiedAudioView` |
| **F3** | P1 | Group by artist, album, folder, device | **Done** | `musicPlayerService.ts`, `UnifiedAudioView` |
| **F4** | P1 | Media keys (MediaSession) & Android foreground playback hooks | **Done** | `MusicPlayerBar.tsx`, `MediaGalleryPlugin.ts` |
| **F5** | P0 | YouTube MP3 downloads auto-added to music library | **Done** | `youtubeService.ts`, `App.tsx` |
| **G1–G6** | P0–P2 | YouTube download popup (`yt-dlp`), cover tagging, queue, playlist, updater, dropdown (`Upload`, `Download`, `Your channel`, `YouTube Studio`) | **Done** | `youtubeService.ts`, `TopBar.tsx`, `YouTubeDownloadModal`, `YouTubeWorkspaceView` |
| **H1–H2** | P1 | Optional Google Sign-In + logout token & cloud cache wipe | **Done** | `googleCloudSync.ts`, `TopBar.tsx` |
| **I1–I4** | P0–P1 | Fixed random first-third video frame, re-roll/pick/custom cover, audio cover confirmation, aspect crop (`4:3`, `9:16`, `16:9`, `1:1`, `custom`) | **Done** | `thumbnailService.ts`, `coverEditorService.ts`, `RightDetailsSidebar.tsx` |
| **J1–J4** | P0–P1 | Drag-and-drop folders/OS, 700ms spring-loaded folders (grid/sidebar/bottom bar), customizable pinned bottom bar, drop-on-device transfer | **Done** | `LeftSidebar.tsx`, `MainGalleryArea.tsx`, `BottomShortcutBar.tsx`, `FeaturePanels.tsx` |
