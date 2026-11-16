# PRD: Media Gallery v0.2 (Rust core + Flutter UI)

**Working title:** Media Gallery (alternatives: Media Finder, Media Manager)
**Version:** 0.2 (Draft). Supersedes v0.1.
**Platforms:** Windows first (macOS/Linux later, same codebase) + Android
**Stack decision:** Rust core + Flutter UI, bridged with `flutter_rust_bridge` (FRB) v2
**Purpose of this document:** Define the product and the technical approach precisely enough that an engineering agent can rebuild it and **migrate from the existing (legacy) build** without guessing.

---

## 1. Summary

Media Gallery scans every media file on a phone and a PC, shows them **visually** (thumbnails, covers, folder previews), and makes them easy to find, rename, move, and play.

It works like a **"serverless Google Drive"**: the user's own phone and PC are the storage. The cloud holds, at most, a small metadata index and thumbnails, never file contents. Files move directly between devices over Wi-Fi/LAN, the internet (hole-punched, with relay fallback), USB, or Bluetooth.

It includes a built-in **YouTube downloader** (yt-dlp), a **music player** that merges phone and PC music, and an optional **Google/YouTube account link**.

## 2. What changed from v0.1

1. **Tech stack is decided:** a **Rust core** does scanning, indexing, hashing, thumbnails, networking, sync, and job orchestration. **Flutter** is the UI for both desktop and Android.
2. **Networking is built on QUIC peer-to-peer (iroh)**: devices are dialed by key, not IP. Hole punching with relay fallback makes **sync across different networks possible in v1** (it was v2 in v0.1).
3. **New section: Migration** from the legacy build (section 16).
4. **New sections:** repo layout, FFI contract, Android native layer, testing gates.
5. Requirement IDs (A1, B2, ...) are unchanged from v0.1 where possible so existing notes still map.

## 3. Goals and non-goals

### Goals
1. One visual library of all media from every connected device.
2. Fast browsing at scale (100,000+ files): thumbnails first, filenames second.
3. Easy alias/rename, filter, and search (including by date and time).
4. Direct device-to-device transfer and sync, no file contents in any cloud.
5. Built-in music player merging phone and PC music.
6. Built-in YouTube download (MP4 / MP3) saved straight into the library.
7. Simple, familiar UI: sidebar, top bar, details panel, drag and drop.
8. **Feature parity with the legacy build, then improvement** (see section 16).

### Non-goals (v1)
- iOS app.
- Storing file contents in any cloud.
- Video/audio editing (only cover/thumbnail editing).
- Streaming from YouTube (download only).
- Sharing with other people / social features.

---

## 4. Architecture

```
+--------------------------------------------------------------+
|                      Flutter UI (Dart)                       |
|  screens, state (Riverpod), virtualized grids, player, crop  |
+------------------------------+-------------------------------+
                               |  flutter_rust_bridge v2 (FFI)
                               |  coarse-grained async API + streams
+------------------------------v-------------------------------+
|                     Rust core (workspace)                    |
|  core-api  (the ONLY crate exposed to Dart)                  |
|  core-index   scanner, watcher, SQLite, FTS5 search          |
|  core-media   thumbnails, EXIF/ID3/video meta, covers, alias |
|  core-net     iroh endpoint, pairing, transports, transfers  |
|  core-sync    index deltas, remote items, sync states        |
|  core-ytdl    yt-dlp job queue + post-processing             |
|  core-platform  trait layer for OS-specific services         |
+------------------------------+-------------------------------+
                               |  traits implemented per OS
        +----------------------+-----------------------+
        |                                              |
+-------v--------+                           +---------v---------+
| Desktop impls  |                           | Android (Kotlin)  |
| notify watcher |                           | MediaStore reader |
| ffmpeg sidecar |                           | Bluetooth, USB    |
| yt-dlp sidecar |                           | foreground svc    |
+----------------+                           | youtubedl-android |
                                             +-------------------+
```

**Principles**
- Rust owns all heavy work and all state of record (SQLite). Flutter is a view layer.
- Every OS-specific capability sits behind a trait in `core-platform` so the core stays testable and portable.
- Each device is authoritative for its own files and index records (no merge conflicts).
- Offline-first. Everything on the LAN works with no internet and no Google account.

## 5. Technology stack

