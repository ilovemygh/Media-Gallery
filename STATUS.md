# Project Status Checklist

## STEP A0 Findings
- `origin/main` was still at `dac9ad7`; local fixes (`8d5d7ef` for `core-index`/`parity-runner` and `34f1a8b` for FRB config/bindings) were never pushed, so GitHub Actions run `43814810190` on `main` (`dac9ad7`) kept failing at `Install FRB codegen & verify bindings up to date` and `Parity & 100k benchmark runner`.
- At `dac9ad7`, `flutter_rust_bridge.yaml`, `app/lib/src/rust/`, and `crates/core-api/src/frb_generated.rs` are absent from git, `tools/parity-runner/src/main.rs:174` calls `idx.upsert_media(&sample_video).unwrap()`, and `crates/core-index/src/repository.rs` uses `ON CONFLICT(id)` instead of `ON CONFLICT(device_id, path)`.
- Clean-checkout inspection of local `34f1a8b` also identified three remaining `flutter-and-bindings` CI blockers to fix before CI can turn green: (1) `flutter_rust_bridge_codegen generate` runs before `flutter pub get` (causing Dart 3.14 vs `>=3.4.0` format drift) and Windows `core.autocrlf=true` needs `.gitattributes` LF rules for generated files; (2) `desktop_shell.dart:667` `_SidebarNavTile` `ListTile` overflows when collapsed to 72 px during `flutter test`; (3) `app/windows/` and `app/android/` Flutter platform runners do not exist yet for `flutter build windows --release` / `flutter build apk --release`.
- CI Run `#7` (`02c2481`) confirmed `rust-core (ubuntu-latest)`, `rust-core (windows-latest)`, `legacy-reference`, FRB drift check, `flutter analyze`, `flutter test`, Windows release build, and Android Rust NDK libs all passed; `Android release APK build` failed at `:desktop_drop:checkReleaseAarMetadata` (`compileSdk 33` < `34`) and `:file_picker:checkReleaseAarMetadata` (`compileSdk 34` < `36` required by `flutter_plugin_android_lifecycle`), both fixed by upgrading `desktop_drop` to `^0.8.4` and enforcing `compileSdk >= 36` via `LibraryAndroidComponentsExtension.finalizeDsl` in `app/android/build.gradle.kts`.

---

## PART A: Fix CI
- [x] **Step A0:** Inspect git state, remote branches/commits, failing CI run `43814810190`, and record findings in `STATUS.md`
- [x] **Failure 1 (FRB codegen & drift check):** Pinned FRB `2.11.1`, added `flutter_rust_bridge.yaml`, committed generated bindings (`app/lib/src/rust/`, `crates/core-api/src/frb_generated.rs`), added `.gitattributes` LF rules, updated `.github/workflows/ci.yml` (`flutter pub get` before codegen + `--locked` + untracked-aware drift check), verified clean-checkout zero-diff twice locally (`b9498d7`)
- [x] **Failure 2 (`upsert_media` & `parity-runner`):** Switch `media` upsert to `ON CONFLICT(device_id, path) DO UPDATE`, isolate parity-runner DB/fixtures, replace `unwrap()` with `anyhow::Context`, add 3 regression tests in `core-index`
- [x] **Flutter CI readiness (`flutter test`, `windows`, `android`):** Fixed collapsed `_SidebarNavTile` overflow in `app/lib/features/shell/desktop_shell.dart` (`4c88a3d`), added `[lib] crate-type = ["cdylib", "rlib"]` to `crates/core-api/Cargo.toml`, scaffolded `app/windows/` (CMake `core_api.dll` build + bundle) & `app/android/` (`io.mediagallery.app` + env-driven release signing), updated `.github/workflows/ci.yml` (verified prebuilt FRB `2.11.1`, JDK 17, NDK `28.2.13676358`, `cargo-ndk`, disk cleanup before APK build, structured APK failure output), bumped `desktop_drop` to `^0.8.4`, and floored Android library `compileSdk >= 36` in `app/android/build.gradle.kts`
- [x] **Part A Gate:** CI Run `#44462776804` (`09429ed`) confirmed 100% green (`rust-core` Windows+Ubuntu, `flutter-and-bindings` including Windows release + Android APK build, `legacy-reference`)

---

