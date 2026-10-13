#!/usr/bin/env bash
# stamp-ports.sh over five fixture zips, offline (ADR 8.10.26q D1).
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
stamp="$here/stamp-ports.sh"
rids="win-x64 osx linux-x64 android-arm64 ios-arm64"
py="$(command -v python3 || command -v python)"

digest() {
  if command -v "sha$1sum" >/dev/null 2>&1; then "sha$1sum" "$2" | cut -d' ' -f1
  else shasum -a "$1" "$2" | cut -d' ' -f1; fi
}

# A zip per rid whose bytes are its own, so each hash is distinct.
write_zips() {
  local dir=$1 rid
  mkdir -p "$dir"
  for rid in $rids; do
    echo "fixture $rid" > "$dir/payload-$rid.txt"
    "$py" -m zipfile -c "$dir/lingara-embed-ffi-v9.9.9-alpha.1-$rid.zip" "$dir/payload-$rid.txt"
  done
}

# 8.10.26q AC1
# Five fixture zips stamp a portfile holding five lowercase SHA512s and a
# conandata.yml holding five sha256s, each its zip's, with every @ token
# replaced; with one rid's zip missing the stamper writes nothing and names it.
test_five_rids_stamped_and_a_missing_rid_refuses() {
  local tmp; tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' RETURN
  write_zips "$tmp/zips"
  bash "$stamp" --version 9.9.9-alpha.1 --zips "$tmp/zips" --url-base https://example.test/dl/ --out "$tmp/out" > "$tmp/log" 2>&1 ||
    { cat "$tmp/log"; echo "FAIL: stamping five zips"; return 1; }

  local port="$tmp/out/vcpkg/ports/lingara-embed" recipe="$tmp/out/conan/recipes/lingara-embed" rid zip
  for f in "$port/portfile.cmake" "$port/vcpkg.json" "$port/usage" "$recipe/config.yml" \
           "$recipe/all/conandata.yml" "$recipe/all/conanfile.py" "$recipe/all/test_package/test_package.cpp"; do
    [ -f "$f" ] || { echo "FAIL: no $f"; return 1; }
  done
  if find "$tmp/out" -name '*.in' | grep -q .; then echo "FAIL: a .in file was copied unstamped"; return 1; fi
  if grep -rnE '@[A-Z0-9_]+@' "$tmp/out"; then echo "FAIL: a token is left"; return 1; fi
  for rid in $rids; do
    zip="$tmp/zips/lingara-embed-ffi-v9.9.9-alpha.1-$rid.zip"
    grep -qx "  set(sha512 $(digest 512 "$zip"))" "$port/portfile.cmake" ||
      { echo "FAIL: the portfile lacks $rid's SHA512"; return 1; }
    grep -qx "      sha256: \"$(digest 256 "$zip")\"" "$recipe/all/conandata.yml" ||
      { echo "FAIL: conandata.yml lacks $rid's sha256"; return 1; }
    grep -qF "url: \"https://example.test/dl/lingara-embed-ffi-v9.9.9-alpha.1-$rid.zip\"" "$recipe/all/conandata.yml" ||
      { echo "FAIL: conandata.yml's $rid url"; return 1; }
  done
  [ "$(grep -cE '^  set\(sha512 [0-9a-f]{128}\)$' "$port/portfile.cmake")" = 5 ] ||
    { echo "FAIL: want five lowercase SHA512s"; return 1; }
  grep -q '"version-semver": "9.9.9-alpha.1"' "$port/vcpkg.json" || { echo "FAIL: vcpkg.json's version"; return 1; }
  grep -q '"9.9.9-alpha.1":' "$recipe/config.yml" || { echo "FAIL: config.yml's version"; return 1; }

  rm "$tmp/zips/lingara-embed-ffi-v9.9.9-alpha.1-android-arm64.zip"
  if bash "$stamp" --version 9.9.9-alpha.1 --zips "$tmp/zips" --url-base https://example.test --out "$tmp/out2" > "$tmp/log" 2>&1; then
    echo "FAIL: stamped with android-arm64's zip missing"; return 1
  fi
  grep -q "no android-arm64 zip" "$tmp/log" || { cat "$tmp/log"; echo "FAIL: the refusal does not name android-arm64"; return 1; }
  [ ! -e "$tmp/out2" ] || { echo "FAIL: a refused stamp wrote $tmp/out2"; return 1; }
}

test_five_rids_stamped_and_a_missing_rid_refuses
echo "✓ stamp_ports_test.sh: test_five_rids_stamped_and_a_missing_rid_refuses"
