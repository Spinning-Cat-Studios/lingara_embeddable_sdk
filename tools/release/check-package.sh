#!/usr/bin/env bash
# check-package.sh — what a native build and a packed artefact must be (ADR 1.10.26ag D3).
#
#   check-package.sh arch <root> <rid>       every library <rid>'s natives wrote under <root>
#   check-package.sh package <id> <file>     a packed artefact against tools/release/release.toml
#
# `arch` reads each library's format and architecture. On osx the ffi and
# Godot libraries are universal, and `lipo -info` must list exactly x86_64 and
# arm64; the JVM's two are thin (K7 D5) and never handed to lipo: `file` reads
# arm64 under darwin-aarch64/ and x86_64 under darwin-x86-64/. objdump reads
# Android (aarch64 ELF, every LOAD aligned to 16 KB) and iOS (a device slice,
# never the simulator).
#
# `package` opens a .nupkg, zip, jar or AAR (or, for a per-engine bundle,
# each zip inside it) and fails on a rid with nothing under its path, a file
# there whose bytes match nothing its natives-<rid> directory holds, a missing
# LICENSE, or anything under Binaries/, Intermediate/, obj/ or .godot/. The
# natives-<rid> directories are read from the repository root, where the
# workflow downloads them.
#
# FILE_CMD, LIPO and OBJDUMP name the tools, so the offline test can stand in
# for them; RELEASE_TOML points release-toml.sh at a fixture.
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
FILE_CMD=${FILE_CMD:-file}
LIPO=${LIPO:-lipo}
OBJDUMP=${OBJDUMP:-$(command -v llvm-objdump || command -v objdump || echo objdump)}
fails=0

no() { echo "✗ $*"; fails=$((fails + 1)); }
toml() { bash "$here/release-toml.sh" "$@"; }

sha() {
  if command -v sha256sum >/dev/null 2>&1; then sha256sum | cut -d' ' -f1; else shasum -a 256 | cut -d' ' -f1; fi
}

# ── arch ───────────────────────────────────────────────────────────────────

lipo_archs() {
  "$LIPO" -info "$1" | sed -e 's/.* are: //' -e 's/.* is architecture: //' | tr ' ' '\n' | grep -v '^$' | sort | tr '\n' ' '
}

load_aligned_16k() {
  local aligns
  aligns=$("$OBJDUMP" -p "$1" | awk '$1 == "LOAD" { print $NF }')
  [ -n "$aligns" ] || return 1
  ! echo "$aligns" | grep -qvE '^2\*\*(1[4-9]|[2-9][0-9])$'
}

ios_device_only() {
  local platforms
  platforms=$("$OBJDUMP" --macho --private-headers "$1" | awk '$1 == "platform" { print $2 }' | sort -u)
  [ -n "$platforms" ] && ! echo "$platforms" | grep -qvE '^(ios|2)$'
}

