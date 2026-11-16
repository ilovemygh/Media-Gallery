# Media Gallery v0.2

**Media Gallery** is a cross-platform visual media explorer, serverless device-to-device sync engine ("serverless Google Drive"), unified PC + Phone music library, and integrated `yt-dlp` YouTube downloader for **Windows Desktop** and **Android**.

- **Device-owned storage:** Every device owns its files and local index records; **no media file contents are ever uploaded to the cloud**.
- **Works offline on LAN:** Device pairing, delta index sync, and resumable **BLAKE3**-verified chunk transfers run over **`iroh` QUIC** (`mg/ctrl/1`, `mg/index/1`, `mg/files/1`), wired USB ADB/tethering, or Bluetooth fallback without any Google login.
- **Rust core + Flutter UI:** Heavy indexing, hashing, SQLite FTS5 queries, metadata parsing, and networking run off the UI thread in Rust (`crates/`) and communicate with Flutter (`app/`) through coarse-grained async `flutter_rust_bridge` v2 contracts (`docs/API.md`).
- **Non-destructive by default:** Renames create searchable aliases first; "Apply as real rename" logs atomic undo entries to `rename_log`; deletes go exclusively to Trash / OS Recycle Bin (`TrashBackend`).

See [`PRD-Media-Gallery.md`](./PRD-Media-Gallery.md), [`docs/DECISIONS.md`](./docs/DECISIONS.md), [`docs/MIGRATION.md`](./docs/MIGRATION.md), [`docs/API.md`](./docs/API.md), and [`docs/PARITY.md`](./docs/PARITY.md) for full architecture, licence audits, and benchmark parity details.

---

## Repository Structure

```text
Media-Gallery/
├── .github/workflows/ci.yml        # Multi-OS Rust, FRB drift check, Flutter, APK & legacy CI
├── crates/
│   ├── core-platform/              # OS abstractions: LGPL ffmpeg sidecar, TrashBackend, SecureStore, FileWatcher, YtDlpBackend, MediaStore DTO
│   ├── core-index/                 # rusqlite + FTS5 schema v1/v2, BLAKE3 streaming/partial hashing, rayon scanner, cursor paging, legacy SQLite importer
│   ├── core-media/                 # Auto-alias engine, offline geocoder (landmarks + GeoNames), undoable rename/move/trash, fixed first-third thumbnails, lofty cover art editor
│   ├── core-net/                   # iroh 1.4.1 QUIC nodes, QR/PIN Ed25519 pairing, path selector (Wired > LAN > HolePunch > Relay > BT), chunked resumable transfer
│   ├── core-sync/                  # Serverless index + thumbnail delta sync, offline guard, optional Google appDataFolder sync & H2 logout wipe
│   ├── core-ytdl/                  # yt-dlp argument builder, downloader queue, updater, auto-ingest into SQLite library
│   └── core-api/                   # Single FRB v2 facade (MediaGalleryCore, CoreEvent broadcast, ApiError, MusicPlayerController) & M0 spike tests
├── app/                            # Flutter UI (DesktopShell, AndroidShell, ImportWizardDialog, CoreBridge / MockCoreBridge, Riverpod state, widget tests)
├── android-plugins/                # Kotlin native plugins: mediastore/, fgservice/, bluetooth/, usb/, ytdlp/, sharesheet/
├── tools/parity-runner/            # Parity verification & 1k / 10k / 100k SQLite + FTS5 benchmark CLI
├── docs/                           # PRD v0.2, MIGRATION.md, DECISIONS.md, API.md, PARITY.md
└── legacy/                         # Read-only TypeScript/React/Electron v0.1 snapshot (`npm --prefix legacy test`)
```

---

## Prerequisites

- **Rust toolchain:** `rustup` matching [`rust-toolchain.toml`](./rust-toolchain.toml) (`stable`, components `rustfmt` and `clippy`).
- **Flutter SDK (`>=3.24.0`, Dart `>=3.4.0`):** For compiling `/app` desktop/mobile UI and running `flutter analyze` / `flutter test`.
- **FRB Codegen v2 (`2.11.1`):** `cargo install flutter_rust_bridge_codegen --version 2.11.1`
- **Android Build Tools (for APK):** Android SDK + **Android NDK**, `cargo-ndk`, and OpenJDK 17+.
- **Optional Runtime Sidecars (Desktop):**
  - LGPL-licensed `ffmpeg` & `ffprobe` binaries (avoid GPL builds; see [`docs/DECISIONS.md`](./docs/DECISIONS.md))
  - `yt-dlp` binary (`https://github.com/yt-dlp/yt-dlp`)
  - `adb` (Android SDK Platform Tools for wired USB ADB forwarding)
