#!/usr/bin/env bash
# The pack's two outcomes (ADR 1.10.26aa D8): over a scratch dist/ffi/ holding
# stand-ins for all five natives it writes a nupkg (the "shape pack" that
# PackageShapeTests reads), and with one RID removed it fails naming that RID
# and writes nothing. Run by `make test-dotnet`.
set -euo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
shape="$root/${SHAPE_DIR:-target/dotnet-shape}"
rids=(win-x64 osx linux-x64 android-arm64 ios-arm64)
files=(lingara_embed.dll liblingara_embed.dylib liblingara_embed.so liblingara_embed.so liblingara_embed.a)

stage() {
  rm -rf "$shape" && mkdir -p "$shape"
  local i
  for i in "${!rids[@]}"; do
    mkdir -p "$shape/ffi/${rids[$i]}"
    printf 'stand-in %s\n' "${rids[$i]}" > "$shape/ffi/${rids[$i]}/${files[$i]}"
  done
}

pack() {
  make -s --no-print-directory -C "$root" dotnet-pack FFI_DIST_DIR="$shape/ffi" PACK_OUT="$1/"
}

test_all_five_pack() {
  stage
  pack "$shape/artifacts" > "$shape/pack.log" 2>&1 || { cat "$shape/pack.log"; echo "✗ the pack failed with all five natives"; return 1; }
  ls "$shape"/artifacts/Lingara.Embed.*.nupkg > /dev/null || { echo "✗ no nupkg written"; return 1; }
  echo "✓ all five natives pack"
}

# 1.10.26aa AC29
test_a_missing_rid_refuses_the_pack() {
  rm -rf "$shape/ffi/android-arm64" "$shape/refused"
  if pack "$shape/refused" > "$shape/refused.log" 2>&1; then
    echo "✗ the pack succeeded with android-arm64 missing"
    return 1
  fi
  grep -q "missing the android-arm64 native" "$shape/refused.log" || { cat "$shape/refused.log"; echo "✗ the refusal does not name android-arm64"; return 1; }
  if ls "$shape"/refused/*.nupkg > /dev/null 2>&1; then
    echo "✗ a refused pack wrote a nupkg"
    return 1
  fi
  echo "✓ a missing RID refuses the pack, naming it"
}

test_all_five_pack
test_a_missing_rid_refuses_the_pack
