# Project Status Checklist

## STEP A0 Findings
- `origin/main` was still at `dac9ad7`; local fixes (`8d5d7ef` for `core-index`/`parity-runner` and `34f1a8b` for FRB config/bindings) were never pushed, so GitHub Actions run `43814810190` on `main` (`dac9ad7`) kept failing at `Install FRB codegen & verify bindings up to date` and `Parity & 100k benchmark runner`.
- At `dac9ad7`, `flutter_rust_bridge.yaml`, `app/lib/src/rust/`, and `crates/core-api/src/frb_generated.rs` are absent from git, `tools/parity-runner/src/main.rs:174` calls `idx.upsert_media(&sample_video).unwrap()`, and `crates/core-index/src/repository.rs` uses `ON CONFLICT(id)` instead of `ON CONFLICT(device_id, path)`.
- Clean-checkout inspection of local `34f1a8b` also identified three remaining `flutter-and-bindings` CI blockers to fix before CI can turn green: (1) `flutter_rust_bridge_codegen generate` runs before `flutter pub get` (causing Dart 3.14 vs `>=3.4.0` format drift) and Windows `core.autocrlf=true` needs `.gitattributes` LF rules for generated files; (2) `desktop_shell.dart:667` `_SidebarNavTile` `ListTile` overflows when collapsed to 72 px during `flutter test`; (3) `app/windows/` and `app/android/` Flutter platform runners do not exist yet for `flutter build windows --release` / `flutter build apk --release`.

---

## PART A: Fix CI
- [x] **Step A0:** Inspect git state, remote branches/commits, failing CI run `43814810190`, and record findings in `STATUS.md`
- [x] **Failure 1 (FRB codegen & drift check):** Pinned FRB `2.11.1`, added `flutter_rust_bridge.yaml`, committed generated bindings (`app/lib/src/rust/`, `crates/core-api/src/frb_generated.rs`), added `.gitattributes` LF rules, updated `.github/workflows/ci.yml` (`flutter pub get` before codegen + `--locked` + untracked-aware drift check), verified clean-checkout zero-diff twice locally (`b9498d7`)
- [x] **Failure 2 (`upsert_media` & `parity-runner`):** Switch `media` upsert to `ON CONFLICT(device_id, path) DO UPDATE`, isolate parity-runner DB/fixtures, replace `unwrap()` with `anyhow::Context`, add 3 regression tests in `core-index`
- [/] **Flutter CI readiness (`flutter test`, `windows`, `android`):** Fixed collapsed `_SidebarNavTile` layout crash in `app/lib/features/shell/desktop_shell.dart` (`flutter analyze` + `flutter test` 2/2 passing); next: scaffold `app/windows/` and `app/android/` (`applicationId = io.mediagallery.app`, `core-api` `cdylib` build) so `flutter build windows --release` and `flutter build apk --release` pass in CI
- [ ] **Part A Gate:** Push to `main` and confirm real GitHub Actions CI run is 100% green (`rust-core` Windows+Ubuntu, `flutter-and-bindings`, `legacy-reference`)

---

## PART B: GitHub Releases + Easy Install README
- [ ] **Inspect platform builds:** Audit `app/pubspec.yaml`, `app/android/`, `app/windows/`, Rust `core-api` loading (`cdylib`), NDK version, and sidecar paths (`ffmpeg.exe`, `ffprobe.exe`, `yt-dlp.exe`)
- [ ] **B1: Triggers & Versioning:** Create `.github/workflows/release.yml` (`v*.*.*` tags, pre-releases, `workflow_dispatch` dry run), tag/pubspec/Cargo version check, `tools/release/set-version.ps1` & `.sh`, gate on `ci.yml` (`workflow_call`)
- [ ] **B2: Windows Release Build:** Bundle pinned & SHA-256 verified LGPL `ffmpeg`/`ffprobe` + `yt-dlp` (`tools/release/sidecars.lock`), `THIRD_PARTY_LICENSES`, VC++ runtime handling, Inno Setup `.exe` + portable `.zip`, optional code signing
- [ ] **B3: Android Release Build:** Env-driven release signing in Gradle (fail loudly in release workflow when secrets absent; debug/CI still works), build signed `arm64-v8a`, `armeabi-v7a`, and `universal` APKs, verify `.so` presence, `apksigner verify`, `versionName`/`versionCode`
- [ ] **B4: Publishing:** Generate `SHA256SUMS.txt`, draft release -> upload all assets -> publish on full success, structured release notes
- [ ] **B5: README "Install" Section:** Badges, download links & quick selector table, Windows SmartScreen & Android sideload/Play Protect guides, SHA-256 verification, 3-step QR pairing, troubleshooting, move dev docs under "Build from source"
- [ ] **B6: Maintainer Docs:** Write `docs/RELEASING.md` (keystore generation, secrets setup, cutting/testing/deleting releases, updating `sidecars.lock`)
- [ ] **Part B Gate:** Run `workflow_dispatch` dry run, verify all exact asset names, SHA-256 checksums, clean Windows silent install/launch/uninstall + portable launch, and pre-release tag if secrets are present