| Area | Choice | Notes |
|---|---|---|
| Core language | Rust (stable, 2021/2024 edition) | Workspace with several crates |
| UI | Flutter (Dart) | One codebase: Android + Windows (macOS/Linux later) |
| Bridge | `flutter_rust_bridge` v2 | Generated bindings; mock bridge for UI tests |
| Async runtime | `tokio` | Single runtime owned by the core |
| Database | SQLite via `rusqlite` (bundled) + FTS5 | WAL mode; migrations versioned |
| File scan | `walkdir`/`jwalk` + `rayon`; `notify` for watching | Desktop. Android uses MediaStore (see section 8) |
| Hashing | `blake3` | Content identity and transfer verification |
| P2P networking | `iroh` (QUIC, key-based dialing, hole punching, relay fallback) | Verify Android build in the M0 spike |
| Local discovery | iroh local-network discovery (mDNS) | Verify in the M0 spike |
| Image decode/resize | `image` + `fast_image_resize`; HEIC via libheif if licensing allows | Output WebP/JPEG thumbnails to disk |
| Video thumbnails/meta | ffmpeg/ffprobe as a **sidecar executable** (LGPL build) on desktop; `MediaMetadataRetriever` on Android | Sidecar avoids static-linking licence issues |
| Audio tags/covers | `lofty` | Read/write ID3, MP4, FLAC, Ogg |
| EXIF | `nom-exif` or `kamadak-exif` | Dates, GPS |
| Reverse geocoding | Offline city-level dataset (e.g. `reverse_geocoder`) | Online lookup is opt-in only |
| Media playback | `media_kit` (libmpv) in Flutter; `audio_service` on Android | Playback stays in Flutter, not Rust |
| Flutter state | Riverpod | Suggested; agent may justify an alternative |
| Drag and drop | Flutter `DragTarget`/`Draggable` in-app; `desktop_drop` for OS files | Spring-load timer implemented in Dart |
| YouTube download | yt-dlp (+ ffmpeg). Desktop: sidecar. Android: `youtubedl-android` via Kotlin plugin | Self-updating yt-dlp |
| Google | OAuth (Sign-In), Drive `appDataFolder`, YouTube Data API v3 | Optional feature |

**Licence audit is part of M0:** list the licence of every dependency and sidecar (iroh, ffmpeg build, libmpv/media_kit, libheif, yt-dlp, Flutter plugins) and flag anything incompatible with distributing a proprietary app.

## 6. Repository layout

```
/app                      Flutter app (lib/features/*, lib/core/*)
/crates
  core-api/               FRB-exposed facade (only crate Dart sees)
  core-index/  core-media/  core-net/  core-sync/  core-ytdl/
  core-platform/          traits + desktop implementations
/android-plugins          Kotlin: mediastore, bluetooth, usb, fgservice, ytdlp
/legacy                   Read-only copy of the previous build (reference only)
/docs                     PRD, DECISIONS.md, MIGRATION.md, API.md
/tools                    codegen scripts, fixture generators, benchmarks
Cargo.toml (workspace)    + rust-toolchain.toml + CI config
```

## 7. FFI contract rules (Rust <-> Flutter)

1. **Coarse-grained, async API.** No chatty per-item calls. Queries are paged (`list_media(filter, sort, cursor, limit)`).
2. **Streams for events:** scan progress, transfer progress, download progress, device status, sync-state changes.
3. **Thumbnails by path, not bytes.** Rust writes thumbnails to the cache directory; Flutter loads them as files (`Image.file`) with its own memory cache.
4. **Never block the UI isolate.** All heavy work runs on Rust's tokio/rayon pools.
5. **Errors are typed** (enum with codes + human message), never panics across FFI.
6. **Cancellation** supported for scans, transfers, downloads, thumbnail batches.
7. **Workflow:** edit Rust API, run codegen, rebuild. CI fails if generated bindings are out of date.
8. A **mock bridge** exists so Flutter UI tests run without native binaries.

## 8. Android native layer (Kotlin plugins)

Rust cannot reach Android system APIs conveniently, so thin Kotlin plugins expose them through platform channels, implementing the `core-platform` traits:

- **MediaStore reader:** enumerates images/video/audio (with change notifications) and streams batches into the Rust index. Avoid `MANAGE_EXTERNAL_STORAGE` unless MediaStore/SAF cannot cover a required case.
- **Foreground service:** keeps transfers, downloads, and scans alive; shows a notification.
- **Bluetooth** transport (fallback, small files) and **USB** (accessory/ADB/tethering detection).
- **yt-dlp:** wraps `youtubedl-android`; Rust's `core-ytdl` orchestrates jobs and receives progress via the plugin.
- **Share-sheet** intent ("Send with Media Gallery").
- Cover/thumbnail helpers where the OS APIs are faster than Rust.

Android 10+ restricts executing downloaded binaries; do not rely on a yt-dlp executable sidecar on Android.

---

## 9. Functional requirements

Priority: **P0** = MVP, **P1** = important, **P2** = later.

### A. Scanning and library
- **A1 (P0)** Scan all storage and build the Media Index. Types: mp4, mkv, webm, mov, avi, mp3, m4a, flac, wav, ogg, png, jpg/jpeg, gif, webp, heic.
- **A2 (P0)** Incremental rescan via file watcher (desktop) / MediaStore changes (Android).
- **A3 (P0)** Background scan with progress; never blocks the UI.
- **A4 (P1)** Include/exclude folders.
- **A5 (P1)** Duplicate detection (size + partial hash). Flag only; never auto-delete.

### B. Gallery views
- **B1 (P0)** Views: **Videos**, **Images**, **Audio**.
- **B2 (P0)** Folder-aware browsing: folders show contents as thumbnails/covers.
- **B3 (P0)** Grid sizes (S/M/L) and list view.
- **B4 (P0)** Sort by name, date, size, duration, type.
- **B5 (P1)** Group by date, folder, device.
- **B6 (P0)** Built-in image viewer, video player, audio player.

### C. Search, filter, alias
- **C1 (P0)** Search by name, alias, date/time, type, folder, device (FTS5, results as you type).
- **C2 (P0)** Auto-alias for every file from file name, date, location (section 13). Search matches aliases and real names.
- **C3 (P0)** Single rename: edit alias, or apply as real rename.
- **C4 (P1)** Bulk rename with pattern (`{date} {place} {name}`), live preview, **undo**.
- **C5 (P1)** Filters: type, date range, time of day, size, duration, resolution, device, sync state.
- **C6 (P1)** Saved searches / smart folders.

### D. Connectivity and transfer
- **D1 (P0)** Pair phone and PC with a **QR code or 6-digit PIN** (exchanges device keys).
- **D2 (P0)** Auto-discover paired devices on the same network.
- **D3 (P0)** Send files PC to phone and phone to PC (drag and drop on PC; share/select on phone).
- **D4 (P1)** **Wired** mode over USB (ADB reverse / USB tethering gives an IP path used by the same protocol).
- **D5 (P2)** **Bluetooth** transport (slow; small files, pairing fallback). Warn the user it is slow.
- **D6 (P0)** **Automatic path choice:** direct LAN/wired > hole-punched internet path > relay; Bluetooth only when nothing else works. Manual override in settings.
- **D7 (P0)** Transfers are chunked, resumable, **hash-verified (BLAKE3)**, with queue, progress, speed, pause, cancel.
- **D8 (P1)** Connection status always visible (Connect button becomes a status chip).
- **D9 (P1)** **Cross-network access:** devices on different networks (home PC, phone on mobile data) connect via key with hole punching / relay fallback.

### E. Sync ("serverless drive")
- **E1 (P0)** Each device publishes its Media Index (metadata + thumbnails only); other devices browse it **without downloading files**.
- **E2 (P0)** Remote items show thumbnail + cloud badge; **Download / Sync** pulls the real file on demand.
- **E3 (P1)** Auto-sync rules (e.g. keep Camera photos synced to PC).
- **E4 (P1)** Preview/stream a remote file without saving it.
- **E5 (P0)** Works **without a Google account**; the account only adds an offline index cache and YouTube features.
- **E6 (P0)** If the owning device is unreachable, its items stay browsable (cached thumbnails) but Download is disabled with a clear message.

### F. Music player
- **F1 (P0)** Play, pause, seek, next/prev, shuffle, repeat, volume, queue.
- **F2 (P0)** Unified library merging phone and PC tracks (de-duplicated view, source device shown).
- **F3 (P1)** Group by artist, album, folder, device; playlists.
- **F4 (P1)** Background playback + notification/lock-screen controls (Android); media keys (PC).
- **F5 (P0)** YouTube-downloaded tracks appear automatically.

