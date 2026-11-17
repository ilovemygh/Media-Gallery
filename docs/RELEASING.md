# Releasing Media Gallery (`docs/RELEASING.md`)

This guide covers maintainer procedures for signing, bumping versions, updating pinned sidecar checksums, running dry-run release builds, publishing GitHub Releases, and rolling back bad tags.

---

## 1. One-Time Signing & Secrets Setup

### Android Release Keystore (Required for Tag Releases)

Production and pre-release tags (`v*.*.*`) enforce `REQUIRE_RELEASE_SIGNING=true` in `app/android/app/build.gradle.kts` so releases never silently ship with the Android debug certificate.

1. Generate a release keystore once and back it up in an encrypted password manager / hardware vault (**never commit `.jks`, `.keystore`, or `key.properties` to git**):
   ```bash
   keytool -genkeypair -v \
     -keystore media-gallery-release.jks \
     -storetype PKCS12 \
     -keyalg RSA -keysize 2048 -validity 10000 \
     -alias mediagallery
   ```
   > **Warning:** Losing this keystore prevents existing users from upgrading over an installed APK without uninstalling first.

2. Base64-encode the keystore for GitHub Actions secrets:
   - **PowerShell (Windows):**
     ```powershell
     [Convert]::ToBase64String([System.IO.File]::ReadAllBytes("media-gallery-release.jks")) | Set-Clipboard
     ```
   - **Linux / macOS:**
     ```bash
     base64 -w 0 media-gallery-release.jks   # Linux
     base64 -i media-gallery-release.jks     # macOS
     ```

3. Add these **Repository Secrets** under **GitHub -> Settings -> Secrets and variables -> Actions**:

   | Secret | Required | Description |
   |---|---|---|
   | `ANDROID_KEYSTORE_BASE64` | Yes (for tags) | Base64-encoded `.jks` / PKCS12 keystore |
   | `ANDROID_KEYSTORE_PASSWORD` | Yes (for tags) | Keystore password |
   | `ANDROID_KEY_ALIAS` | Yes (for tags) | Key alias (e.g. `mediagallery`) |
   | `ANDROID_KEY_PASSWORD` | Yes (for tags) | Key password |
   | `WINDOWS_CERT_PFX_BASE64` | Optional | Base64-encoded Authenticode `.pfx` code-signing certificate |
   | `WINDOWS_CERT_PASSWORD` | Optional | Password for the `.pfx` certificate |

   When `WINDOWS_CERT_PFX_BASE64` is omitted, Windows binaries are built unsigned and `README.md` documents the SmartScreen "More info -> Run anyway" flow alongside SHA-256 verification.

---

## 2. Updating `tools/release/sidecars.lock`

Windows builds bundle pinned, SHA-256 verified sidecar binaries (`ffmpeg.exe`, `ffprobe.exe` from `BtbN/FFmpeg-Builds` **LGPL**, and `yt-dlp.exe` from `yt-dlp/yt-dlp`) declared in [`tools/release/sidecars.lock`](../tools/release/sidecars.lock).

> **Important:** `BtbN/FFmpeg-Builds` prunes dated `autobuild-*` release tags after roughly 14 days. Refresh `sidecars.lock` before cutting a release if the pinned FFmpeg autobuild tag has aged out.

1. Run the lockfile updater from Windows PowerShell:
   ```powershell
   .\tools\release\update-sidecars-lock.ps1
   ```
   This fetches official upstream checksum manifests (`SHA2-256SUMS` from `yt-dlp/yt-dlp` and `checksums.sha256` from the latest dated `BtbN/FFmpeg-Builds` `autobuild-*` release), selects the **`win64-lgpl`** ZIP (never GPL), and updates `tools/release/sidecars.lock`.

