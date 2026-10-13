#!/usr/bin/env bash
# check-ffi-zip.sh over fixture zips, offline (ADR 8.10.26q D4).
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
check="$here/check-ffi-zip.sh"
py="$(command -v python3 || command -v python)"

# A zip of D4's tree for <rid> at <zip>, less any path named after it.
write_zip() {
  local zip=$1 rid=$2; shift 2
  local root; root="$(mktemp -d)"
  mkdir -p "$root/include" "$root/lib" "$root/bin" "$root/share/lingara-embed"
  touch "$root/LICENSE" "$root/include/lingara_embed.h" "$root/include/lingara_embed.hpp" \
        "$root/share/lingara-embed/lingara-embed-config.cmake" "$root/share/lingara-embed/lingara-embed-config-version.cmake"
  case "$rid" in
    win-x64) touch "$root/bin/lingara_embed.dll" "$root/lib/lingara_embed.dll.lib" ;;
    osx) touch "$root/lib/liblingara_embed.dylib" ;;
    ios-arm64) touch "$root/lib/liblingara_embed.a" ;;
    *) touch "$root/lib/liblingara_embed.so" ;;
  esac
  local drop
  for drop in "$@"; do rm "$root/$drop"; done
  rm -f "$zip"
  # Whatever is left at the root, LICENSE included unless it was dropped.
  local top; top="$(cd "$root" && ls)"
  # shellcheck disable=SC2086 # one word per top-level entry
  (cd "$root" && "$py" -m zipfile -c "$zip" $top)
  rm -rf "$root"
}

# 8.10.26q AC2
# A complete zip passes; one missing LICENSE, the C++ header, the CMake
# config or the platform's library fails and names the path.
test_zip_layout_rejects_each_defect() {
  local tmp; tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' RETURN
  local rid
  for rid in win-x64 osx linux-x64 android-arm64 ios-arm64; do
    write_zip "$tmp/ok-$rid.zip" "$rid"
    bash "$check" "$tmp/ok-$rid.zip" "$rid" > "$tmp/log" 2>&1 || { cat "$tmp/log"; echo "FAIL: a complete $rid zip"; return 1; }
  done

  local defect
  for defect in LICENSE include/lingara_embed.hpp share/lingara-embed/lingara-embed-config.cmake lib/liblingara_embed.so; do
    write_zip "$tmp/bad.zip" linux-x64 "$defect"
    if bash "$check" "$tmp/bad.zip" linux-x64 > "$tmp/log" 2>&1; then echo "FAIL: passed without $defect"; return 1; fi
    grep -qF "no $defect" "$tmp/log" || { cat "$tmp/log"; echo "FAIL: the failure does not name $defect"; return 1; }
  done

  write_zip "$tmp/bad.zip" win-x64 bin/lingara_embed.dll
  if bash "$check" "$tmp/bad.zip" win-x64 > "$tmp/log" 2>&1; then echo "FAIL: passed without the DLL"; return 1; fi
  grep -qF "no bin/lingara_embed.dll" "$tmp/log" || { cat "$tmp/log"; echo "FAIL: the failure does not name the DLL"; return 1; }
}

test_zip_layout_rejects_each_defect
echo "✓ check_ffi_zip_test.sh: test_zip_layout_rejects_each_defect"
