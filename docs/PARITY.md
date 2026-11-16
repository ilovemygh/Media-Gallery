# Behavior Parity & Benchmark Report (`docs/PARITY.md`)

This document records module-by-module verification of **Media Gallery v0.2** (Rust core workspace + Flutter UI + `iroh` QUIC + Kotlin plugins) against the read-only v0.1 reference in [`/legacy`](../legacy) and [`PRD-Media-Gallery-v0.2.md`](./PRD-Media-Gallery-v0.2.md).

---

## 1. Local vs. CI Verification Scope

Per [`docs/DECISIONS.md`](./DECISIONS.md) (`D-001`), the Windows workstation used during migration has the Rust toolchain (`rustc 1.99.1`, `cargo 1.99.1`, `clippy`, `rustfmt`) and Node.js runtime, but does **not** have the `flutter`/`dart` SDK, Android NDK (`aarch64-linux-android`), external `ffmpeg`/`yt-dlp` binaries on `PATH`, or an attached physical Android phone.

| Verification Suite | Execution Environment | Observed Result |
|---|---|---|
| `cargo fmt --all -- --check` | **Local Windows host** | Passed (0 diffs) |
| `cargo clippy --workspace --all-targets -- -D warnings` | **Local Windows host** | Passed (0 warnings) |
| `cargo test --workspace` | **Local Windows host** | **12 / 12 integration tests passed** across 6 test suites (`0` failed) |
| `cargo run --release -p parity-runner -- --benchmark-100k` | **Local Windows host** | Passed (`parity_checks_passed: true`) |
| `npm --prefix legacy test` | **Local Windows host** | **8 / 8 files, 22 / 22 tests passed** (`7.39s`) |
| `flutter_rust_bridge_codegen` drift check, `flutter analyze`, `flutter test`, `flutter build windows --release`, `flutter build apk --release` | **CI (`windows-latest` in [`.github/workflows/ci.yml`](../.github/workflows/ci.yml))** | Configured in CI; **not executed locally** due to missing `flutter` SDK & Android NDK on host |
| Real device-to-device Windows <-> Physical Android relay & OS binary `ffmpeg`/`yt-dlp` runtime checks | **External hardware / binaries required** | Rust `IrohQuicNode` loopback QUIC + sidecar invocation contract verified locally; live cellular relay & phone APK execution require physical hardware |

---

## 2. Module-by-Module Parity Verification (`PRD` A1–J4 & D9)

