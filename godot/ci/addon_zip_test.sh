#!/usr/bin/env bash
# `make godot-addon-zip`'s two rules (ADR 1.10.26ab D6, AC8), over a copy of
# the committed addon with a fixture bin/, so the real tree is never touched:
# the zip holds addons/lingara/** and nothing else, LICENSE included, and it
# refuses while any K8 rid is missing.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
rids="win-x64 osx linux-x64 android-arm64 ios-arm64"

# 1.10.26ab AC8.
test_zip_holds_only_the_addon_and_refuses_a_missing_rid() {
  local work
  work=$(mktemp -d)
  trap 'rm -rf "$work"' RETURN
  mkdir -p "$work/addon/addons"
  cp -R "$root/godot/addon/addons/lingara" "$work/addon/addons/"
  rm -rf "$work/addon/addons/lingara/bin"
  echo "stray" > "$work/addon/ASSETLIB.md"

  for rid in $rids; do mkdir -p "$work/addon/addons/lingara/bin/$rid"; done
  for rid in win-x64 osx linux-x64 android-arm64; do echo "fixture" > "$work/addon/addons/lingara/bin/$rid/lib"; done
  if bash "$root/godot/ci/make-addon-zip.sh" "$work/addon" "$rids" 2>"$work/err"; then
    echo "✗ a zip was written with ios-arm64 missing"; return 1
  fi
  grep -q "ios-arm64" "$work/err" || { echo "✗ the refusal does not name the missing rid"; cat "$work/err"; return 1; }

  echo "fixture" > "$work/addon/addons/lingara/bin/ios-arm64/lib"
  bash "$root/godot/ci/make-addon-zip.sh" "$work/addon" "$rids" >/dev/null
  local version zip
  version=$(sed -n 's/^version *= *"\(.*\)"$/\1/p' "$root/godot/addon/addons/lingara/plugin.cfg")
  zip="$work/addon/dist/lingara-godot-addon-$version.zip"
  [ -f "$zip" ] || { echo "✗ no $zip"; return 1; }
  local listed
  listed=$(unzip -Z1 "$zip")
  if echo "$listed" | grep -v '^addons/lingara/' | grep -q .; then
    echo "✗ the zip carries more than addons/lingara/:"; echo "$listed" | grep -v '^addons/lingara/'; return 1
  fi
  echo "$listed" | grep -qx 'addons/lingara/LICENSE' || { echo "✗ no addons/lingara/LICENSE in the zip"; return 1; }
  for rid in $rids; do
    echo "$listed" | grep -q "^addons/lingara/bin/$rid/" || { echo "✗ no $rid in the zip"; return 1; }
  done
  cmp -s "$root/LICENSE" "$root/godot/addon/addons/lingara/LICENSE" || { echo "✗ the addon's LICENSE is not byte-equal to the root's"; return 1; }
  echo "✓ the zip holds only addons/lingara/**, LICENSE included, and refuses a missing rid"
}

test_zip_holds_only_the_addon_and_refuses_a_missing_rid