2. Verify the diff and confirm `stage-windows.ps1` passes the LGPL flag check (no `--enable-gpl` or `--enable-nonfree` in `ffmpeg.exe -version`):
   ```powershell
   git diff tools/release/sidecars.lock
   git add tools/release/sidecars.lock
   git commit -m "build(release): refresh pinned ffmpeg LGPL and yt-dlp sidecar checksums"
   ```

---

## 3. Testing a Release Without Publishing (`workflow_dispatch` Dry Run)

Before tagging, run a full end-to-end release build in dry-run mode:

1. Open **GitHub -> Actions -> Release -> Run workflow**.
2. Keep **`dry_run: true`** (the default) and click **Run workflow**.
3. The workflow runs:
   - Version consistency check (`Cargo.toml` `[workspace.package].version` == `app/pubspec.yaml` version name).
   - Full `ci.yml` gate via `workflow_call`.
   - Windows x64 build + sidecar SHA-256 verification + VC++ CRT bundling + Inno Setup installer + portable ZIP + clean silent install/launch/uninstall & portable launch checks.
   - Android `arm64-v8a`, `armeabi-v7a`, and `universal` APK builds + `.so` / `apksigner` / `aapt2` checks (uses release signing if secrets exist; warns and falls back to debug signing during dry run when secrets are unset).
   - `SHA256SUMS.txt` generation and verification.
4. Download the `release-assets` workflow artifact from the run summary to inspect every binary locally. No GitHub Release or tag is created during a dry run.

---

## 4. Cutting a Release (Pre-Release or Production)

1. Ensure `main` is clean and CI is green:
   ```bash
   git checkout main
   git pull --ff-only origin main
   ```

2. Bump versions across `Cargo.toml` and `app/pubspec.yaml` (using either PowerShell or Bash):
   ```powershell
   # Production release (auto-increments Android +buildNumber in pubspec.yaml)
   .\tools\release\set-version.ps1 0.2.1

   # Or pre-release
   .\tools\release\set-version.ps1 0.3.0-rc.1
   ```
   ```bash
   ./tools/release/set-version.sh 0.2.1
   ```

3. Commit, push, and verify CI passes on `main`:
   ```bash
   git add Cargo.toml app/pubspec.yaml
   git commit -m "chore(release): bump version to v0.2.1"
   git push origin main
   ```

4. Create and push the annotated tag (`v*.*.*`):
   - Tags with a `-` suffix (e.g. `v0.3.0-rc.1`, `v0.2.0-beta.1`) are automatically marked as **Pre-release** on GitHub.
   - Tags without `-` (e.g. `v0.2.1`) are published as the **Latest Release**.
   ```bash
   git tag -a v0.2.1 -m "Media Gallery v0.2.1"
   git push origin v0.2.1
   ```

5. What `.github/workflows/release.yml` guarantees:
   - Enforces version parity between the tag, `Cargo.toml`, and `app/pubspec.yaml`.
   - Fails immediately if `ci.yml` does not pass or if Android release signing secrets are missing.
   - Creates a **Draft** GitHub Release first, uploads all 6 assets (`MediaGallery-<v>-windows-x64-setup.exe`, `MediaGallery-<v>-windows-x64-portable.zip`, `MediaGallery-<v>-android-arm64-v8a.apk`, `MediaGallery-<v>-android-armeabi-v7a.apk`, `MediaGallery-<v>-android-universal.apk`, and `SHA256SUMS.txt`), verifies every upload, and only then transitions the release from draft to published.

---

## 5. Deleting or Rolling Back a Release

If a pre-release or failed draft release needs to be removed:

```bash
# Delete the GitHub Release (keep or delete tag separately)
gh release delete v0.3.0-rc.1 --yes

# Delete the remote and local tag
git push origin :refs/tags/v0.3.0-rc.1
git tag -d v0.3.0-rc.1
```

> **Rule of thumb:** Never delete and re-push an already-public stable tag (`v0.2.0`); increment the patch version (`v0.2.1`) instead so cached downloads and `SHA256SUMS.txt` remain auditable.