check_osx() {
  local lib=$1 desc=$2
  case "$lib" in
    */darwin-aarch64/*) [[ "$desc" == *arm64* && "$desc" != *universal* ]] || no "$lib: not a thin arm64 library: $desc" ;;
    */darwin-x86-64/*) [[ "$desc" == *x86_64* && "$desc" != *universal* ]] || no "$lib: not a thin x86_64 library: $desc" ;;
    *) [ "$(lipo_archs "$lib")" = "arm64 x86_64 " ] || no "$lib: lipo -info lists $(lipo_archs "$lib"); want exactly x86_64 arm64" ;;
  esac
}

check_library() {
  local rid=$1 lib=$2 desc=$3
  case "$rid" in
    osx) check_osx "$lib" "$desc" ;;
    ios-arm64)
      [[ "$desc" == *arm64* || "$desc" == *"ar archive"* ]] || no "$lib: not arm64: $desc"
      ios_device_only "$lib" || no "$lib: not a device slice (a simulator platform, or none)" ;;
    android-arm64)
      [[ "$desc" == *ELF\ 64-bit* && "$desc" == *aarch64* ]] || no "$lib: not an aarch64 ELF: $desc"
      load_aligned_16k "$lib" || no "$lib: a LOAD segment is aligned below 16 KB" ;;
    linux-x64) [[ "$desc" == *ELF\ 64-bit* && "$desc" == *x86-64* ]] || no "$lib: not an x86-64 ELF: $desc" ;;
    win-x64) [[ "$desc" == *PE32+* && "$desc" == *x86-64* ]] || no "$lib: not an x86-64 PE32+ DLL: $desc" ;;
  esac
}

# A file is a library when `file` says it is code; import libraries (.lib)
# are archives of stubs, read only on iOS, where the static library is one.
is_library() {
  case "$2" in
    *Mach-O* | *ELF* | *PE32*) return 0 ;;
    *"ar archive"*) [ "$1" = ios-arm64 ] ;;
    *) return 1 ;;
  esac
}

arch() {
  local root=$1 rid=$2 outs out lib desc found
  # Read first: a failing `< <(toml …)` is invisible to set -e, and an empty
  # list passes vacuously.
  outs=$(toml outs "$rid") || exit 2
  [ -n "$outs" ] || { no "$rid: release.toml names no native"; return; }
  while read -r out; do
    [ -d "$root/$out" ] || { no "$root/$out: not built"; continue; }
    found=0
    while read -r lib; do
      desc=$("$FILE_CMD" -b "$lib")
      is_library "$rid" "$desc" || continue
      check_library "$rid" "$lib" "$desc"
      found=1
    done < <(find "$root/$out" -type f | sort)
    [ "$found" -eq 1 ] || no "$root/$out: holds no library"
  done <<< "$outs"
}

# ── package ────────────────────────────────────────────────────────────────

forbidden() {
  grep -E '(^|/)(Binaries|Intermediate|obj|\.godot)/' || true
}

check_rid() {
  local id=$1 zip=$2 entries=$3 rid=$4 native paths path under entry hashes found
  native=$(toml package-native "$id" "$rid") || exit 2
  paths=$(toml package "$id" "$rid") || exit 2
  [ -d "$native" ] || { no "$native: the natives-$rid bytes to compare with are absent"; return; }
  hashes=$(while read -r f; do sha < "$f"; done < <(find "$native" -type f))
  while read -r path; do
    under=$(echo "$entries" | grep -E "(^|/)${path//./\\.}[^/]" | grep -v '/$' || true)
    [ -n "$under" ] || { no "$zip: nothing under $path: the $rid natives are missing"; continue; }
    while read -r entry; do
      found=$(unzip -p "$zip" "$entry" | sha)
      echo "$hashes" | grep -qx "$found" || no "$zip: $entry differs from every file in $native"
    done <<< "$under"
  done <<< "$paths"
}

check_zip() {
  local id=$1 zip=$2 entries bad rid
  entries=$(unzip -Z1 "$zip")
  echo "$entries" | grep -qE '(^|/)LICENSE$' || no "$zip: no LICENSE"
  bad=$(echo "$entries" | forbidden)
  [ -z "$bad" ] || no "$zip: build output inside: $(echo "$bad" | head -3 | tr '\n' ' ')"
  for rid in $(toml package-rids "$id"); do check_rid "$id" "$zip" "$entries" "$rid"; done
}

package() {
  local id=$1 file=$2 work inner
  [ -f "$file" ] || { no "$file: not packed"; return; }
  if [ "$(toml per-engine "$id")" = true ]; then
    work=$(mktemp -d)
    unzip -q "$file" -d "$work"
    inner=$(find "$work" -name '*.zip' | sort)
    [ -n "$inner" ] || no "$file: a bundle with no per-engine zip"
    for zip in $inner; do check_zip "$id" "$zip"; done
    rm -rf "$work"
  else
    check_zip "$id" "$file"
  fi
}

case "${1:-}" in
  arch) [ $# -eq 3 ] || { echo "usage: check-package.sh arch <root> <rid>" >&2; exit 2; }; arch "$2" "$3" ;;
  package) [ $# -eq 3 ] || { echo "usage: check-package.sh package <id> <file>" >&2; exit 2; }; package "$2" "$3" ;;
  *) sed -n '4,5p' "$0" >&2; exit 2 ;;
esac

[ "$fails" -eq 0 ] || { echo "✗ check-package.sh $1: $fails finding(s)"; exit 1; }
echo "✓ check-package.sh $1 ${*:2}"
