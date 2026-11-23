#!/usr/bin/env bash
set -euo pipefail

VERSION=""
OUTPUT_DIR=""
CACHE_DIR=""
SKIP_FLUTTER_BUILD="false"
BUILD_APPIMAGE="true"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --version) VERSION="${2#v}"; shift 2 ;;
    --output-dir) OUTPUT_DIR="$2"; shift 2 ;;
    --cache-dir) CACHE_DIR="$2"; shift 2 ;;
    --skip-flutter-build) SKIP_FLUTTER_BUILD="true"; shift ;;
    --no-appimage) BUILD_APPIMAGE="false"; shift ;;
    *) echo "Unknown arg: $1" >&2; exit 1 ;;
  esac
done

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../../.." && pwd)"
APP_DIR="${REPO_ROOT}/app"
LOCK_PATH="${REPO_ROOT}/tools/release/sidecars.lock"

if [[ -z "$VERSION" ]]; then
  VERSION="$(python3 -c "import pathlib,re; m=re.search(r'(?m)^version:\s*([^+\r\n]+)', pathlib.Path('${APP_DIR}/pubspec.yaml').read_text()); print(m.group(1).strip())")"
fi

OUTPUT_DIR="${OUTPUT_DIR:-${REPO_ROOT}/dist/linux}"
CACHE_DIR="${CACHE_DIR:-${REPO_ROOT}/build-cache/sidecars}"
mkdir -p "$OUTPUT_DIR" "$CACHE_DIR"

STAGE_PARENT="${OUTPUT_DIR}/stage-root"
PORTABLE_NAME="MediaGallery-${VERSION}-linux-x64"
STAGE_DIR="${STAGE_PARENT}/${PORTABLE_NAME}"
rm -rf "$STAGE_PARENT"
mkdir -p "$STAGE_DIR"

if [[ "$SKIP_FLUTTER_BUILD" != "true" ]]; then
  pushd "$APP_DIR" >/dev/null
  flutter pub get
  flutter build linux --release
  popd >/dev/null
fi

RELEASE_BUNDLE="${APP_DIR}/build/linux/x64/release/bundle"
if [[ ! -x "${RELEASE_BUNDLE}/media_gallery" ]]; then
  echo "Missing Flutter Linux release binary at ${RELEASE_BUNDLE}/media_gallery" >&2
  exit 1
fi
if [[ ! -f "${RELEASE_BUNDLE}/lib/libcore_api.so" ]]; then
  echo "Missing libcore_api.so in ${RELEASE_BUNDLE}/lib" >&2
  exit 1
fi

cp -a "${RELEASE_BUNDLE}/." "$STAGE_DIR/"
chmod +x "${STAGE_DIR}/media_gallery"

# Download & verify Linux sidecars from tools/release/sidecars.lock
read -r FFMPEG_URL FFMPEG_SHA FFMPEG_VER FFMPEG_ASSET YTDLP_URL YTDLP_SHA YTDLP_VER YTDLP_ASSET < <(
  python3 - "$LOCK_PATH" <<'PY'
import json, sys
lock = json.load(open(sys.argv[1], encoding="utf-8"))["sidecars"]
ff = lock["ffmpeg_linux64_lgpl"]
yt = lock["yt_dlp_linux64"]
print(ff["url"], ff["sha256"], ff["version"], ff["asset"], yt["url"], yt["sha256"], yt["version"], yt["asset"])
PY
)

verify_download() {
  local url="$1" sha="$2" dest="$3"
  if [[ -f "$dest" ]]; then
    if echo "${sha}  ${dest}" | sha256sum -c --status; then
      return 0
    fi
    rm -f "$dest"
  fi
  curl -fsSL -L -o "$dest" "$url"
  echo "${sha}  ${dest}" | sha256sum -c --status
}

FFMPEG_ARCHIVE="${CACHE_DIR}/${FFMPEG_ASSET}"
YTDLP_BIN_CACHE="${CACHE_DIR}/${YTDLP_ASSET}"
verify_download "$FFMPEG_URL" "$FFMPEG_SHA" "$FFMPEG_ARCHIVE"
verify_download "$YTDLP_URL" "$YTDLP_SHA" "$YTDLP_BIN_CACHE"

SIDECARS_DIR="${STAGE_DIR}/sidecars"
mkdir -p "$SIDECARS_DIR"
FFMPEG_EXTRACT="${STAGE_PARENT}/ffmpeg-extract"
rm -rf "$FFMPEG_EXTRACT"
mkdir -p "$FFMPEG_EXTRACT"
tar -xf "$FFMPEG_ARCHIVE" -C "$FFMPEG_EXTRACT" --strip-components=1
cp -f "${FFMPEG_EXTRACT}/bin/ffmpeg" "${SIDECARS_DIR}/ffmpeg"
cp -f "${FFMPEG_EXTRACT}/bin/ffprobe" "${SIDECARS_DIR}/ffprobe"
cp -f "$YTDLP_BIN_CACHE" "${SIDECARS_DIR}/yt-dlp"
chmod +x "${SIDECARS_DIR}/ffmpeg" "${SIDECARS_DIR}/ffprobe" "${SIDECARS_DIR}/yt-dlp"

cat > "${STAGE_DIR}/THIRD_PARTY_LICENSES.txt" <<EOF
Media Gallery bundles the following third-party components:

1. FFmpeg / ffprobe (${FFMPEG_VER}) — LGPL-2.1-or-later
   Source: ${FFMPEG_URL}
   SHA-256: ${FFMPEG_SHA}

