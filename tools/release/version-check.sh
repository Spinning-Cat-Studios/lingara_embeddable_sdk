#!/usr/bin/env bash
# version-check.sh — every desktop library a natives leg built reports VERSION (ADR 1.10.26ag D3).
#
#   version-check.sh <rid>                         every library of <rid>'s natives, from the repo root
#   version-check.sh lib <file> <symbol> <want>    one library: <symbol>() must return <want>
#
# K3's lingara_embed_version() and K7's lingara_embed_uniffi_version() are
# plain C, `const char *(void)`, so python3's ctypes loads each library and
# calls its symbol. A library built from another commit fails here, before it
# is packed. A runner loads only its own architecture, so a thin library for
# the other one (the JVM's darwin-x86-64/ on an arm64 macOS runner) is left to
# check-package.sh arch. K4's Godot library has no such symbol (D10).
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
py=$(command -v python3 || command -v python || true)
[ -n "$py" ] || { echo "version-check.sh: python3 is required" >&2; exit 2; }

one() {
  "$py" - "$1" "$2" "$3" <<'PY'
import ctypes, os, sys
# Windows' console encoding is cp1252, which has no ✓ or ✗.
sys.stdout.reconfigure(encoding="utf-8")
sys.stderr.reconfigure(encoding="utf-8")
path, symbol, want = sys.argv[1:]
fn = getattr(ctypes.CDLL(os.path.abspath(path)), symbol)
fn.restype, fn.argtypes = ctypes.c_char_p, []
got = fn().decode()
if got != want:
    sys.exit(f"✗ {path}: {symbol}() is {got!r}, VERSION is {want!r}")
print(f"✓ {path}: {symbol}() is {got}")
PY
}

# This host's JNA directory names, so the other macOS slice is skipped.
host_arch() {
  case "$(uname -m)" in arm64 | aarch64) echo aarch64 ;; *) echo x86-64 ;; esac
}

rid_libraries() {
  local rid=$1 outs out
  # Read first: a failing `< <(…)` is invisible to set -e.
  outs=$(bash "$here/release-toml.sh" outs "$rid") || exit 2
  while read -r out; do
    [ -d "$out" ] || { echo "✗ $out: not built" >&2; exit 1; }
    find "$out" -type f \( -name '*.dll' -o -name '*.dylib' -o -name '*.so' \) | sort
  done <<< "$outs"
}

check_rid() {
  local rid=$1 want lib checked=0
  want=$(tr -d '[:space:]' < VERSION)
  case "$rid" in win-x64 | osx | linux-x64) ;; *) echo "- $rid: not loadable on a desktop runner"; return 0 ;; esac
  while read -r lib; do
    case "$lib" in
      */bin/*) continue ;;                                  # K4's Godot library: no version symbol
      *lingara_embed_uniffi*)
        case "$lib" in */darwin-*) [[ "$lib" == */darwin-"$(host_arch)"/* ]] || continue ;; esac
        one "$lib" lingara_embed_uniffi_version "$want" ;;
      *lingara_embed.*) one "$lib" lingara_embed_version "$want" ;;
      *) continue ;;
    esac
    checked=$((checked + 1))
  done < <(rid_libraries "$rid")
  [ "$checked" -gt 0 ] || { echo "✗ $rid: no library carried a version symbol" >&2; exit 1; }
}

case "${1:-}" in
  lib) [ $# -eq 4 ] || { echo "usage: version-check.sh lib <file> <symbol> <want>" >&2; exit 2; }; one "$2" "$3" "$4" ;;
  "") echo "usage: version-check.sh <rid> | lib <file> <symbol> <want>" >&2; exit 2 ;;
  *) check_rid "$1" ;;
esac