- **Node.js 22+ (optional):** Only required to run the preserved read-only `/legacy` baseline tests.

---

## Setup, Codegen, Run & Test

```bash
# 1. Verify Rust formatting, lints (-D warnings), and all workspace tests + M0 spikes
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

# 2. Run parity checks & 100,000-record FTS5 benchmark (release mode)
cargo run --release -p parity-runner -- --benchmark-100k

# 3. Regenerate flutter_rust_bridge v2 bindings (when editing crates/core-api)
flutter_rust_bridge_codegen generate

# 4. Run Flutter static checks & widget tests
cd app
flutter pub get
flutter analyze
flutter test

# 5. Launch Flutter app (Desktop Windows)
flutter run -d windows

# 6. Verify preserved read-only v0.1 reference suite
npm --prefix legacy test
```

---

## Building the Windows Desktop Installer

1. Build the Rust FFI shared library (`core_api.dll`) and Flutter Windows release bundle:
   ```bash
   cargo build --release -p core-api
   cd app
   flutter build windows --release
   ```
2. Place an **LGPL** `ffmpeg.exe`, `ffprobe.exe`, and `yt-dlp.exe` next to `app/build/windows/x64/runner/Release/` (or ensure they are on `PATH`), along with dynamic `libmpv` (`mpv-2.dll` for `media_kit`) and attribution notices (`LGPL v2.1+` and `GeoNames CC-BY-4.0` per [`docs/DECISIONS.md`](./docs/DECISIONS.md)).
3. Package `app/build/windows/x64/runner/Release/` into a Windows installer using **Inno Setup** (`iscc installer.iss`) or **MSIX** (`msix` Dart dev dependency) so Windows users install cleanly without CLI prerequisites.

---

## Building a Signed Android APK (Direct Distribution)

Per PRD Section 12, Android builds are distributed directly as signed APKs (outside the Play Store):

1. Install Rust Android targets and compile `libcore_api.so` into `app/android/app/src/main/jniLibs/`:
   ```bash
   rustup target add aarch64-linux-android armv7-linux-androideabi
   cargo install cargo-ndk
   cargo ndk -t arm64-v8a -t armeabi-v7a -o app/android/app/src/main/jniLibs build --release -p core-api
   ```
2. Wire the Kotlin plugins under [`android-plugins/`](./android-plugins) (`MediaStorePlugin.kt`, `ForegroundTransferService.kt`, `BluetoothTransportPlugin.kt`, `UsbTransportPlugin.kt`, `YtDlpAndroidPlugin.kt`, `ShareTargetActivity.kt`) into the Flutter Android module (`android/app/src/main/kotlin/...` and `AndroidManifest.xml`). To avoid GPLv3 propagation in closed-source APKs, select an optional build flavor that omits `youtubedl-android` (`docs/DECISIONS.md`).
3. Create a release signing keystore (once):
   ```bash
   keytool -genkeypair -v -keystore media-gallery-release.jks -keyalg RSA -keysize 2048 -validity 10000 -alias mediagallery
   ```
4. Build the release APK, align, sign, verify, and sideload:
   ```bash
   cd app
   flutter build apk --release

   zipalign -v -p 4 build/app/outputs/flutter-apk/app-release-unsigned.apk app-release-aligned.apk
   apksigner sign --ks ../media-gallery-release.jks --ks-key-alias mediagallery --out ../MediaGallery-0.2.0-signed.apk app-release-aligned.apk
   apksigner verify --verbose ../MediaGallery-0.2.0-signed.apk

   adb install -r ../MediaGallery-0.2.0-signed.apk
   ```

---

## Final Migration Report (v0.1 `/legacy` -> v0.2 Rust + Flutter)