2. yt-dlp (${YTDLP_VER}) — Unlicense
   Source: ${YTDLP_URL}
   SHA-256: ${YTDLP_SHA}
EOF
if [[ -f "${FFMPEG_EXTRACT}/LICENSE.txt" ]]; then
  echo "" >> "${STAGE_DIR}/THIRD_PARTY_LICENSES.txt"
  cat "${FFMPEG_EXTRACT}/LICENSE.txt" >> "${STAGE_DIR}/THIRD_PARTY_LICENSES.txt"
fi
rm -rf "$FFMPEG_EXTRACT"

# Desktop icon + launcher + portable installer for Arch & all Linux distros
cat > "${STAGE_DIR}/media-gallery.svg" <<'SVG'
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 128 128">
  <rect width="128" height="128" rx="28" fill="#1565C0"/>
  <rect x="20" y="24" width="88" height="62" rx="10" fill="#E3F2FD"/>
  <circle cx="44" cy="46" r="9" fill="#FFB300"/>
  <path d="M24 80l24-22 18 16 14-12 24 18z" fill="#1E88E5"/>
  <path d="M54 96h20l-10 14z" fill="#90CAF9"/>
</svg>
SVG

cat > "${STAGE_DIR}/media-gallery.desktop" <<'DESKTOP'
[Desktop Entry]
Name=Media Gallery
Comment=Cross-platform visual media explorer, P2P sync, music player & YouTube downloader
Exec=media-gallery %U
Icon=media-gallery
Terminal=false
Type=Application
Categories=AudioVideo;Utility;FileManager;
StartupWMClass=io.mediagallery.app
DESKTOP

cat > "${STAGE_DIR}/install.sh" <<'INSTALL'
#!/usr/bin/env bash
set -euo pipefail
SRC_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
INSTALL_DIR="${HOME}/.local/share/MediaGallery"
BIN_DIR="${HOME}/.local/bin"
APP_DIR="${HOME}/.local/share/applications"
ICON_DIR="${HOME}/.local/share/icons/hicolor/scalable/apps"

mkdir -p "$INSTALL_DIR" "$BIN_DIR" "$APP_DIR" "$ICON_DIR"
rm -rf "${INSTALL_DIR:?}/"*
cp -a "${SRC_DIR}/." "$INSTALL_DIR/"
ln -sf "${INSTALL_DIR}/media_gallery" "${BIN_DIR}/media-gallery"
cp -f "${INSTALL_DIR}/media-gallery.svg" "${ICON_DIR}/media-gallery.svg"
sed "s|^Exec=.*|Exec=${INSTALL_DIR}/media_gallery %U|" \
  "${INSTALL_DIR}/media-gallery.desktop" > "${APP_DIR}/media-gallery.desktop"
command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$APP_DIR" || true
echo "Installed Media Gallery to ${INSTALL_DIR}"
echo "Run 'media-gallery' (ensure ${BIN_DIR} is in PATH) or launch 'Media Gallery' from your app menu."
INSTALL
chmod +x "${STAGE_DIR}/install.sh"

# Package portable .tar.gz
TARBALL="${OUTPUT_DIR}/MediaGallery-${VERSION}-linux-x64-portable.tar.gz"
tar -C "$STAGE_PARENT" -czf "$TARBALL" "$PORTABLE_NAME"
echo "Created portable tarball: $TARBALL"

# Package universal AppImage (Arch, Ubuntu, Fedora, Debian, Mint, openSUSE, SteamOS)
if [[ "$BUILD_APPIMAGE" == "true" ]]; then
  APPDIR="${STAGE_PARENT}/MediaGallery.AppDir"
  rm -rf "$APPDIR"
  mkdir -p "${APPDIR}/usr/bin"
  cp -a "${STAGE_DIR}/." "${APPDIR}/"
  cp -f "${STAGE_DIR}/media-gallery.desktop" "${APPDIR}/media-gallery.desktop"
  cp -f "${STAGE_DIR}/media-gallery.svg" "${APPDIR}/media-gallery.svg"
  ln -sf media-gallery.svg "${APPDIR}/.DirIcon"
  sed -i 's|^Exec=.*|Exec=AppRun|' "${APPDIR}/media-gallery.desktop"

  cat > "${APPDIR}/AppRun" <<'APPRUN'
#!/usr/bin/env bash
HERE="$(dirname "$(readlink -f "$0")")"
export PATH="${HERE}/sidecars:${PATH}"
export LD_LIBRARY_PATH="${HERE}/lib:${LD_LIBRARY_PATH:-}"
exec "${HERE}/media_gallery" "$@"
APPRUN
  chmod +x "${APPDIR}/AppRun"

  APPIMAGETOOL="${CACHE_DIR}/appimagetool-x86_64.AppImage"
  if [[ ! -x "$APPIMAGETOOL" ]]; then
    curl -fsSL -L -o "$APPIMAGETOOL" \
      "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage"
    chmod +x "$APPIMAGETOOL"
  fi

  APPIMAGE_OUT="${OUTPUT_DIR}/MediaGallery-${VERSION}-linux-x86_64.AppImage"
  ARCH=x86_64 APPIMAGE_EXTRACT_AND_RUN=1 "$APPIMAGETOOL" --no-appstream "$APPDIR" "$APPIMAGE_OUT"
  chmod +x "$APPIMAGE_OUT"
  echo "Created AppImage: $APPIMAGE_OUT"
fi