| Module / PRD IDs | Legacy Behavior (`/legacy`) | v0.2 Implementation | Parity & Upgrade Verification |
|---|---|---|---|
| **Index & Scanner** (`A1`–`A5`) | Node `ScannerService` + SHA-256 partial hash + `node:sqlite` FTS5 | `core-index` (`rusqlite` bundled SQLite + FTS5 + `walkdir` + `rayon` + `blake3`) | `index_tests.rs::scans_all_16_formats_skips_excluded_and_flags_duplicates_with_blake3` verifies all 16 extensions (`mp4..heic`), excluded folder filtering, incremental rescan skip, duplicate flagging via BLAKE3 without deleting, and atomic cancellation. `parity-runner` confirms `supported_extensions_count = 16`, `duplicate_groups_found = 1`. |
| **Auto-Alias, Offline Geocoder & Search** (`C1`–`C6`) | TS `aliasEngine.ts`, 12-region Haversine gazetteer, `renameService.ts`, FTS5 query | `core-media` (`alias.rs`, `geocoder.rs`, `rename.rs`) + `core-index` cursor FTS5 (`repository.rs`) | `media_tests.rs::auto_alias_and_offline_reverse_geocoding_match_prd_rules` verifies `IMG_2031.jpg` cleanup, `VID_0042_sunsetWalk-09.mp4` -> `2026-03-14 sunset Walk`, and offline GPS lookup (`-8.7185, 115.1686` -> `"2026-03-14 Kuta Beach Bali"`). Upgrades legacy gazetteer with `reverse_geocoder` (GeoNames cities) while retaining legacy landmarks for exact parity. `media_tests.rs::alias_disk_rename_undo_bulk_rename_folder_move_and_safe_trash_work` verifies non-destructive aliases, collision-checked disk renames, atomic bulk `rename_log` undo, folder move, and `.media-gallery-trash` safe delete. `cursor_paging_fts5_and_mediastore_batch_ingestion_work` and `parity-runner` verify cursor-paged FTS5 `"2026 beach"` search. |
| **Thumbnails & Cover Crop Editor** (`I1`–`I4`) | Fixed first-third frame (`ffmpeg` CLI / SVG fallback) + aspect presets (`4:3`, `9:16`, `16:9`, `1:1`, `custom`) | `core-media::thumbnails`, `core-media::covers` (`lofty` 0.25 audio tags), `DesktopFfmpegSidecar` | `media_tests.rs::video_first_third_thumbnail_persists_across_restarts_and_cover_crop_works` proves random frame in `0..duration/3` (`frame_time_ms <= 30_000`, `parity-runner` recorded `15000 ms` on a `90_000 ms` fixture) persists across DB restarts and fixes legacy audio cover overwrite bugs using `lofty` atomic tag writes + explicit `confirmed` guard. Produces valid PNG files via `image` crate when `ffmpeg` CLI is absent. |
| **Pairing, Transport Priority, `iroh` QUIC & BLAKE3 Transfers** (`D1`–`D9`) | mDNS + WebSocket + ADB stub + SHA-256 chunks (`D9` missing in v0.1) | `core-net` (`iroh 1.4.1` ALPN `mg/ctrl/1`, `mg/index/1`, `mg/files/1`, `PathSelector`, `TransferEngine`, `TransferReceiver`) | `net_tests.rs::pairing_pin_qr_allowlist_and_path_priority_selector_work` verifies 6-digit PIN + QR pairing, Ed25519 key pinning, unauthenticated peer rejection, strict priority `Wired > WifiLan > HolePunched > Relay > Bluetooth`, and slow Bluetooth warning (`D5`). `net_tests.rs::real_iroh_quic_endpoints_transfer_chunks_resume_after_loss_and_verify_blake3` binds two live `iroh 1.4.1` QUIC nodes, exchanges `CtrlMessage::Ping`, injects packet drop at chunk 4 via `SimulatedFaultyTransport`, resumes cleanly to completion (`640 KiB`), matches byte-for-byte + BLAKE3 digest, and rejects corrupted chunks (`NetError::HashMismatch`). Also verified in `m0_spikes.rs::spike_a_iroh_endpoints_connect_and_exchange_control_messages`. |
| **Serverless Sync & Optional Google Auth** (`E1`–`E6`, `H1`–`H2`) | Delta JSON + base64 thumb blobs + `EncryptedMemoryTokenStore` | `core-sync` (`IndexSyncManager`, `GoogleCloudSyncClient`) + `core-platform::EncryptedMemoryStore` | `sync_tests.rs::serverless_delta_sync_offline_guard_auto_sync_and_h2_logout_wipe` verifies metadata + base64 PNG thumbnail delta replication (`Remote` / `OnDemand`), offline blocking (`SyncError::PeerOffline`), auto-sync rules, and `H2` Google logout wipe (clears OAuth token and deletes local cloud metadata cache without touching local disk media). |
| **Unified Music Player & YouTube Downloader** (`F1`–`F5`, `G1`–`G6`) | Track de-dup + `yt-dlp` spawn queue | `core-api::music` (`build_unified_music_library`, `group_unified_tracks`, `MusicPlayerController`) + `core-ytdl` (`build_yt_dlp_args`, `YoutubeDownloader`) | `ytdl_tests.rs::builds_args_updates_ytdlp_and_auto_ingests_mp3_into_library` verifies `yt-dlp` argument generation (`--no-playlist`, `--embed-thumbnail`, height filter), updater invocation, and automatic ingestion into SQLite as `source = "youtube"` playable audio. |
| **Android Layer & Flutter Shell** (`B1`–`B6`, `J1`–`J4`) | React desktop UI + Capacitor stub | `app/` Flutter shell (`DesktopShell`, `AndroidShell`, `ImportWizardDialog`, `CoreBridge`, `MockCoreBridge`) + `android-plugins/` (6 Kotlin classes) | Rust ingestion contract verified via `m0_spikes.rs::spike_bcd_frb_streams_sidecar_thumbnail_and_mediastore_ingest` & `index_tests.rs::cursor_paging_fts5_and_mediastore_batch_ingestion_work`. Two Flutter widget tests (`app/test/widget_test.dart`) test sidebar collapse, `"2026 beach"` search, details crop presets, and `ImportWizardDialog`, slated for CI verification via `flutter test`. |
| **Idempotent Legacy Migration** (`Sec 16.6`) | N/A | `core_index::import_legacy_sqlite` (strictly `SQLITE_OPEN_READ_ONLY`) + `ImportWizardDialog` | `index_tests.rs::imports_legacy_v01_sqlite_idempotently_and_requires_relogin` and `parity-runner` (`legacy_import_media_count = 1`) verify idempotent import across devices, media, preserved video frame timestamps, pinned folders, `rename_log`, `saved_search`, `auto_sync_rule`, and settings, while refusing to copy auth secrets (`auth_relogin_required = true`). |