## PART B: GitHub Releases + Easy Install README
- [x] **Inspect platform builds:** Audited `app/pubspec.yaml`, `app/android/`, `app/windows/`, `core-api` (`cdylib`), NDK `28.2.13676358`, and sidecar lookup (`ffmpeg.exe`, `ffprobe.exe`, `yt-dlp.exe`); added executable-adjacent `resolve_sidecar_bin` in `core-platform` + `core-api` with unit test coverage
- [x] **B1: Triggers & Versioning:** Created `.github/workflows/release.yml` (`v*.*.*` tags, automatic pre-releases on `-`, `workflow_dispatch` dry run), tag/`pubspec.yaml`/`Cargo.toml` version parity check, `tools/release/set-version.ps1` & `set-version.sh`, and enabled `workflow_call` gate on `.github/workflows/ci.yml`
- [x] **B2: Windows Release Build:** Added pinned & SHA-256 verified LGPL `ffmpeg`/`ffprobe` + `yt-dlp` (`tools/release/sidecars.lock`, `tools/release/update-sidecars-lock.ps1`), `THIRD_PARTY_LICENSES.txt` generation, VS `Microsoft.VC143.CRT` runtime bundling, Inno Setup `.exe` (`tools/release/windows/installer.iss`) + portable `.zip` (`tools/release/windows/stage-windows.ps1`), and optional Authenticode code signing
- [x] **B3: Android Release Build:** Verified env-driven release signing in `app/android/app/build.gradle.kts` (release workflow enforces `REQUIRE_RELEASE_SIGNING=true` on tags when secrets are required; dry runs warn cleanly if secrets absent), configured split `arm64-v8a`, `armeabi-v7a`, and `universal` APKs, and added `tools/release/android/verify-apks.ps1` (`.so`, `apksigner`, `versionName`/`versionCode` checks)
- [x] **B4: Publishing:** Configured `SHA256SUMS.txt` generation/verification, draft release -> upload 6 assets -> verify count -> publish, and structured release notes in `.github/workflows/release.yml`
- [x] **B5: README "Install" Section:** Added CI/Release badges, download links & quick selector table, Windows SmartScreen & Android sideload/Play Protect guides, SHA-256 verification, 3-step QR pairing, troubleshooting, and moved developer docs under `Build from source` in `README.md`
- [x] **B6: Maintainer Docs:** Wrote `docs/RELEASING.md` (keystore generation, secrets setup, cutting/testing/deleting releases, updating `sidecars.lock`)
- [x] **Part B Gate:** Verified full Release dry-run (`44579573145` on `c32532e`): all 8 jobs passed (`prepare`, `ci-gate`, `build-windows` with Inno Setup silent install/launch/uninstall + portable `.zip`, `build-android` with verified `arm64-v8a`/`armeabi-v7a`/`universal` APKs, and `publish` `SHA256SUMS.txt` artifact bundle)

---

## PART C: Live Core Wiring + Google-Files & Explorer UX Upgrades
- [x] **Live Rust FFI Core (`core-api` + `LiveCoreBridge`):** Exposed `MediaGalleryCore` via FRB v2 (`frb_init_core`, `frb_scan_folders`, `frb_ingest_android_mediastore`, `frb_list_media`, `frb_ensure_thumbnails_batch`, `frb_get_storage_stats`, `frb_preview_bulk_rename`/`frb_apply_bulk_rename`/`frb_undo_bulk_rename`, `frb_restore_from_trash`/`frb_empty_trash`, pairing, YouTube queue, pinned folders) with regenerated bindings and live integration test
- [x] **Android MediaStore Channel (`MainActivity.kt`):** Wired `io.mediagallery.app/mediastore` `MethodChannel` with runtime media permissions (`READ_MEDIA_*` / `READ_EXTERNAL_STORAGE`) and JSON batch ingestion into Rust
- [x] **Desktop & Android UX Upgrades:** Added real `Image.file` thumbnails & 2x2 folder mosaics, Home **Recent Media** strip, **Clean & Storage** dashboard (Duplicates, Large Files >50 MB, Recycle Bin restore/empty), **Bulk Rename** preview/undo, interactive **MediaViewerDialog**, **Devices & Backup** QR pairing (`qr_flutter`), **YouTube** download queue, S/M/L/List density & sort controls, and persistent **MiniPlayerBar**

---

## PART D: 3 Big Meaningful Improvements & Final Release Gate
- [x] **Improvement 1 — Real `media_kit` Playback + Unified Music Library:** Bundled `media_kit_libs_windows_video` & `media_kit_libs_android_video`, added `MediaPlaybackService` (real video surface in `MediaViewerDialog`, live seekbar/position/duration in `MiniPlayerBar`, shuffle/repeat/next/prev), and added Unified Music grouping (`All / Artist / Album / Folder / Device`) in Audio view
- [x] **Improvement 2 — Live `iroh` QUIC P2P Pairing, Transfer Queue & 1-Tap Backup:** Bound `IrohQuicNode` in `MediaGalleryCore` with incoming `ALPN_CTRL`/`ALPN_INDEX`/`ALPN_FILES` loops, wired `frb_pair_with_device` (QR + 6-digit PIN), `frb_send_media_to_device`, `frb_download_remote_media`, `frb_backup_local_media_to_device`, `frb_list_transfers`/pause/cancel, and Auto-Sync rule toggles in `DevicesBackupView` and Details sidebar
- [x] **Improvement 3 — 1-Tap Duplicate Auto-Cleaner & Explorer Power Tools:** Added `frb_clean_duplicates` (1-tap Keep Newest/Oldest safe trash cleanup), `Saved Searches` (`frb_save_search`/`frb_list_saved_searches`), pinned folder shortcut rename & reorder (`J3`), and dedicated `SettingsView` (included/excluded scan folders, alias template, theme, transport preference)
- [x] **Final Gate:** Verified CI Run `44595868947` (`9dd1664`) 100% green and Release Dry-Run `44602025894` 100% green (all 8 jobs including Windows installer/portable `.zip` + Android split/universal APKs + `SHA256SUMS.txt`)

