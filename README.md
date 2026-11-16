# Media Gallery

**Media Gallery** is a cross-platform visual media explorer, serverless device-to-device sync engine ("serverless Google Drive"), unified PC + Phone music player, and integrated `yt-dlp` YouTube downloader for **Windows Desktop** and **Android**.

- Every device owns and stores its own files; **no file contents are ever uploaded to the cloud**.
- Local LAN transfers and index delta sync work completely **offline without any Google login**.
- Backed by **SQLite with FTS5**, streaming **SHA-256** verification, **Electron** (Desktop), and **Capacitor** (Android).

See [`PRD-Media-Gallery.md`](./PRD-Media-Gallery.md), [`DECISIONS.md`](./DECISIONS.md), and [`CHANGELOG.md`](./CHANGELOG.md) for product specifications, stack decisions, and requirement coverage.

---

## Repository Structure

```text
Media-Gallery/
├── android-bridge/              # Capacitor config, MediaStore mapper & Kotlin MediaMetadataRetriever bridge
├── electron/                    # Electron main process, secure preload IPC bridge & file watcher host
├── src/
│   ├── core/
│   │   ├── database/            # SQLite schema (8 tables + media_fts FTS5), NodeSqliteAdapter, MediaRepository
│   │   ├── types/               # Shared domain types
│   │   └── utils/               # Streaming SHA-256 & 64KB partial file hashing
│   ├── features/
│   │   ├── alias/               # PRD Sec 9 auto-alias engine, offline GPS gazetteer, undoable rename & trash
│   │   ├── connectivity/        # TransportChannel, QR/PIN pairing, Wi-Fi LAN, USB ADB, Bluetooth, chunked resume engine
│   │   ├── player/              # Unified PC+Phone de-duplicated music library & playback controller
│   │   ├── scanner/             # Non-blocking chunked scanner, fs.watch incremental watcher, duplicate detector
│   │   ├── search/              # FTS5 query service & saved smart searches
│   │   ├── sync/                # Serverless index delta sync & optional Google Drive appDataFolder sync
│   │   ├── thumbnails/          # Fixed random first-third video frame extractor, LRU cache & aspect crop editor
│   │   └── youtube/             # yt-dlp + ffmpeg download queue, metadata tagger & updater
│   └── ui/                      # React Desktop UI shell (Sec 7) & Android Mobile UI preview (Sec 8)
└── tests/                       # Vitest unit, integration & 10,000-file benchmark tests
```

---

## Prerequisites

- **Node.js 22+** (verified on Node.js v26.7.0 with native `node:sqlite` + FTS5) and **npm 10+**
- Optional Desktop binaries on `PATH` (used when executing real media extractions/downloads):
  - [`ffmpeg`](https://ffmpeg.org/) (video frame capture, audio cover embedding, YouTube merging)
  - [`yt-dlp`](https://github.com/yt-dlp/yt-dlp) (YouTube downloads)
  - `adb` (Android SDK Platform Tools, for wired USB ADB port-forwarding)
- For Windows Installer packaging: `electron` + `electron-builder`
- For Android APK packaging: **Android SDK** (`platforms`, `build-tools`), **Java JDK 17+**, and Capacitor Android platform (`@capacitor/android`)

---

## Setup, Run & Test

```bash
# 1. Install dependencies
npm install

# 2. Run all unit, integration & 10k-file benchmark tests
npm test

# 3. Typecheck and build production web bundle
npm run build

# 4. Run interactive UI in development mode
npm run dev
```

To preview the **Android Mobile Layout** (PRD Section 8) directly in the browser, open **Settings → Preview Android Mobile UI**.

---

## Building the Windows Desktop Installer (NSIS `.exe`)

1. Install desktop packaging dev dependencies:
   ```bash
   npm install --save-dev electron electron-builder
   ```
2. Compile the web bundle and Electron main/preload processes:
   ```bash
   npm run build
   npm run electron:build
   ```
3. Add `"build"` packaging metadata to `package.json` (or pass CLI config) and build the NSIS Windows installer:
   ```bash
   npx electron-builder --win nsis --x64
   ```
   Output installer will be generated in `release/Media Gallery Setup 0.1.0.exe`.
   *(Tip: bundle `ffmpeg.exe` and `yt-dlp.exe` inside `resources/bin/` via `extraResources` so desktop thumbnails and YouTube downloads work out of the box without user PATH setup.)*

---

## Building a Signed Android APK (Direct Sideload Distribution)

Per PRD Section 12, Media Gallery is distributed directly as a signed APK (not via Google Play Store).

1. Initialize and sync Capacitor Android project:
   ```bash
   npm install @capacitor/core @capacitor/cli @capacitor/android
   npm run build
   npx cap add android
   npx cap sync android
   ```
2. Copy the native Kotlin MediaStore / `MediaMetadataRetriever` bridge [`android-bridge/native/MediaGalleryBridge.kt`](./android-bridge/native/MediaGalleryBridge.kt) into `android/app/src/main/java/io/mediagallery/app/` and register permissions (`READ_MEDIA_IMAGES`, `READ_MEDIA_VIDEO`, `READ_MEDIA_AUDIO`, `INTERNET`, `ACCESS_NETWORK_STATE`, `BLUETOOTH_CONNECT`, `FOREGROUND_SERVICE`) in `AndroidManifest.xml`.
3. Generate a release signing keystore (once):
   ```bash
   keytool -genkeypair -v -keystore media-gallery-release.jks -keyalg RSA -keysize 2048 -validity 10000 -alias mediagallery
   ```
4. Build the unsigned release APK with Gradle:
   ```bash
   cd android
   ./gradlew assembleRelease
   ```
5. Zipalign and sign the APK using Android SDK `build-tools`:
   ```bash
   zipalign -v -p 4 app/build/outputs/apk/release/app-release-unsigned.apk app-release-aligned.apk
   apksigner sign --ks ../media-gallery-release.jks --ks-key-alias mediagallery --out ../MediaGallery-0.1.0-signed.apk app-release-aligned.apk
   apksigner verify --verbose ../MediaGallery-0.1.0-signed.apk
   ```
6. Install directly onto an attached Android device over USB:
   ```bash
   adb install -r MediaGallery-0.1.0-signed.apk
   ```
