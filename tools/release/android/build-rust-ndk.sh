#!/usr/bin/env bash
set -euo pipefail

NDK_VERSION="${ANDROID_NDK_VERSION:-28.2.13676358}"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
SDK_ROOT="${ANDROID_SDK_ROOT:-${ANDROID_HOME:-}}"

if [[ -z "$SDK_ROOT" || ! -d "$SDK_ROOT" ]]; then
  echo "ANDROID_SDK_ROOT / ANDROID_HOME is not configured." >&2
  exit 1
fi

if ! command -v cargo-ndk >/dev/null 2>&1; then
  cargo install cargo-ndk --locked
fi

NDK_DIR="${SDK_ROOT}/ndk/${NDK_VERSION}"
if [[ ! -d "$NDK_DIR" ]]; then
  SDK_MANAGER="$(find "${SDK_ROOT}/cmdline-tools" -type f \( -name 'sdkmanager' -o -name 'sdkmanager.bat' \) 2>/dev/null | head -n 1 || true)"
  if [[ -z "$SDK_MANAGER" ]]; then
    echo "sdkmanager not found under ${SDK_ROOT}/cmdline-tools" >&2
    exit 1
  fi
  yes | "$SDK_MANAGER" --licenses >/dev/null 2>&1 || true
  "$SDK_MANAGER" "ndk;${NDK_VERSION}"
fi

export ANDROID_NDK_HOME="$NDK_DIR"
export CARGO_PROFILE_RELEASE_LTO="${CARGO_PROFILE_RELEASE_LTO:-thin}"

cd "$REPO_ROOT"
cargo ndk \
  -t arm64-v8a \
  -t armeabi-v7a \
  -t x86_64 \
  -o app/android/app/src/main/jniLibs \
  build --release --locked -p core-api