### G. YouTube
- **G1 (P0)** **Download from YouTube** popup: link, MP4/MP3, quality, destination device/folder, then yt-dlp.
- **G2 (P0)** Auto-save to the device library with title and thumbnail as cover.
- **G3 (P1)** Queue with progress/errors; playlist links.
- **G4 (P1)** Optional Google login also connects the YouTube account.
- **G5 (P1)** Dropdown items **Your channel** and **YouTube Studio** open in browser.
- **G6 (P2)** **Upload** entry (open YouTube upload page, or API upload).

### H. Account
- **H1 (P1)** Optional Google login. **H2 (P1)** Logout removes tokens and cached cloud index.

### I. Cover and thumbnail editing
- **I1 (P0)** Video thumbnail = frame at a **random point in the first third** of the video; once chosen it is **saved to cache and synced, and stays fixed**.
- **I2 (P1)** Re-roll, pick a frame, or use a custom image.
- **I3 (P0)** Audio cover edit in the right sidebar: replace, remove, restore (writes embedded art after confirmation).
- **I4 (P0)** Crop with aspect presets: **4:3, 9:16, 16:9, 1:1, custom**.

### J. Folders, drag and drop, shortcuts
- **J1 (P0)** Drag and drop between folders and to/from the OS.
- **J2 (P0)** **Spring-loaded folders:** hovering a dragged item over a folder (about 700 ms) opens it, including folders in the left sidebar.
- **J3 (P0)** Pin folders as shortcuts in a **bottom bar** (add, remove, reorder, rename).
- **J4 (P1)** Drop a file on a device chip to send it there.

---

## 10. UI specification

### 10.1 Desktop layout

```
+------+-----------------------------------------------------------------+
| [=]  | [Connect] [ Search................... ] [YT v]        [ Google ] |
|      +-----------------------------------------------+-----------------+
| Home |                                               | DETAILS         |
|      |            MAIN CONTENT AREA                  | (right sidebar) |
| LIB  |        (grid of thumbnails / list)            |  cover preview  |
| Vid  |                                               |  name / alias   |
| Img  |                                               |  metadata       |
| Aud  |                                               |  change cover   |
|      |                                               |  aspect ratio   |
| YT   |                                               |  send / sync    |
+------+-----------------------------------------------+-----------------+
| [pinned folder] [pinned folder] [pinned folder] [+]   (bottom bar)      |
+-------------------------------------------------------------------------+
```

### 10.2 Left sidebar
- Big icons with labels. Sections: **Home** (recent media, devices, storage, quick actions); **Library** (Videos, Images, Audio); **YouTube** (Download, history, My Channel); optional **Devices & Transfers** (devices, transfer queue, sync rules), plus optional Favorites/Recent/Settings.
- **Only the left sidebar is closable.** A toggle at its **top-left** collapses it to **icons only** and expands it. **Width is resizable** when open and remembered.
- Folders in it are drop targets with spring-loaded opening.

