#!/usr/bin/env bash
# package.sh over stand-in libraries (ADR 1.10.26ac AC16): one stamped zip
# per line of a fixture engine-versions.txt, each passing check_plugin.sh;
# with one rid missing, no zip and the rid named.
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

# Stand-ins for all five dist/ffi/<rid>/ directories.
fake_dist() {
  local dist=$1
  mkdir -p "$dist"/{win-x64,osx,linux-x64,android-arm64,ios-arm64}
  for f in win-x64/lingara_embed.dll win-x64/lingara_embed.dll.lib osx/liblingara_embed.dylib \
           linux-x64/liblingara_embed.so android-arm64/liblingara_embed.so ios-arm64/liblingara_embed.a; do
    echo "stand-in" > "$dist/$f"
  done
}

uplugin_of() {
  local zip=$1 into=$2
  mkdir -p "$into"
  (cd "$into" && cmake -E tar xf "$zip")
  echo "$into/LingaraEmbed/LingaraEmbed.uplugin"
}

# 1.10.26ac AC16
test_one_zip_per_engine_version_and_a_missing_rid_refuses() {
  fake_dist "$work/dist"
  printf '5.6\n5.7\n' > "$work/engine-versions.txt"
  echo "1.2.3-alpha.1" > "$work/VERSION"
  export VERSION_FILE=$work/VERSION
  bash "$here/package.sh" package "$work/dist" "$work/out" "$work/engine-versions.txt" > /dev/null
  for ue in 5.6 5.7; do
    local zip=$work/out/LingaraEmbed-1.2.3-alpha.1-UE$ue.zip descriptor
    [ -f "$zip" ] || { echo "✗ no $zip"; return 1; }
    descriptor=$(uplugin_of "$zip" "$work/x$ue")
    [ "$(jq -r .EngineVersion "$descriptor" | tr -d '\r')" = "$ue.0" ] || { echo "✗ $ue: EngineVersion"; return 1; }
    [ "$(jq -r .Version "$descriptor" | tr -d '\r')" = 1002003 ] || { echo "✗ $ue: Version"; return 1; }
    [ "$(jq -r .VersionName "$descriptor" | tr -d '\r')" = 1.2.3-alpha.1 ] || { echo "✗ $ue: VersionName"; return 1; }
  done
  [ "$(ls "$work/out" | wc -l | tr -d ' ')" = 2 ] || { echo "✗ not one zip per engine version"; return 1; }

  rm -rf "$work/dist/android-arm64"
  local out
  if out=$(bash "$here/package.sh" package "$work/dist" "$work/refused" "$work/engine-versions.txt" 2>&1); then
    echo "✗ packaged with android-arm64 missing"
    return 1
  fi
  case "$out" in *android-arm64*) ;; *) echo "✗ the refusal does not name the rid: $out"; return 1;; esac
  [ ! -e "$work/refused" ] || [ -z "$(ls "$work/refused")" ] || { echo "✗ a zip was written"; return 1; }
}

test_one_zip_per_engine_version_and_a_missing_rid_refuses
echo "✓ package_test: one stamped zip per engine version; a missing rid writes none and is named"