### What Was Migrated
- **100% Core Business Logic & Schema Parity (`A1`–`J4` + `D9`):**
  - Scanned media formats (`16` video/audio/image extensions), include/exclude folders, incremental rescan skip, duplicate flagging (`BLAKE3` partial/full hashes), and SQLite `FTS5` search (`core-index`).
  - Auto-alias naming (`{date} {place} {name}` cleanup rules, camera prefix stripping, CamelCase splitting), offline reverse geocoding (`core-media::geocoder`), non-destructive aliases, atomic undoable disk renames + `batch_id` bulk renames, folder moves, Trash staging (`core-media::rename`), fixed random first-third video frame selection, and cover aspect cropping (`4:3`, `9:16`, `16:9`, `1:1`, `custom`).
  - Pairing (6-digit PIN + QR + Ed25519 key pinning), automatic path selection (`Wired > WifiLan > HolePunched > Relay > Bluetooth`), Bluetooth slow-transfer warning, chunked resumable transfers, serverless delta + thumbnail sync, optional Google Sign-In (`H2` wipe-on-logout), unified PC+Phone music grouping & playback state (`core-api::music`), and `yt-dlp` downloader + auto-ingest (`core-ytdl`).
  - Complete UI shells (`DesktopShell` matching PRD Sec 10, `AndroidShell` matching Sec 10.8, `ImportWizardDialog`) and all 6 Kotlin Android plugins (`android-plugins/`).
- **Safe Idempotent Legacy Data Importer (`core_index::import_legacy_sqlite`):**
  - Reads v0.1 `.db` files strictly with `SQLITE_OPEN_READ_ONLY`, never alters `/legacy` data, preserves fixed video frame timestamps and covers, migrates devices/media/pinned folders/`rename_log`/`saved_search`/`auto_sync_rule`/settings, and triggers `ImportWizardDialog` on first launch.

### What Changed from Legacy v0.1
- **Runtime & Storage Engine:** Replaced Electron/Node `node:sqlite` + React DOM with a native Rust workspace (`rusqlite` bundled SQLite + `tokio` + `rayon`) behind coarse async `flutter_rust_bridge` v2 contracts and Flutter sliver virtualization.
- **Networking Upgrade:** Replaced self-signed WebSocket + LAN-only mDNS with **`iroh 1.4.1` QUIC** (`mg/ctrl/1`, `mg/index/1`, `mg/files/1`), adding NAT hole-punching and configurable relay fallback (`D9`, `D-002`).
- **Hashing & Performance:** Switched from `SHA-256` to `BLAKE3`. On the same Windows host (`parity-runner --release`), 1k-file scan improved from `869.76 ms` -> **`122.18 ms`** (`7.12x` faster), 10k batch indexing improved from `1,701.13 ms` -> **`516.64 ms`** (`3.29x` faster), 100k mixed-record indexing completed in **`5,852.88 ms`**, 100k FTS5 `"2026 beach"` query took **`10.19 ms`**, and first-thumbnail generation took **`2.10 ms`** (`docs/PARITY.md`).
- **Bug Fixes Over v0.1 (`docs/MIGRATION.md` Sec 6):** Eliminated Electron-vs-`localStorage` state split; fixed audio cover embedding (`lofty` atomic writes + explicit `confirmed` guard); fixed same-day alias sort ties via secondary `file_name` ordering.

### What Was Dropped or Mitigated
- **Removed:** Electron + Capacitor runtime wrappers and `localStorage` fallback from active builds (archived read-only in [`/legacy`](./legacy)).
- **Mitigated for Licence Compliance (`docs/DECISIONS.md`):**
  - Avoided linking `libheif` (LGPLv3); HEIC decodes through OS platform APIs or an out-of-process LGPL `ffmpeg` sidecar.
  - Isolated `youtubedl-android` (GPLv3) behind a modular Kotlin plugin adapter so proprietary APK builds can toggle or omit it via Gradle flavor.
- **Security Policy Change:** Legacy pairing/OAuth secrets are **never** copied in plain text during migration; `ImportWizardDialog` explicitly requires re-pairing / Google re-login (`auth_relogin_required = true`).

### Open Issues & Remaining Environment Gaps
- **Host Tooling Absent Locally (`docs/DECISIONS.md` `D-001`):** `flutter`/`dart` CLI, Android NDK (`aarch64-linux-android`), `ffmpeg`/`yt-dlp` binaries, and physical Android hardware were unavailable on the build workstation. As a result:
  - Rust workspace checks (`cargo fmt --check`, `cargo clippy -- -D warnings`, all `12` integration tests incl. live `iroh` QUIC transfer tests & `m0_spikes`), `parity-runner --benchmark-100k`, and `npm --prefix legacy test` (`22/22` tests) all ran and passed locally.
  - Flutter analysis, widget tests (`app/test/widget_test.dart`), `flutter_rust_bridge_codegen` drift checks, Windows/APK release builds (`.github/workflows/ci.yml`), live Android `MediaStorePlugin`/`ForegroundTransferService` integration, live cross-network cellular relay phone tests, and live external `ffmpeg`/`yt-dlp` binary invocations still require running in CI and on physical Windows + Android devices.
