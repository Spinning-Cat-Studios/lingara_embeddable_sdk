#!/usr/bin/env bash
# One lingara-embed-ffi zip holds D4's install tree (ADR 8.10.26q D4):
#
#   check-ffi-zip.sh <zip> <rid>
#
# include/lingara_embed.h and .hpp, the platform's library under lib/ (the
# DLL under bin/), the CMake package under share/lingara-embed/, and LICENSE,
# which vcpkg_install_copyright and ConanCenter's licenses/ both need.
set -euo pipefail

[ $# -eq 2 ] || { echo "Usage: check-ffi-zip.sh <zip> <rid>" >&2; exit 2; }
zip=$1 rid=$2
[ -f "$zip" ] || { echo "✗ no $zip" >&2; exit 1; }

case "$rid" in
  win-x64) libs="bin/lingara_embed.dll lib/lingara_embed.dll.lib" ;;
  osx) libs="lib/liblingara_embed.dylib" ;;
  linux-x64|android-arm64) libs="lib/liblingara_embed.so" ;;
  ios-arm64) libs="lib/liblingara_embed.a" ;;
  *) echo "✗ unknown rid $rid" >&2; exit 2 ;;
esac

# Python's zipfile, not unzip: Git Bash on Windows has no unzip.
py="$(command -v python3 || command -v python)"
names="$("$py" -c 'import sys, zipfile; print("\n".join(zipfile.ZipFile(sys.argv[1]).namelist()))' "$zip")"

missing=0
for want in LICENSE include/lingara_embed.h include/lingara_embed.hpp \
            share/lingara-embed/lingara-embed-config.cmake share/lingara-embed/lingara-embed-config-version.cmake $libs; do
  if ! printf '%s\n' "$names" | grep -qxF "$want"; then
    echo "✗ $zip: no $want" >&2
    missing=1
  fi
done
[ "$missing" -eq 0 ] || exit 1
echo "✓ $zip: the $rid install tree"
