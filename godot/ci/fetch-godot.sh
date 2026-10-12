#!/usr/bin/env bash
# `make godot-fetch` (ADR 1.10.26ab D10): the official Linux Godot binary for
# one version in godot-versions.toml, downloaded into the cache once and
# checked against its pinned SHA-256 every time. Prints only the binary's
# path on stdout; progress goes to stderr, because make/godot.mk reads stdout
# as $(GODOT).
#
#   fetch-godot.sh <godot-versions.toml> <version> <cache-dir>
set -euo pipefail

pins=${1:?the pins file}
version=${2:?a version}
cache=${3:?the cache directory}

if [ "$(uname -s)" != "Linux" ] || [ "$(uname -m)" != "x86_64" ]; then
  echo "✗ godot-fetch fetches the Linux x86_64 binary CI runs; on this host pass GODOT=<your Godot $version binary>" >&2
  exit 1
fi

# One [[godot]] table's key, for the matching version.
field() {
  awk -v want="$version" -v key="$1" '
    /^\[\[godot\]\]/ { inside = 0 }
    $1 == "version" { gsub(/"/, "", $3); inside = ($3 == want) }
    inside && $1 == key { gsub(/"/, "", $3); print $3; exit }
  ' "$pins"
}

url=$(field url)
sha=$(field sha256)
binary=$(field binary)
[ -n "$url" ] && [ -n "$sha" ] && [ -n "$binary" ] || { echo "✗ no pin for Godot $version in $pins" >&2; exit 1; }

dir="$cache/$version"
zip="$dir/$(basename "$url")"
mkdir -p "$dir"
if [ ! -f "$zip" ]; then
  echo "fetching Godot $version" >&2
  curl -fsSL --retry 3 -o "$zip.part" "$url" >&2
  mv "$zip.part" "$zip"
fi
got=$(sha256sum "$zip" | cut -d' ' -f1)
if [ "$got" != "$sha" ]; then
  rm -f "$zip"
  echo "✗ Godot $version: sha256 $got, pinned $sha" >&2
  exit 1
fi
if [ ! -x "$dir/$binary" ]; then
  unzip -oq "$zip" -d "$dir" >&2
  chmod +x "$dir/$binary"
fi
echo "$(cd "$dir" && pwd)/$binary"