### 10.3 Top bar
- Placed **beside** the left sidebar (not above it). Left to right: **Connect** button (pair/connect; becomes status chip), **search bar**, **YouTube icon dropdown**, **Google account button** (optional login, far right).
- **YouTube dropdown** (styled like YouTube's create/profile menu): **Upload**, **Download from YouTube**, **Your channel**, **YouTube Studio**.
- **Download from YouTube** opens a modal: link, format, quality, destination, Download.

### 10.4 Main content area
- **Videos:** thumbnail grid (I1) with duration badge, sync badge (cloud for remote items), device icon. Remote items show thumbnail + **Download / Sync**.
- **Images:** uniform/justified grid; folder cards show a 2x2 preview.
- **Audio:** list/grid with cover, title, artist, duration, source device, play button.
- Folder cards appear alongside files so browsing feels like a gallery, not a file tree.
- Grids are **virtualized** (Flutter slivers); thumbnails fade in from placeholders.

### 10.5 Right details sidebar
- Appears on selection (File Explorer-style). Shows cover/thumbnail, name, editable alias, type, size, dates, duration, resolution or audio tags, location, device, sync state, path.
- **Change cover** + **crop with presets (4:3, 9:16, 16:9, 1:1, custom)** for video and audio.
- Actions: Send to device, Download/Sync, Rename, Open folder, Delete (recycle bin only).

### 10.6 Bottom bar
- Customizable **pinned folder shortcuts**; right-click rename/remove; drag to reorder; drop files on a shortcut to move them.

### 10.7 Visual style
- Clean, modern, light and dark themes, large icons, smooth thumbnail loading, keyboard shortcuts.

### 10.8 Android layout
- **Bottom navigation** (Home, Library, YouTube, Devices); Library tabs for Videos / Images / Audio.
- Top bar: search, Connect status chip, YouTube menu, account avatar.
- Details panel becomes a **bottom sheet**; mini player at bottom, full player on tap.
- Share-sheet integration: "Send with Media Gallery".

---

## 11. Connectivity and sync design

### 11.1 Identity and pairing
- Each device has a long-lived keypair (its identity). Pairing via QR (encodes the device's address info + one-time secret) or 6-digit PIN. Paired devices are stored in an allow list; "Forget device" revokes.
- All connections are encrypted and mutually authenticated by key. Unpaired devices cannot request files.

### 11.2 Paths and transports
1. **Direct path** on LAN or wired (USB tethering/ADB reverse).
2. **Hole-punched internet path** (cross-network).
3. **Relay fallback** when hole punching fails.
4. **Bluetooth** (P2): separate transport implementing the same transfer trait.

Decision needed: use the iroh **public relay network** or **self-hosted relays** (see open questions). Relays only carry encrypted traffic.

### 11.3 Protocols (custom ALPNs over the QUIC connection)
- `index/1`: exchange index **deltas** (new/changed/removed records) and thumbnail availability.
- `files/1`: request a file by ID/hash; chunked, resumable, BLAKE3-verified stream.
- `ctrl/1`: pairing, device info, transfer requests/approvals, ping.

### 11.4 Index sync
- Each device exports deltas of **its own** records only (no conflicts).
- With Google login: deltas and thumbnails are also cached in Drive `appDataFolder` so devices can browse each other's libraries even when the owner is offline.
- Without login: deltas are exchanged directly when connected.

### 11.5 On-demand file access
- Select remote item, send `files/1` request, owner streams chunks, verify hash, mark `Synced`. Preview streams without saving (E4).

### 11.6 Limits to show users
- If the owner is offline, files cannot be downloaded (E6), though cached thumbnails remain.

---

## 12. Data model (minimum)

- **device**: id (public key), name, type, last_seen, paths/transports
- **media**: id, device_id, path, type, mime, size, blake3, created_at, modified_at, duration, width, height, gps_lat, gps_lon, place, tags (json), alias, auto_alias, cover_id, sync_state
- **cover**: id, media_id, source (generated/custom/embedded), file_path, cloud_id, aspect, crop (json), frame_time_ms
- **transfer**: id, media_id, from_device, to_device, status, bytes_done, bytes_total, path_kind, error
- **pinned_folder**: id, device_id, path, label, icon, position
- **rename_log**: id, media_id, old_name, new_name, applied_at
- **download_job**: id, url, format, quality, status, output_media_id, error
- **settings**: key, value
- Schema migrations are versioned and forward-only.

## 13. Auto-alias and search rules

| Source | Rule |
|---|---|
| File name | Strip prefixes (`IMG_`, `VID_`, `Screenshot_`), `_`/`-` to spaces, split camelCase, trim trailing numbers |
| Date | EXIF date (photos); recorded/creation date (video); fallback modified date |
| Time | For filters ("morning", "evening") and optional token |
| Location | GPS to place name via offline dataset; online lookup only with opt-in |
| Tags | Audio: `Artist - Title`; YouTube video: video title |
| Device | Source device name |

- Default alias template `{date} {place} {name}` (editable).
- Aliases are **non-destructive**. "Apply as real rename" renames on disk, logs to `rename_log`, supports undo.
- Search index covers real name, alias, tags, date, place, device.

## 14. YouTube integration

- yt-dlp (+ ffmpeg) with an in-app **updater** (YouTube changes often).
- Title/uploader/thumbnail written to tags; thumbnail saved as cover.
- Saved to the device's default Videos/Music folder (configurable).
- Google OAuth scopes minimal: identity, Drive appDataFolder, YouTube read-only; upload scope only when Upload is built.
- **Compliance:** downloading can conflict with YouTube's Terms of Service and content owners' rights, and Google Play generally rejects such apps. Distribute as installer/APK outside the Play Store and show a short "download only content you have rights to" notice. Keep the downloader modular so it can be removed.

## 15. Non-functional requirements

- **Performance:** scan 100,000 files without UI jank; first thumbnails visible within 2 s of opening a folder; scan throughput and memory tracked in benchmarks; thumbnail cache is size-limited (LRU).
- **UI:** 60 fps scrolling in grids on a mid-range Android phone and a mid-range PC.
- **Offline-first**, **privacy** (no file names/locations leave the device by default), **security** (key-based auth, encrypted transport, tokens in OS secure storage).
- **Reliability:** resumable transfers, safe rename with undo, deletes go to trash.
- **Battery (Android):** throttled background scan; foreground service only for active work.
- **Accessibility:** keyboard navigation, scalable text, contrast.
- **Localization-ready:** externalized strings (English first, Indonesian next).
- **Binary size:** report Rust core size per ABI; strip and use LTO for release.

---

## 16. Migration from the legacy build

The legacy product was already built (by an orchestrator agent, which chose its own language). It is the **reference for behavior**, not for code structure.

### 16.1 Principles
- **Behavior parity first, then improvement.** Nothing in the legacy build is dropped unless recorded in `DECISIONS.md` with a reason.
- **Legacy code is read-only** in `/legacy`. Do not edit it.
- **Strangler approach:** migrate module by module; each migrated module must pass parity tests before the next starts.

### 16.2 Step 1: Inventory (before writing new code)
Produce `docs/MIGRATION.md` containing:
1. Detected legacy language(s), frameworks, build system, and dependencies.
2. Module map: each legacy module/file group and its purpose.
3. **Feature parity matrix:** every feature in the legacy build mapped to a PRD requirement ID, with status (works / partial / broken / missing) and the target crate or Flutter feature.
4. Data inventory: legacy DB schema/files, cache layout, settings, thumbnails, alias data, pinned folders, tokens.
5. Behavior notes: anything the legacy build does that this PRD does not describe (candidates for new requirements).
6. Known bugs and performance problems in the legacy build.

### 16.3 Step 2: Data migration
- Write an **importer** that reads the legacy data (DB/files) into the new schema: media index, aliases, covers/thumbnails (keep existing thumbnails if the "random first third" rule was already applied), pinned folders, settings, paired devices where possible.
- Importer is **idempotent**, never modifies legacy data, and produces an import report (counts, skipped items, errors).
- If legacy auth tokens cannot be migrated safely, require re-login (never copy tokens in plain text).

### 16.4 Step 3: Parity testing
- Build a **fixture library** (generated mixed media: images with/without EXIF, videos, audio with/without tags, odd names, duplicates, 10k+ files).
- Run the legacy build and the new core on the same fixtures and compare: file counts per type, extracted metadata, auto-alias output, search results for a fixed query list, thumbnail frame positions (must lie in the first third), transfer integrity (hashes equal).
- Record differences in `docs/PARITY.md`; each is either fixed, or accepted with reasoning.
- Compare performance: scan time, memory, time to first thumbnail. The new build must be **equal or better**, otherwise explain why.

### 16.5 Step 4: Order of migration
1. Core index + scanner (A) and media metadata (B, C data side)
2. Alias engine + search (C)
3. Thumbnails/covers (I)
4. Flutter shell + desktop UI (section 10)
5. Networking: pairing, transfer, sync (D, E)
6. Android layer (section 8)
7. YouTube (G), account (H), music player (F)

### 16.6 Step 5: Cutover
- Ship an **import wizard** on first launch if legacy data is detected.
- Keep the legacy build runnable side by side until the acceptance criteria in section 21 pass.
- Final report: what was migrated, what changed, what was dropped, and open issues.

---

## 17. Milestones

| # | Milestone | Outcome |
|---|---|---|
| M0 | **Spike + foundation** | Workspace, FRB wiring, CI, licence audit. Spikes proving: iroh builds and connects on Windows **and** Android (direct + relay), FRB streams work, ffmpeg sidecar thumbnail, MediaStore plugin feeding Rust. Migration inventory (16.2) done. |
| M1 | Core index + desktop library | Scan, SQLite/FTS5, aliases, thumbnails, Flutter shell, gallery views, details panel |
| M2 | Data migration + parity | Importer, fixtures, parity report for implemented features |
| M3 | Android app | MediaStore scan, mobile layout, foreground service, music player |
| M4 | Pairing + transfer | QR/PIN pairing, LAN transfer both ways, queue, resume, verification |
| M5 | Sync + cross-network | Index deltas, remote items, Download/Sync, hole-punched/relay paths, optional Google cache |
| M6 | YouTube | Download popup, yt-dlp (desktop sidecar, Android plugin), dropdown |
| M7 | Wired + Bluetooth + polish | USB mode, Bluetooth fallback, bulk rename, auto-sync rules, duplicates, performance pass, installers |

**M0 is a gate:** if iroh on Android or the FFI approach fails a spike, stop and report in `DECISIONS.md` with alternatives before continuing.

## 18. Testing and quality gates

- **Rust:** unit tests per crate; property tests for alias rules and path handling; integration tests for scanner, DB migrations, transfer resume and hash verification, thumbnail frame selection (first third).
- **Network:** two-process tests on loopback; simulated packet loss/interruption; hole-punch and relay paths tested on real devices at least once per release.
- **Flutter:** widget tests with the mock bridge; golden tests for key screens in light/dark.
- **Parity:** fixture comparison against the legacy build (16.4).
- **Performance benchmarks** in CI: scan 100k fixture files, query latency, thumbnail throughput.
- CI: `cargo fmt`, `clippy -D warnings`, tests, `flutter analyze`, codegen-up-to-date check, Windows and Android build.

## 19. Risks

| Risk | Mitigation |
|---|---|
| iroh or Rust on Android has surprises (NDK, ABIs, battery) | M0 spike gate; keep transport behind a trait |
| FFI churn slows UI work | Coarse API, stable `core-api`, mock bridge, codegen check in CI |
| Two languages (Rust + Dart) and Kotlin plugins raise complexity | Strict layering, documented contracts in `/docs/API.md` |
| Bluetooth/USB differ by vendor | Treat as P1/P2; LAN/internet first |
| yt-dlp breaks or is blocked | Updater; clear errors; modular removal |
| Android restrictions on executables/storage | `youtubedl-android`; MediaStore/scoped storage |
| Licence conflicts (ffmpeg, libmpv, libheif) | Licence audit in M0; sidecar/dynamic linking; LGPL builds |
| Legacy behavior undocumented | Inventory + fixture parity tests + side-by-side runs |
| Relay reliance and privacy concerns | Encrypted end to end; option to self-host relays |
| Large libraries slow the UI | Virtualization, paged queries, thumbnail cache |

## 20. Assumptions and open questions

### Assumptions
1. One optional Google account button (far right of the top bar).
2. "Alias" = display name stored in the app, with optional real rename.
3. "Cache and cloud" for thumbnails = local cache + small metadata cache in the user's own Drive app data.
4. Second sidebar category is named **Library**.
5. Windows is the first desktop target.
6. The legacy build is complete enough to serve as a behavior reference.

### Open questions
1. Final product name?
2. iroh public relays or self-hosted relays?
3. Should Bluetooth ship in the first release, or after LAN/internet/USB?
4. Is YouTube **Upload** needed in v1?
5. macOS/Linux at launch, or Windows + Android only?
6. Can the app delete files, or only move to trash?
7. Which packaging for Windows (MSIX, installer, portable)?

## 21. Acceptance criteria (MVP)

1. Desktop app scans a folder of 10,000 mixed media files and shows Videos, Images, Audio with thumbnails, no UI freeze.
2. Video thumbnails come from a random frame in the first third and stay the same after restart.
3. Selecting a file opens the right sidebar; changing an audio cover with a 16:9 crop saves and shows immediately.
4. Searching "2026 beach" finds files by auto-alias (date + place) and by real name.
5. Dragging a file over a folder opens it after about 700 ms; dropping moves the file.
6. Left sidebar collapses to icons only, expands, and remembers its width.
7. Phone and PC pair via QR on the same Wi-Fi; a 1 GB video sent from PC to phone completes with hash verification and resumes after interruption.
8. PC shows phone videos as thumbnails with a cloud badge; Download pulls the real file.
9. With phone on mobile data and PC on home Wi-Fi, paired devices still connect and transfer (hole punch or relay).
10. Pasting a YouTube link downloads MP3/MP4 into the library with title and cover.
11. App works fully on LAN with no Google login.
12. **Legacy parity:** every feature in the parity matrix is works/accepted; imported legacy library opens with aliases, covers, pinned folders intact; performance equal or better than legacy.
