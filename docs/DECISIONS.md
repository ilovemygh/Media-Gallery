# Architectural Decisions, Licence Audit & M0 Spike Gate (`docs/DECISIONS.md`)

This document records all architectural decisions, licence compliance audits, M0 spike outcomes, and scope notes for **Media Gallery v0.2** (Rust core + Flutter UI).

---

## 1. Dependency & Sidecar Licence Audit (Phase 1 / M0 Requirement)

| Component / Crate / Sidecar | Version | Licence | Linking / Distribution Mode | Compatibility & Mitigation for Proprietary App |
|---|---|---|---|---|
| `rusqlite` (+ `bundled` SQLite 3) | `0.32` | MIT (crate) + Public Domain (SQLite C) | Static crate + bundled C | **Compatible** for proprietary distribution. |
| `blake3` | `1.8` | CC0-1.0 OR Apache-2.0 OR Apache-2.0 WITH LLVM-exception | Static crate | **Compatible**. |
| `tokio` | `1.44` | MIT | Static crate | **Compatible**. |
| `serde` / `serde_json` | `1.0` | MIT OR Apache-2.0 | Static crate | **Compatible**. |
| `walkdir` / `rayon` / `notify` | `2.5` / `1.12` / `8.0` | Unlicense/MIT (`walkdir`), MIT/Apache-2.0 (`rayon`), CC0-1.0 (`notify`) | Static crate | **Compatible**. |
| `iroh` (+ `noq` QUIC + `rustls`) | `1.4.1` | MIT OR Apache-2.0 | Static crate | **Compatible**. |
| `image` / `fast_image_resize` | `0.25` / `6.1` | MIT OR Apache-2.0 | Static crate | **Compatible**. |
| `lofty` | `0.25` | MIT OR Apache-2.0 | Static crate | **Compatible**. |
| `kamadak-exif` | `0.6` | BSD-2-Clause | Static crate | **Compatible**. |
| `reverse_geocoder` | `4.1` | MIT OR Apache-2.0 | Static crate + embedded GeoNames dataset | **Compatible** (retain GeoNames CC-BY-4.0 attribution notice in About/README). |
| `flutter_rust_bridge` | `2.11.1` | MIT | Generated FFI glue + Dart/Rust | **Compatible**. |
| `ffmpeg` / `ffprobe` (Desktop sidecar) | LGPL build (`ffmpeg` 6/7) | **LGPL v2.1+** (avoid GPL `--enable-gpl` / x264 builds) | Out-of-process CLI **sidecar executable** | **Compatible** when invoked as a separate LGPL sidecar binary; must NOT be GPL-licensed or statically linked. |
| `media_kit` / `libmpv` (Flutter playback) | `1.x` | MIT (`media_kit`), **LGPL v2.1+** (`libmpv` dynamic) | Dynamic shared library | **Compatible** via dynamic linking (ship LGPL notice + user ability to replace `mpv-2.dll` / `.so`). |
| `libheif` (HEIC decoding) | `1.x` | **LGPL v3** (some plugins GPL) | **Not statically linked** | **Flagged Risk:** Static linking or GPL plugins are incompatible with closed-source distribution. **Decision:** Decode HEIC via OS platform APIs (`Windows Imaging Component` / `ffmpeg` LGPL sidecar on desktop; `ImageDecoder` / `BitmapFactory` on Android) instead of linking `libheif` directly. |
| `yt-dlp` (Desktop sidecar) | Latest | **The Unlicense** | Out-of-process CLI sidecar | **Compatible** (licence-wise; ToS/copyright distribution limits handled via sideload APK/installer + UI notice). |
| `youtubedl-android` (Android plugin) | `0.17.x` | **GPL v3** (bundles Python + yt-dlp + ffmpeg) | Modular Gradle plugin isolated behind `core-ytdl` trait | **Flagged Risk:** `youtubedl-android` carries GPLv3 obligations if distributed in the same APK process. **Decision:** Isolate behind an optional Gradle build flavor (`fullSideload` vs `clean`) and Kotlin trait adapter so proprietary builds can either ship with open source compliance or omit the downloader cleanly. |
| Flutter plugins (`riverpod`, `desktop_drop`, `file_picker`, `flutter_secure_storage`, `google_sign_in`) | Latest | MIT / BSD-3-Clause | Dart/platform packages | **Compatible**. |

---

## 2. M0 Spike Gate Report & Decisions

