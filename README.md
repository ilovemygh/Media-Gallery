# Media Gallery v0.2

[![CI](https://github.com/ilovemygh/Media-Gallery/actions/workflows/ci.yml/badge.svg)](https://github.com/ilovemygh/Media-Gallery/actions/workflows/ci.yml)
[![Latest Release](https://img.shields.io/github/v/release/ilovemygh/Media-Gallery?include_prereleases&label=release)](https://github.com/ilovemygh/Media-Gallery/releases)
[![Platforms](https://img.shields.io/badge/platforms-Windows%20x64%20%7C%20Android-blue)](#install)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-green)](#third-party-notices)

**Media Gallery** is a fast, visual cross-platform media explorer, serverless device-to-device sync engine ("serverless Google Drive"), unified PC + Phone music library, and built-in `yt-dlp` YouTube downloader for **Windows Desktop** and **Android**.

- **Device-owned storage:** Every device keeps its own files and SQLite index; **no media file contents are ever uploaded to the cloud**.
- **Offline-first & cross-network sync:** Direct peer-to-peer pairing, metadata/thumbnail index sync, and resumable **BLAKE3**-verified file transfers over **`iroh` QUIC** (LAN, wired USB/ADB, NAT hole punching, relay fallback, or Bluetooth).
- **Non-destructive by default:** Searchable auto-aliases (`{date} {place} {name}`), undoable real renames (`rename_log`), and Recycle Bin / Trash deletes (`TrashBackend`).

---

## Install

Download prebuilt binaries directly from **[GitHub Releases](https://github.com/ilovemygh/Media-Gallery/releases/latest)** (or view **[All Releases & Pre-releases](https://github.com/ilovemygh/Media-Gallery/releases)**).

### Which file should I download?

| Platform | Release Asset | Best For |
|---|---|---|
| **Windows 10/11 (64-bit) Installer** | `MediaGallery-<version>-windows-x64-setup.exe` | Most Windows users — installs shortcuts, sidecars (`ffmpeg`, `ffprobe`, `yt-dlp`), VC++ runtime, and clean uninstaller (no admin required) |
| **Windows 10/11 (64-bit) Portable** | `MediaGallery-<version>-windows-x64-portable.zip` | No-install / USB drive use — extract the whole folder and run `media_gallery.exe` |
| **Android (Modern 64-bit Phones)** | `MediaGallery-<version>-android-arm64-v8a.apk` | Smaller download for virtually all phones released since 2018 (Pixel, Samsung Galaxy, Xiaomi, OnePlus, etc.) |
| **Android (Older 32-bit Phones)** | `MediaGallery-<version>-android-armeabi-v7a.apk` | Older 32-bit ARM Android devices |
| **Android (Universal Fallback)** | `MediaGallery-<version>-android-universal.apk` | Works on every supported Android ABI (`arm64-v8a`, `armeabi-v7a`, `x86_64`) if unsure |
| **Checksums** | `SHA256SUMS.txt` | Verify file integrity before installing |

---

### Windows Installation & SmartScreen Guide

1. Download **`MediaGallery-<version>-windows-x64-setup.exe`** (or extract **`MediaGallery-<version>-windows-x64-portable.zip`** into a new folder).
2. *(Recommended)* Verify the SHA-256 checksum against `SHA256SUMS.txt` (see [Verify Checksums](#verify-sha-256-checksums)).
3. Run the installer. Because open-source community builds without an EV code-signing certificate can trigger **Microsoft Defender SmartScreen**, if you see **"Windows protected your PC"**:
   1. Click **More info**.
   2. Confirm the filename matches the release you verified against `SHA256SUMS.txt`.
   3. Click **Run anyway**.
4. Choose per-user install (no administrator UAC prompt required) or system-wide install, then launch **Media Gallery**.

---

### Android Sideload & Play Protect Guide

Per [`PRD-Media-Gallery.md`](./PRD-Media-Gallery.md) Section 14, Android builds are distributed directly outside Google Play because Media Gallery includes a built-in YouTube downloader (`yt-dlp`).

1. Download **`MediaGallery-<version>-android-arm64-v8a.apk`** (or **`universal.apk`** if unsure) on your phone.
2. Tap the downloaded `.apk` in your browser or **Files** app.
3. If Android prompts **"For your security, your phone currently isn't allowed to install unknown apps from this source"**:
   - Tap **Settings** -> toggle **Allow from this source** -> navigate back and tap **Install**.
4. If **Google Play Protect** shows **"Unsafe app blocked"** or asks to scan the APK:
   - Tap **More details** -> **Install anyway** (or **Scan app** -> **Install**).
5. On first launch, grant storage/media permissions (`Photos and videos` / `Music and audio`) so Media Gallery can index your phone's local media.

---

### Verify SHA-256 Checksums

Download `SHA256SUMS.txt` alongside your installer or APK, then verify:

- **Windows (PowerShell):**
  ```powershell
  Get-FileHash .\MediaGallery-<version>-windows-x64-setup.exe -Algorithm SHA256
  Select-String -Path .\SHA256SUMS.txt -Pattern "MediaGallery-<version>-windows-x64-setup.exe"
  ```
- **Linux / macOS / Git Bash:**
  ```bash
  sha256sum -c SHA256SUMS.txt --ignore-missing
  ```

---

### Quick Start: 3-Step QR Pairing (Phone <-> PC)

1. **Open Devices on PC:** Click **Connect** in the top bar (or open **Devices & Transfers** in the left sidebar) to display your PC's pairing **QR code** and **6-digit PIN**.
2. **Pair from Android:** Open Media Gallery on your phone -> tap **Devices** -> scan the QR code (or enter the 6-digit PIN). Both devices exchange Ed25519 keys over encrypted `iroh` QUIC.
3. **Browse & Transfer:** Your phone and PC libraries merge visually with cloud badges on remote items. Drag files onto a device chip or click **Download** to transfer directly with BLAKE3 verification—no file contents ever touch a cloud server.

---

### Troubleshooting

- **Windows portable build fails to launch (`core_api.dll` or `flutter_windows.dll` not found):**
  Make sure you extracted the **entire** `.zip` folder rather than running `media_gallery.exe` from inside Windows Explorer's ZIP preview. Keep `core_api.dll`, `flutter_windows.dll`, `vcruntime140.dll`, `ffmpeg.exe`, `ffprobe.exe`, `yt-dlp.exe`, and the `data/` folder together.
- **"App not installed" on Android:**
  1. If you previously installed a local debug build, uninstall it first (release APKs use a different signing certificate).
  2. If `arm64-v8a` is rejected by an older device or emulator, install `MediaGallery-<version>-android-universal.apk`.
- **Phone and PC cannot find each other:**
  - Verify both devices have network access; allow `media_gallery.exe` on **Private** networks if **Windows Defender Firewall** prompts.
  - Across different networks (e.g., phone on cellular data, PC at home), `iroh` automatically falls back to hole punching and encrypted relay transport.
- **YouTube downloads fail:**
  - Ensure `yt-dlp.exe` and `ffmpeg.exe` exist next to `media_gallery.exe` on Windows, and update `yt-dlp` if YouTube changed its site format (`Download only content you have the right to download`).

---

## Build from source

See [`PRD-Media-Gallery.md`](./PRD-Media-Gallery.md), [`docs/DECISIONS.md`](./docs/DECISIONS.md), [`docs/MIGRATION.md`](./docs/MIGRATION.md), [`docs/API.md`](./docs/API.md), [`docs/PARITY.md`](./docs/PARITY.md), and [`docs/RELEASING.md`](./docs/RELEASING.md) for architecture, licence audits, benchmarks, and release automation details.

### Repository Structure

```text
Media-Gallery/
├── .github/workflows/
│   ├── ci.yml                      # Multi-OS Rust, FRB drift check, Flutter, APK & legacy CI
│   └── release.yml                 # Tag & dry-run release pipeline (Windows + signed APKs + SHA256SUMS)
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
├── tools/
│   ├── parity-runner/              # Parity verification & 1k / 10k / 100k SQLite + FTS5 benchmark CLI
│   └── release/                    # Version bumper, pinned sidecars.lock, Inno Setup installer, APK verifier
├── docs/                           # PRD v0.2, MIGRATION.md, DECISIONS.md, API.md, PARITY.md, RELEASING.md
└── legacy/                         # Read-only TypeScript/React/Electron v0.1 snapshot (`npm --prefix legacy test`)
```

### Prerequisites

- **Rust toolchain:** `rustup` matching [`rust-toolchain.toml`](./rust-toolchain.toml) (`1.100.0`, components `rustfmt` and `clippy`).
- **Flutter SDK (`3.50.0` / `>=3.24.0`, Dart `>=3.4.0`):** For compiling `/app` desktop/mobile UI and running `flutter analyze` / `flutter test`.
- **FRB Codegen v2 (`2.11.1`):** Prebuilt binary or `cargo install flutter_rust_bridge_codegen --version 2.11.1 --locked`.
- **Android Build Tools (for APK):** Android SDK, **Android NDK (`28.2.13676358`)**, `cargo-ndk`, and OpenJDK 17+.
- **Windows Packaging Tools (for Installer):** Visual Studio 2022 C++ Build Tools + VC++ Redistributable CRT, **Inno Setup 6** (`iscc`), and optional sidecars pinned in [`tools/release/sidecars.lock`](./tools/release/sidecars.lock) (LGPL `ffmpeg`/`ffprobe` + `yt-dlp`).
- **Node.js 24+ (optional):** Only required to run the preserved read-only `/legacy` baseline tests.

### Setup, Codegen, Run & Test

```bash
# 1. Verify Rust formatting, lints (-D warnings), and all workspace tests + M0 spikes
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

# 2. Run parity checks & 100,000-record FTS5 benchmark (release mode)
cargo run --release -p parity-runner -- --benchmark-100k

# 3. Regenerate flutter_rust_bridge v2 bindings (when editing crates/core-api)
cd app && flutter pub get && cd ..
flutter_rust_bridge_codegen generate --config-file flutter_rust_bridge.yaml

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

### Building the Windows Installer & Portable Bundle Locally

```powershell
# Builds Flutter Windows release + core_api.dll, downloads/verifies sidecars from
# tools/release/sidecars.lock, bundles VC++ CRT DLLs + THIRD_PARTY_LICENSES.txt,
# and creates dist/windows/MediaGallery-<version>-windows-x64-setup.exe & portable.zip
.\tools\release\windows\stage-windows.ps1 -VerifyInstaller
```

### Building Android APKs Locally

```powershell
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
cargo install cargo-ndk --locked
cargo ndk -t arm64-v8a -t armeabi-v7a -t x86_64 -o app/android/app/src/main/jniLibs build --release -p core-api

cd app
flutter build apk --release --split-per-abi
flutter build apk --release
```

For signing configuration, automated GitHub Releases, and maintainer checklists, see [`docs/RELEASING.md`](./docs/RELEASING.md).

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

Made with Love <3