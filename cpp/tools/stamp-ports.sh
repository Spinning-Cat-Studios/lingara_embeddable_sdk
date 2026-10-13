#!/usr/bin/env bash
# Stamp the vcpkg port and the Conan recipe for one release (ADR 8.10.26q D1).
#
#   stamp-ports.sh --version V --zips DIR --url-base URL --out DIR
#
# A port pins each download's SHA512 and a recipe its sha256, and those exist
# only once the release's zips do, so the committed trees are templates. This
# fills @VERSION@, @URL_BASE@ and one hash token per platform from the five
# lingara-embed-ffi-v<V>-<rid>.zip in --zips, writing
#   <out>/vcpkg/ports/lingara-embed/
#   <out>/conan/recipes/lingara-embed/
# It refuses, naming the platform and writing nothing, when a zip is missing.
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
cpp="$(cd "$here/.." && pwd)"
rids="win-x64 osx linux-x64 android-arm64 ios-arm64"

usage() { echo "Usage: stamp-ports.sh --version V --zips DIR --url-base URL --out DIR" >&2; exit 2; }

version="" zips="" url_base="" out=""
while [ $# -gt 0 ]; do
  case "$1" in
    --version) version="${2:-}"; shift 2 ;;
    --zips) zips="${2:-}"; shift 2 ;;
    --url-base) url_base="${2:-}"; shift 2 ;;
    --out) out="${2:-}"; shift 2 ;;
    *) usage ;;
  esac
done
[ -n "$version" ] && [ -n "$zips" ] && [ -n "$url_base" ] && [ -n "$out" ] || usage

# shasum on macOS, sha<N>sum on Linux and Git Bash.
digest() {
  if command -v "sha$1sum" >/dev/null 2>&1; then "sha$1sum" "$2" | cut -d' ' -f1
  else shasum -a "$1" "$2" | cut -d' ' -f1; fi
}

zip_of() { echo "$zips/lingara-embed-ffi-v$version-$1.zip"; }

# Every zip before any write: a partial tree is worse than none.
for rid in $rids; do
  [ -f "$(zip_of "$rid")" ] || { echo "✗ no $rid zip: $(zip_of "$rid")" >&2; exit 1; }
done

# One sed script for every template: the version, the URL base and ten hashes.
script="$(mktemp)"
trap 'rm -f "$script"' EXIT
{
  echo "s|@VERSION@|$version|g"
  echo "s|@URL_BASE@|${url_base%/}|g"
  for rid in $rids; do
    token="$(echo "$rid" | tr 'a-z-' 'A-Z_')"
    echo "s|@SHA512_${token}@|$(digest 512 "$(zip_of "$rid")" | tr 'A-F' 'a-f')|g"
    echo "s|@SHA256_${token}@|$(digest 256 "$(zip_of "$rid")" | tr 'A-F' 'a-f')|g"
  done
} > "$script"

# Copy a template tree, dropping each .in suffix as it is filled.
stamp_tree() {
  local from=$1 to=$2 file rel
  rm -rf "$to" && mkdir -p "$to"
  (cd "$from" && find . -type f) | while read -r rel; do
    rel="${rel#./}"
    mkdir -p "$to/$(dirname "$rel")"
    case "$rel" in
      *.in) sed -f "$script" "$from/$rel" > "$to/${rel%.in}" ;;
      *) cp "$from/$rel" "$to/$rel" ;;
    esac
  done
}

stamp_tree "$cpp/vcpkg/ports/lingara-embed" "$out/vcpkg/ports/lingara-embed"
stamp_tree "$cpp/conan/recipes/lingara-embed" "$out/conan/recipes/lingara-embed"

# A token the script did not know is a template bug, never a shipped one.
if grep -rnE '@[A-Z0-9_]+@' "$out/vcpkg" "$out/conan" >&2; then
  echo "✗ an unstamped token is left in $out" >&2
  exit 1
fi
echo "✓ stamped lingara-embed $version into $out"
