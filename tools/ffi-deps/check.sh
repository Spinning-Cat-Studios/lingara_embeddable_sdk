#!/usr/bin/env bash
# One dist/ffi/<rid>/ artefact against tools/ffi-deps/allowlist.toml (ADR
# 1.10.26aa D7, AC28): every dynamic import must be on its platform's list,
# and on linux-x64 no GLIBC_ symbol version may exceed `glibc_max`, so the
# floor is checked on the artefact rather than trusted to the build host.
#
#   check.sh <rid> <dir>
#
# FFI_DEPS_IMPORTS=<file> replaces the platform tool's output with a fixture
# (one `NEEDED <lib>` or `GLIBC <x.y>` per line), which check_test.sh uses.
set -euo pipefail

rid="$1"
dir="$2"
here="$(cd "$(dirname "$0")" && pwd)"
allowlist="${FFI_DEPS_ALLOWLIST:-$here/allowlist.toml}"

# The `allow = [...]` line under `[<rid>]`, one name per line.
allowed() {
  awk -v table="[$rid]" '$0 == table {in_table = 1; next} /^\[/ {in_table = 0} in_table && /^allow *=/ {print}' "$allowlist" \
    | { grep -o '"[^"]*"' || true; } | tr -d '"'
}

glibc_max() {
  sed -n 's/^glibc_max *= *"\(.*\)"/\1/p' "$allowlist"
}

artefact() {
  case "$rid" in
    win-x64) echo "$dir/lingara_embed.dll" ;;
    osx) echo "$dir/liblingara_embed.dylib" ;;
    linux-x64 | android-arm64) echo "$dir/liblingara_embed.so" ;;
    ios-arm64) echo "$dir/liblingara_embed.a" ;;
    *) echo "unknown RID $rid" >&2; exit 2 ;;
  esac
}

# `NEEDED <lib>` and `GLIBC <version>` lines, from the platform's own tool.
imports() {
  local file="$1"
  if [ -n "${FFI_DEPS_IMPORTS:-}" ]; then
    cat "$FFI_DEPS_IMPORTS"
    return
  fi
  case "$rid" in
    win-x64) dumpbin /dependents "$file" | sed -n 's/^ *\([A-Za-z0-9_.-]*\.dll\) *$/NEEDED \1/p' ;;
    # Imports are the indented lines: a universal dylib prints one unindented
    # `<file> (architecture …):` header per slice, not one header in all.
    osx) otool -L "$file" | awk '/^[[:space:]]/ {print "NEEDED " $1}' | sort -u ;;
    linux-x64 | android-arm64)
      objdump -p "$file" | awk '$1 == "NEEDED" {print "NEEDED " $2}'
      objdump -T "$file" | grep -o 'GLIBC_[0-9.]*' | sed 's/GLIBC_/GLIBC /' | sort -u || true
      ;;
    ios-arm64) ;;
  esac
}

# Is version $1 above $2 (dotted, numeric)?
above() {
  [ "$1" != "$2" ] && [ "$(printf '%s\n%s\n' "$1" "$2" | sort -t. -k1,1n -k2,2n -k3,3n | tail -1)" = "$1" ]
}

main() {
  local file bad=0 allow ceiling
  file="$(artefact)"
  if [ -z "${FFI_DEPS_IMPORTS:-}" ] && [ ! -f "$file" ]; then
    echo "✗ $rid: no artefact at $file"
    return 1
  fi
  if [ "$rid" = ios-arm64 ] && [ -z "${FFI_DEPS_IMPORTS:-}" ] && ! head -c 7 "$file" | grep -q '!<arch>'; then
    echo "✗ ios-arm64: $file is not a static archive"
    return 1
  fi
  allow="$(allowed)"
  ceiling="$(glibc_max)"
  while read -r kind name; do
    [ -n "$kind" ] || continue
    if [ "$kind" = NEEDED ]; then
      if ! printf '%s\n' "$allow" | grep -qixF -- "$name"; then
        echo "✗ $rid imports $name, which allowlist.toml does not allow"
        bad=1
      fi
    elif [ "$kind" = GLIBC ] && [ "$rid" = linux-x64 ] && above "$name" "$ceiling"; then
      echo "✗ $rid needs GLIBC_$name, above the $ceiling floor"
      bad=1
    fi
  done < <(imports "$file")
  [ "$bad" = 0 ] && echo "✓ $rid imports only its allowlisted libraries"
  return "$bad"
}

main
