#!/usr/bin/env bash
# The release half's own logic, offline (ADR 8.10.26q D4, D5): the CMake
# version file's compatibility rules, and `make cpp-ports` over five zips.
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../.." && pwd)"
py="$(command -v python3 || command -v python)"
cmake="${CMAKE:-cmake}"

# Whether a stamped version file for <have> accepts a request for <want>.
compatible() {
  local tmp=$1 have=$2 want=$3
  sed "s|@CMAKE_VERSION_NUMBER@|$have|g; s|@VERSION@|$have|g" \
    "$repo/cpp/cmake/lingara-embed-config-version.cmake.in" > "$tmp/version.cmake"
  local major minor
  major="${want%%.*}"; minor="${want#*.}"; minor="${minor%%.*}"
  "$cmake" -DPACKAGE_FIND_VERSION="$want" -DPACKAGE_FIND_VERSION_MAJOR="$major" \
    -DPACKAGE_FIND_VERSION_MINOR="$minor" -DCMAKE_SIZEOF_VOID_P=8 \
    -P "$tmp/probe.cmake" 2>&1
}

# 8.10.26q D4
# SameMinorVersion before 1.0.0 and SameMajorVersion from 1.0.0.
test_config_version_compatibility() {
  local tmp; tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' RETURN
  printf 'include("%s/version.cmake")\nmessage("compatible=${PACKAGE_VERSION_COMPATIBLE}")\n' "$tmp" > "$tmp/probe.cmake"
  local have want expect got
  while read -r have want expect; do
    got="$(compatible "$tmp" "$have" "$want")"
    [ "$got" = "compatible=$expect" ] || { echo "FAIL: $have for a request of $want: $got, want $expect"; return 1; }
  done <<'CASES'
0.1.0 0.1.0 TRUE
0.1.3 0.1.0 TRUE
0.2.0 0.1.0 FALSE
0.1.0 0.2.0 FALSE
1.4.0 1.2.0 TRUE
2.0.0 1.2.0 FALSE
1.1.0 1.2.0 FALSE
CASES
}

# 8.10.26q D5
# cpp-ports checks five D4 zips, then writes the port and recipe zips,
# each holding a stamped tree whose URLs name the public Release.
test_cpp_ports_writes_both_zips() {
  local tmp; tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' RETURN
  mkdir -p "$tmp/zips"
  local rid lib root
  for rid in win-x64 osx linux-x64 android-arm64 ios-arm64; do
    root="$tmp/tree-$rid"
    mkdir -p "$root/include" "$root/lib" "$root/bin" "$root/share/lingara-embed"
    touch "$root/LICENSE" "$root/include/lingara_embed.h" "$root/include/lingara_embed.hpp" \
      "$root/share/lingara-embed/lingara-embed-config.cmake" "$root/share/lingara-embed/lingara-embed-config-version.cmake"
    case "$rid" in
      win-x64) touch "$root/bin/lingara_embed.dll" "$root/lib/lingara_embed.dll.lib" ;;
      osx) touch "$root/lib/liblingara_embed.dylib" ;;
      ios-arm64) touch "$root/lib/liblingara_embed.a" ;;
      *) echo "$rid" > "$root/lib/liblingara_embed.so" ;;
    esac
    (cd "$root" && "$py" -m zipfile -c "$tmp/zips/lingara-embed-ffi-v9.9.9-$rid.zip" LICENSE include lib bin share)
  done
  make -s -C "$repo" -f make/cpp.mk cpp-ports ZIPS="$tmp/zips" RELEASE_TAG=v9.9.9 \
    FFI_RIDS="win-x64 osx linux-x64 android-arm64 ios-arm64" CPP_PORTS_BUILD="$tmp/build" > "$tmp/log" 2>&1 ||
    { cat "$tmp/log"; echo "FAIL: cpp-ports"; return 1; }

  lib="$("$py" -c 'import sys, zipfile; print("\n".join(zipfile.ZipFile(sys.argv[1]).namelist()))' "$tmp/zips/lingara-embed-vcpkg-port-v9.9.9.zip")"
  printf '%s\n' "$lib" | grep -qx "ports/lingara-embed/portfile.cmake" || { echo "$lib"; echo "FAIL: the port zip's portfile"; return 1; }
  lib="$("$py" -c 'import sys, zipfile; print("\n".join(zipfile.ZipFile(sys.argv[1]).namelist()))' "$tmp/zips/lingara-embed-conan-recipe-v9.9.9.zip")"
  printf '%s\n' "$lib" | grep -qx "recipes/lingara-embed/all/conandata.yml" || { echo "$lib"; echo "FAIL: the recipe zip's conandata"; return 1; }
  grep -qF "https://github.com/Spinning-Cat-Studios/lingara_embeddable_sdk/releases/download/v9.9.9/lingara-embed-ffi-v9.9.9-osx.zip" \
    "$tmp/build/conan/recipes/lingara-embed/all/conandata.yml" || { echo "FAIL: the recipe does not name the public Release"; return 1; }
}

test_config_version_compatibility
echo "✓ release_ports_test.sh: test_config_version_compatibility"
test_cpp_ports_writes_both_zips
echo "✓ release_ports_test.sh: test_cpp_ports_writes_both_zips"
