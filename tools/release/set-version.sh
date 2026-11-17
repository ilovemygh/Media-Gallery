#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 || $# -gt 2 ]]; then
  echo "Usage: $0 <version> [build_number]" >&2
  exit 1
fi

VERSION="${1#v}"
BUILD_NUMBER="${2:-0}"

if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]]; then
  echo "Invalid SemVer version '$VERSION'. Expected X.Y.Z or X.Y.Z-prerelease." >&2
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
CARGO_TOML="${REPO_ROOT}/Cargo.toml"
PUBSPEC_YAML="${REPO_ROOT}/app/pubspec.yaml"

if [[ ! -f "$CARGO_TOML" ]]; then
  echo "Missing Cargo.toml at $CARGO_TOML" >&2
  exit 1
fi
if [[ ! -f "$PUBSPEC_YAML" ]]; then
  echo "Missing app/pubspec.yaml at $PUBSPEC_YAML" >&2
  exit 1
fi

CURRENT_LINE="$(grep -E '^version:[[:space:]]*' "$PUBSPEC_YAML" | head -n 1)"
CURRENT_BUILD="$(echo "$CURRENT_LINE" | sed -E 's/^version:[[:space:]]*[^+]+(\+([0-9]+))?[[:space:]]*$/\2/')"
if [[ -z "$CURRENT_BUILD" ]]; then
  CURRENT_BUILD=1
fi

if [[ "$BUILD_NUMBER" -le 0 ]]; then
  BUILD_NUMBER=$((CURRENT_BUILD + 1))
fi

python3 - "$CARGO_TOML" "$PUBSPEC_YAML" "$VERSION" "$BUILD_NUMBER" <<'PY'
import pathlib, re, sys

cargo_path = pathlib.Path(sys.argv[1])
pubspec_path = pathlib.Path(sys.argv[2])
version = sys.argv[3]
build_number = sys.argv[4]

cargo_text = cargo_path.read_text(encoding="utf-8")
cargo_pattern = re.compile(r'(\[workspace\.package\]\r?\n(?:[^\[]*?\r?\n)?version\s*=\s*")([^"]+)(")', re.S)
new_cargo, count = cargo_pattern.subn(rf'\g<1>{version}\g<3>', cargo_text, count=1)
if count != 1:
    raise SystemExit("Failed to update [workspace.package] version in Cargo.toml")
cargo_path.write_text(new_cargo.replace("\r\n", "\n"), encoding="utf-8", newline="\n")

pubspec_text = pubspec_path.read_text(encoding="utf-8")
pubspec_pattern = re.compile(r'^version:\s*[^\r\n]+', re.M)
new_pubspec, pcount = pubspec_pattern.subn(f'version: {version}+{build_number}', pubspec_text, count=1)
if pcount != 1:
    raise SystemExit("Failed to update version in app/pubspec.yaml")
pubspec_path.write_text(new_pubspec.replace("\r\n", "\n"), encoding="utf-8", newline="\n")
PY

echo "Updated version across workspace:"
echo "  Cargo.toml:       ${VERSION}"
echo "  app/pubspec.yaml: ${VERSION}+${BUILD_NUMBER}"
echo ""
echo "Next steps:"
echo "  git add Cargo.toml app/pubspec.yaml"
echo "  git commit -m \"chore(release): bump version to v${VERSION}\""
echo "  git tag -a v${VERSION} -m \"Release v${VERSION}\""
echo "  git push origin main --follow-tags"