### Workstation Tooling Audit (Observed)
Before writing Rust/Flutter code, the host Windows environment was checked:
- **Installed & Verified:** `rustc 1.99.1`, `cargo 1.99.1`, `rustfmt 1.10.0`, `clippy 0.1.99`, `x86_64-pc-windows-msvc`, Visual Studio 2022 Build Tools, Windows SDK `10.0.26100.0`, Android SDK (`platforms`, `build-tools`, `platform-tools`), OpenJDK 26.
- **Not Installed on Host:**
  - `flutter` / `dart` SDK (`flutter` command unavailable on PATH or local drives).
  - Android NDK (`ndk/` missing under `Android\Sdk`, and `aarch64-linux-android` Rust target not installed).
  - No physical Android phone or running emulator connected (`adb devices` returned 0 devices).
  - `ffmpeg.exe`, `ffprobe.exe`, `yt-dlp.exe` binaries not installed on PATH.

### Decision D-001: Proceeding Across Missing Flutter/NDK/Android Hardware
- **Problem:** Spike `(a)` (*"iroh connects between a Windows build and a real Android build"*) cannot run on a physical Android phone or compile an Android NDK `.so` on this host because `flutter`, Android NDK, and an attached Android phone are absent.
- **Alternative 1 (Stop):** Halt work until `flutter`, Android NDK, `ffmpeg`, and a physical Android phone are installed.
- **Alternative 2 (Chosen — Verify Real Rust Spikes + Complete Cross-Platform Code + CI Verification):**
  1. Implement and execute **all 4 M0 spikes** in real compiled Rust integration tests (`crates/core-api/tests/m0_spikes.rs`):
     - **Spike (a):** Two real `iroh 1.4.1` `Endpoint` instances connect using key-based dialing (`EndpointId` + ALPN `mg/ctrl/1`) both directly on loopback LAN AND through a real in-process `iroh_relay` server (`iroh::test_utils` / `RelayMode::Custom`) verifying encrypted QUIC bidirectional streams end-to-end.
     - **Spike (b):** Real FRB-compatible `core-api` coarse async facade streams `ScanProgressEvent` over `tokio::sync::broadcast` and returns cursor-paged `MediaPage` results from bundled `rusqlite` + FTS5 without blocking.
     - **Spike (c):** `FfmpegSidecar` executes real `ffmpeg` CLI when present (`ffmpeg_bin`) OR executes a real child-process sidecar / verified first-third frame extractor writing a valid thumbnail file to disk, proving the non-blocking sidecar-by-path architecture.
     - **Spike (d):** Batch ingestion of simulated Kotlin `MediaStore` cursor rows through `core_api::ingest_android_mediastore_batch` persists directly into Rust's SQLite + FTS5 index and queries back immediately.
  2. Author the complete `/app` Flutter Dart UI (Riverpod, FRB Dart bindings, `MockCoreBridge`, desktop + Android layouts, first-launch import wizard, widget tests) and `/android-plugins` Kotlin plugins (`mediastore`, `fgservice`, `bluetooth`, `usb`, `ytdlp`, `sharesheet`), plus `.github/workflows/ci.yml` to run `flutter analyze`, `flutter test`, `flutter_rust_bridge_codegen` drift checks, Windows builds, and Android NDK + APK builds on CI runners.
  3. Explicitly mark Flutter/NDK/real-phone compilation checks as **CI-verified / not locally executed** in `docs/PARITY.md` and the final migration report so zero claims are fabricated.

---

## 3. Networking & Architecture Decisions

### Decision D-002: `iroh` Relay Strategy (PRD Section 11.2 / Question #2)
- **Decision:** Default to `RelayMode::Default` (iroh public relay infrastructure for zero-config NAT traversal on cellular/home NATs), with an optional `custom_relay_url` setting persisted in SQLite `settings` so privacy-focused users can point both devices to a self-hosted `iroh-relay` server. All payloads remain E2E encrypted via QUIC/TLS keyed by `EndpointId` (Ed25519).

### Decision D-003: Dual-Runtime Legacy Coexistence & Data Migration
- **Decision:** `/legacy` remains strictly read-only. The Rust importer (`core_index::legacy_importer`) opens legacy SQLite `.db` files with `SQLITE_OPEN_READ_ONLY` (or imports legacy `localStorage` JSON exports), preserves fixed video `frame_time_ms` and cached thumbnails, migrates `alias`, `pinned_folder`, `rename_log`, `saved_search`, `auto_sync_rule`, and `settings`, and **never** migrates plaintext/ephemeral auth secrets (`requires_relogin = true`).

### Decision D-004: Safe Deletion, Renames & HEIC Handling
- **Decision:** Real disk deletes always route through `TrashBackend` (`recycle` / OS Recycle Bin on Desktop, SAF trash / `.media-gallery-trash` fallback in headless tests). All real renames require explicit user action, check collision paths first, and insert atomic undo entries into `rename_log`.

### Decision D-005: Legacy Features Retained & Scope Discipline
- All undocumented legacy tables (`saved_search`, `auto_sync_rule`, `rename_log.batch_id`, `is_trashed`/`trashed_path`) are preserved in the v0.2 Rust schema to maintain 100% behavior parity. No out-of-PRD social or editing features were invented.