---

## 3. Side-by-Side Performance Benchmark: Legacy v0.1 vs. Rust v0.2

Measured on the same Windows host:
- **Legacy v0.1 Baseline:** Recorded in [`docs/MIGRATION.md`](./MIGRATION.md) Section 6 using `/legacy` Node.js + `node:sqlite` + SHA-256 (`npm --prefix legacy test` also ran in `7.39s`).
- **Rust v0.2 Measurement:** Directly observed from `cargo run --release -p parity-runner -- --benchmark-100k`:

```json
{
  "parity_checks_passed": true,
  "supported_extensions_count": 16,
  "duplicate_groups_found": 1,
  "auto_alias_bali_photo": "2026-03-14 Kuta Beach Bali",
  "auto_alias_camel_video": "2026-03-14 sunset Walk",
  "fts_beach_hits": 1,
  "first_third_frame_ms": 15000,
  "blake3_transfer_verified": true,
  "legacy_import_media_count": 1,
  "scan_1k_files_ms": 122.18,
  "insert_10k_ms": 516.64,
  "insert_100k_ms": 5852.88,
  "fts_query_100k_ms": 10.19,
  "first_thumb_ms": 2.1
}
```

| Benchmark Metric | Legacy v0.1 (`Node.js` + `SHA-256`) | Rust v0.2 (`release` + `BLAKE3`) | Observed Speedup / Change |
|---|---|---|---|
| **Scan 1,000 fixture files** (hash + auto-alias + FTS5 insert) | `869.76 ms` | **`122.18 ms`** | **7.12x faster** |
| **Batch insert 10,000 mixed records + FTS5 triggers** | `1,701.13 ms` | **`516.64 ms`** | **3.29x faster** |
| **Batch insert 100,000 mixed records + FTS5 triggers** | Not run (10k max in legacy suite) | **`5,852.88 ms` (`5.85 s`)** | Targets PRD 100k scale cleanly |
| **FTS5 `"2026 beach"` query latency** | `1.38 ms` (over 11,000 rows) | **`10.19 ms`** (over **100,000** rows) | Sub-11 ms at ~9x larger scale |
| **Time to first thumbnail (fallback asset generation)** | `11.18 ms` (SVG stub) | **`2.10 ms`** (valid PNG) | **5.32x faster** (and real PNG output) |