---

## PART E: v0.2.1 Stability, Audio Visualizer, Dropdowns, Profile & Expanded Settings
- [x] **Windows Hidden Sidecars (`core-platform`):** Set `CREATE_NO_WINDOW` (`0x08000000`) and `Stdio::null()` on all `ffmpeg`, `ffprobe`, `yt-dlp`, and `reg` subprocesses so scanning and YouTube downloads never flash terminal/console windows on Windows (`8282d1d`)
- [x] **Full-Disk Scan Crash Prevention & Deduplication (`core-index`, `core-media`, `core-api`):** Replaced full-memory path collection with bounded streaming chunks, added NTFS reparse-point/junction cycle guards, default OS/system directory exclusions (`Windows`, `Program Files`, `AppData`, `$Recycle.Bin`, `System Volume Information`), `catch_unwind` around EXIF/image/audio/video decoders, SQLite `busy_timeout`, and canonical path/folder deduplication (`e090058`, `6331cd8`)
- [x] **Persistent SQLite Library & Silent Background Auto-Scan:** Loaded existing SQLite index immediately on startup without requiring a manual rescan, added silent background incremental scan (`autoScanOnOpen` on desktop + `scanMediaStoreSilent` on Android `MainActivity.kt`), and persisted extended settings in `user_prefs.json` (`c105268`, `280bd78`)
- [x] **Openable YouTube Downloads:** Resolved completed `yt-dlp` jobs to indexed `MediaItem` records and added an openable bottom `CompletedDownloadBar` plus `Open` actions on completed queue items (`6331cd8`, `280bd78`, `b5c0628`)
- [x] **Audio Sound-Wave Visualizer, Profile Avatar, Dropdowns & New Logo:** Added interactive `AudioWaveformVisualizer` and `MiniSoundWaveIndicator`, replaced the Google chip with `ProfileAvatarButton`, converted grid/view size and Details sidebar sections into dropdowns (`GridSizeViewDropdown` + collapsible `Details & Size`), redesigned Windows `.ico` / Android launcher icons / vector `MediaGalleryLogo`, expanded Settings (Open at Startup, Auto-Scan, Media Types, File Formats, YouTube Defaults) on desktop and Android, and bumped version to `v0.2.1` (`5e30728`, `b5c0628`, `a0bcf12`)

---

## PART F: Universal Linux Support, Sorting & Feature Wiring Fixes (v0.2.2)
- [x] **No auto-rescan loop & thumbnail safety:** Persisted `initial_scan_done` in `user_prefs.json`, changed `autoScanOnOpen` default to `false` after initial index creation, tracked `_attemptedThumbIds` per session, and verified on-disk thumbnail existence in `ThumbnailManager::ensure_thumbnail`
- [x] **Sorting, Audio Grouping & Action Wiring:** Updated `SortField::Date` SQL to `MAX(m.modified_at, m.created_at)`, stopped overriding Sort inside `Recent`, added Sort & Audio Grouping controls to Android Library, rendered `ui.musicGroups` in Audio view, wired YouTube Upload/Channel/Studio browser launch + Reveal in Folder + directory picker for Pin Folder + shuffle track skip + user Home resolution for relative YouTube downloads
- [x] **Universal Linux & Arch Linux Builds:** Added `app/linux/` CMake runner bundling `libcore_api.so`, `media_kit_libs_linux`, pinned Linux LGPL `ffmpeg`/`ffprobe` + `yt-dlp` checksums in `tools/release/sidecars.lock`, `tools/release/linux/stage-linux.sh` (`.AppImage` + portable `.tar.gz` + `install.sh`), and `linux-build` / `build-linux` CI & Release jobs

---

## PART G: Out-of-the-Box Power Upgrades & Local Linux Setup (v0.2.3)
- [x] **Magic Query Operators & Filter Chips:** Parsed `res:4k`, `res:hd`, `len:long`, `len:short`, `ext:<fmt>`, and `time:morning|afternoon|evening|night` directly alongside FTS5 in `core-index` + `MockCoreBridge`, and exposed one-click **Magic Filter Chips** (`4K+`, `HD+`, `Long >5m`, `Morning`, `Evening`, `Night`) in `DesktopShell`
- [x] **Playback Speed, Mute Toggle, Slideshow & Keyboard Cockpit:** Added `MediaPlaybackService.setRate`, speed menu (`0.5x–2.0x`) + volume mute button in `MiniPlayerBar`, 3s **Slideshow** auto-advance in `_MediaViewerDialog`, and desktop keyboard shortcuts (`Ctrl+F` search focus, `←`/`→` navigate items, `Space` play/view, `Delete` trash)
- [x] **Duplicate Visual Compare & Linux Native Integration:** Added **Storage Breakdown Bar** + **Compare Copies & Pick Keeper** (`Keep Only This`) cards in `CleanerDashboardView`, XDG `~/.config/autostart/media-gallery.desktop` support for `Open at Startup` on Linux, default `auto_scan_on_open = false` consistency in `core-platform`, and PNG `.DirIcon` fix for `appimagetool`

