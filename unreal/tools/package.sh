#!/usr/bin/env bash
# The plugin's libraries and zips, with no engine (ADR 1.10.26ac D3, D9).
#
#   package.sh stage   <ffi-dist> <lib-dir>
#       dist/ffi/<rid>/ into lib/<Platform>/ for every rid that has been built
#   package.sh package <ffi-dist> <out-dir> <engine-versions.txt> [5.<m>]
#       one dist/unreal/LingaraEmbed-<VERSION>-UE5.<m>.zip per supported
#       engine version (or just the one named), each stamped and passed
#       through check_plugin.sh --zip. Refuses, writing nothing, while any of
#       the five rids is missing.
#
# PLUGIN_DIR, VERSION_FILE and LICENSE_FILE point it at a fixture.
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../.." && pwd)
plugin=${PLUGIN_DIR:-$root/unreal/LingaraEmbed}
rids="win-x64 osx linux-x64 android-arm64 ios-arm64"
lib_in_plugin=Source/ThirdParty/LingaraEmbedLibrary/lib

# K3's rid → the plugin's lib/<Platform> (K8's {ue_platform} map, D3).
platform_of() {
  case $1 in
    win-x64) echo Win64 ;; osx) echo Mac ;; linux-x64) echo Linux ;;
    android-arm64) echo Android/arm64-v8a ;; ios-arm64) echo IOS ;;
  esac
}

# The files each rid ships: the DLL with its import library, never the static .lib.
files_of() {
  case $1 in
    win-x64) echo "lingara_embed.dll lingara_embed.dll.lib" ;; osx) echo liblingara_embed.dylib ;;
    linux-x64 | android-arm64) echo liblingara_embed.so ;; ios-arm64) echo liblingara_embed.a ;;
  esac
}

stage_rid() {
  local dist=$1 lib=$2 rid=$3 dest f
  dest=$lib/$(platform_of "$rid")
  mkdir -p "$dest"
  for f in $(files_of "$rid"); do cp "$dist/$rid/$f" "$dest/"; done
}

stage() {
  local dist=$1 lib=$2 rid staged=0
  for rid in $rids; do
    if [ -d "$dist/$rid" ]; then
      stage_rid "$dist" "$lib" "$rid"
      echo "staged $rid → lib/$(platform_of "$rid")"
      staged=1
    fi
  done
  [ "$staged" -eq 1 ] || { echo "✗ nothing in $dist: run make ffi-dist RID=<rid> first"; exit 1; }
}

missing_rids() {
  local dist=$1 rid f
  for rid in $rids; do
    for f in $(files_of "$rid"); do
      if [ ! -f "$dist/$rid/$f" ]; then echo "$rid"; break; fi
    done
  done
}

# major × 1 000 000 + minor × 1 000 + patch, any pre-release suffix ignored (D9).
version_int() {
  local major minor patch
  IFS=. read -r major minor patch <<< "${1%%-*}"
  echo $((major * 1000000 + minor * 1000 + patch))
}

# The plugin's source without build output, editor state or staged libraries.
copy_source() {
  local dest=$1 f
  (cd "$plugin" && find . -type f ! -path '*/Binaries/*' ! -path '*/Intermediate/*' ! -path '*/Saved/*' \
    ! -path '*/DerivedDataCache/*' ! -path "./$lib_in_plugin/*") | while read -r f; do
    mkdir -p "$dest/$(dirname "$f")"
    cp "$plugin/$f" "$dest/$f"
  done
}

package_one() {
  local dist=$1 out=$2 ue=$3 version=$4 work dest zip rid
  work=$(mktemp -d)
  dest=$work/LingaraEmbed
  mkdir -p "$dest"
  copy_source "$dest"
  cp "${LICENSE_FILE:-$root/LICENSE}" "$dest/LICENSE"
  for rid in $rids; do stage_rid "$dist" "$dest/$lib_in_plugin" "$rid"; done
  jq --arg e "$ue.0" --arg v "$version" --argjson n "$(version_int "$version")" \
    '.EngineVersion = $e | .VersionName = $v | .Version = $n' "$plugin/LingaraEmbed.uplugin" > "$dest/LingaraEmbed.uplugin"
  zip=$out/LingaraEmbed-$version-UE$ue.zip
  rm -f "$zip"
  (cd "$work" && cmake -E tar cf "$zip" --format=zip LingaraEmbed)
  rm -rf "$work"
  echo "$zip"
}

package() {
  local dist=$1 out=$2 versions=$3 only=${4:-} missing version list ue zips=()
  missing=$(missing_rids "$dist" | tr '\n' ' ')
  if [ -n "$missing" ]; then
    echo "✗ no zip written: $dist lacks ${missing% }; run make ffi-dist RID=<rid> for each"
    exit 1
  fi
  version=$(tr -d '[:space:]' < "${VERSION_FILE:-$root/VERSION}")
  list=${only:-$(grep -E '^5\.[0-9]+$' "$versions" | tr -d '\r')}
  [[ "$list" =~ ^5\.[0-9]+ ]] || { echo "✗ no engine version: '$list'"; exit 1; }
  mkdir -p "$out"
  out=$(cd "$out" && pwd)
  for ue in $list; do zips+=("$(package_one "$dist" "$out" "$ue" "$version")"); done
  bash "$here/check_plugin.sh" --zip "${zips[@]}"
  printf '%s\n' "${zips[@]}"
}

case "${1:-}" in
  stage) stage "$2" "$3" ;;
  package) package "$2" "$3" "$4" "${5:-}" ;;
  *) echo "usage: package.sh stage <ffi-dist> <lib-dir> | package <ffi-dist> <out-dir> <versions> [5.<m>]"; exit 2 ;;
esac
