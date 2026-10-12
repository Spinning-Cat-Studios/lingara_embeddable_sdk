#!/usr/bin/env bash
# check-package.sh over fixture natives and packages, offline (ADR 1.10.26ag D3).
#
# A fixture library is a text file carrying, beside a line that makes its
# bytes its own, the lines `file -b`, `lipo -info` and `objdump` would print
# for it, in those tools' own formats; FILE_CMD, LIPO and OBJDUMP are stand-ins
# that print them. So every host runs the same cases, with no cross toolchain.
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../.." && pwd)"
check="$here/check-package.sh"
rids="win-x64 osx linux-x64 android-arm64 ios-arm64"

# The three stand-ins, in $1; lipo also notes each file it is handed.
write_tools() {
  mkdir -p "$1"
  printf '#!/bin/sh\nshift\nd=$(sed -n "s/^#file //p" "$1" | head -1)\necho "${d:-ASCII text}"\n' > "$1/file"
  printf '#!/bin/sh\necho "$2" >> "%s/lipo.calls"\nsed -n "s/^#lipo //p" "$2"\n' "$1" > "$1/lipo"
  printf '#!/bin/sh\nfor f; do :; done\nsed -n "s/^#objdump //p" "$f"\n' > "$1/objdump"
  chmod +x "$1/file" "$1/lipo" "$1/objdump"
}

# lib <path> <file line> [extra marker lines...]
lib() {
  local path=$1 desc=$2; shift 2
  mkdir -p "$(dirname "$path")"
  { echo "fixture $path"; echo "#file $desc"; for l in "$@"; do echo "$l"; done; } > "$path"
}

universal() { lib "$1" "Mach-O universal binary with 2 architectures: [x86_64] [arm64]" "#lipo Architectures in the fat file: $1 are: x86_64 arm64"; }
aligned() { lib "$1" "ELF 64-bit LSB shared object, ARM aarch64, version 1 (SYSV), dynamically linked" "#objdump     LOAD off    0x0 vaddr 0x0 paddr 0x0 align 2**14" "#objdump     LOAD off    0x1 vaddr 0x1 paddr 0x1 align 2**14"; }
device() { lib "$1" "$2" "#objdump   platform ios"; }

# Every native release.toml names, for every rid, under $1: the shape
# release-natives leaves on a leg, all five legs merged.
write_natives() {
  local r=$1
  lib "$r/dist/ffi/win-x64/lingara_embed.dll" "PE32+ executable (DLL) (console) x86-64, for MS Windows"
  lib "$r/dist/ffi/win-x64/lingara_embed.dll.lib" "current ar archive"
  universal "$r/dist/ffi/osx/liblingara_embed.dylib"
  lib "$r/dist/ffi/linux-x64/liblingara_embed.so" "ELF 64-bit LSB shared object, x86-64, version 1 (SYSV), dynamically linked"
  aligned "$r/dist/ffi/android-arm64/liblingara_embed.so"
  device "$r/dist/ffi/ios-arm64/liblingara_embed.a" "current ar archive"
  for rid in $rids; do echo "/* header */" > "$r/dist/ffi/$rid/lingara_embed.h"; done
  lib "$r/godot/addon/addons/lingara/bin/win-x64/lingara_godot.dll" "PE32+ executable (DLL) (GUI) x86-64, for MS Windows"
  universal "$r/godot/addon/addons/lingara/bin/osx/liblingara_godot.framework/liblingara_godot"
  echo "<plist/>" > "$r/godot/addon/addons/lingara/bin/osx/liblingara_godot.framework/Resources.plist"
  lib "$r/godot/addon/addons/lingara/bin/linux-x64/liblingara_godot.so" "ELF 64-bit LSB shared object, x86-64, dynamically linked"
  aligned "$r/godot/addon/addons/lingara/bin/android-arm64/liblingara_godot.so"
  device "$r/godot/addon/addons/lingara/bin/ios-arm64/lingara_godot.xcframework/ios-arm64/lingara_godot.framework/lingara_godot" \
    "Mach-O 64-bit dynamically linked shared library arm64"
  lib "$r/dist/jvm/win-x64/win32-x86-64/lingara_embed_uniffi.dll" "PE32+ executable (DLL) (console) x86-64, for MS Windows"
  lib "$r/dist/jvm/osx/darwin-aarch64/liblingara_embed_uniffi.dylib" "Mach-O 64-bit dynamically linked shared library arm64"
  lib "$r/dist/jvm/osx/darwin-x86-64/liblingara_embed_uniffi.dylib" "Mach-O 64-bit dynamically linked shared library x86_64"
  lib "$r/dist/jvm/linux-x64/linux-x86-64/liblingara_embed_uniffi.so" "ELF 64-bit LSB shared object, x86-64, dynamically linked"
  aligned "$r/dist/jvm/android-arm64/arm64-v8a/liblingara_embed_uniffi.so"
}

# A zip at $1 of staging directory $2's contents.
zipdir() { rm -f "$1"; (cd "$2" && zip -qr "$1" .); }

# Each package's staging directory under $2, from the natives under $1.
stage_nupkg() {
  local r=$1 s=$2 rid
  mkdir -p "$s/lib/net8.0" && echo dll > "$s/lib/net8.0/Lingara.Embed.dll" && cp "$repo/LICENSE" "$s/LICENSE"
  for rid in $rids; do
    mkdir -p "$s/runtimes/$rid/native"
    find "$r/dist/ffi/$rid" -type f ! -name '*.h' ! -name '*.lib' -exec cp {} "$s/runtimes/$rid/native/" \;
  done
}

stage_godot() {
  mkdir -p "$2/addons/lingara" && cp -R "$1/godot/addon/addons/lingara/bin" "$2/addons/lingara/" && cp "$repo/LICENSE" "$2/addons/lingara/LICENSE"
}

stage_unreal() {
  local r=$1 s=$2/LingaraEmbed rid platform
  mkdir -p "$s" && cp "$repo/LICENSE" "$s/LICENSE" && echo '{}' > "$s/LingaraEmbed.uplugin"
  for rid in $rids; do
    platform=$(bash "$here/release-toml.sh" package unreal "$rid" | sed 's#.*/lib/##')
    mkdir -p "$s/Source/ThirdParty/LingaraEmbedLibrary/lib/$platform"
    find "$r/dist/ffi/$rid" -type f ! -name '*.h' -exec cp {} "$s/Source/ThirdParty/LingaraEmbedLibrary/lib/$platform" \;
  done
}

stage_jar() {
  mkdir -p "$2/META-INF" && cp "$repo/LICENSE" "$2/META-INF/LICENSE"
  cp -R "$1/dist/jvm/win-x64/." "$1/dist/jvm/osx/." "$1/dist/jvm/linux-x64/." "$2/"
}

stage_aar() {
  mkdir -p "$2/jni" && cp "$repo/LICENSE" "$2/LICENSE" && cp -R "$1/dist/jvm/android-arm64/arm64-v8a" "$2/jni/"
}

expect_fail() {
  local what=$1 pattern=$2 out=$3; shift 3
  if "$@" > "$out" 2>&1; then cat "$out"; echo "FAIL: $what passed"; return 1; fi
  grep -q -- "$pattern" "$out" || { cat "$out"; echo "FAIL: $what failed without naming '$pattern'"; return 1; }
}

expect_pass() {
  local what=$1 out=$2; shift 2
  "$@" > "$out" 2>&1 || { cat "$out"; echo "FAIL: $what was refused"; return 1; }
}

# 1.10.26ag AC9
# A .nupkg missing one rid, a Godot zip whose library differs from its
# natives-<rid> bytes, an Unreal bundle holding Binaries/, and a package with
# no LICENSE each fail; the complete fixtures pass.
test_package_rejects_each_defect() {
  local tmp; tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' RETURN
  write_natives "$tmp/root"
  cd "$tmp/root"

  stage_nupkg "$tmp/root" "$tmp/nupkg"
  zipdir "$tmp/full.nupkg" "$tmp/nupkg"
  expect_pass "a complete .nupkg" "$tmp/out" bash "$check" package dotnet "$tmp/full.nupkg"
  rm -rf "$tmp/nupkg/runtimes/ios-arm64"
  zipdir "$tmp/short.nupkg" "$tmp/nupkg"
  expect_fail "a .nupkg missing ios-arm64" "nothing under runtimes/ios-arm64/native/" "$tmp/out" bash "$check" package dotnet "$tmp/short.nupkg"
  stage_nupkg "$tmp/root" "$tmp/nupkg-bare" && rm "$tmp/nupkg-bare/LICENSE"
  zipdir "$tmp/bare.nupkg" "$tmp/nupkg-bare"
  expect_fail "a .nupkg without LICENSE" "no LICENSE" "$tmp/out" bash "$check" package dotnet "$tmp/bare.nupkg"

  stage_godot "$tmp/root" "$tmp/godot"
  zipdir "$tmp/godot.zip" "$tmp/godot"
  expect_pass "a complete Godot zip" "$tmp/out" bash "$check" package godot "$tmp/godot.zip"
  echo "rebuilt" >> "$tmp/godot/addons/lingara/bin/linux-x64/liblingara_godot.so"
  zipdir "$tmp/godot-drift.zip" "$tmp/godot"
  expect_fail "a Godot zip with another linux-x64 library" "bin/linux-x64/liblingara_godot.so differs" "$tmp/out" bash "$check" package godot "$tmp/godot-drift.zip"

  stage_unreal "$tmp/root" "$tmp/ue"
  mkdir -p "$tmp/bundle"
  zipdir "$tmp/bundle/LingaraEmbed-9.9.9-UE5.6.zip" "$tmp/ue"
  zipdir "$tmp/bundle/LingaraEmbed-9.9.9-UE5.7.zip" "$tmp/ue"
  (cd "$tmp/bundle" && zip -q "$tmp/unreal.zip" ./*.zip)
  expect_pass "a complete Unreal bundle" "$tmp/out" bash "$check" package unreal "$tmp/unreal.zip"
  mkdir -p "$tmp/ue/LingaraEmbed/Binaries/Win64" && echo dll > "$tmp/ue/LingaraEmbed/Binaries/Win64/x.dll"
  zipdir "$tmp/bundle/LingaraEmbed-9.9.9-UE5.7.zip" "$tmp/ue"
  rm -f "$tmp/unreal.zip" && (cd "$tmp/bundle" && zip -q "$tmp/unreal.zip" ./*.zip)
  expect_fail "an Unreal bundle holding Binaries/" "build output inside" "$tmp/out" bash "$check" package unreal "$tmp/unreal.zip"

  stage_jar "$tmp/root" "$tmp/jar"
  zipdir "$tmp/natives-desktop.jar" "$tmp/jar"
  expect_pass "a complete natives-desktop jar" "$tmp/out" bash "$check" package nativesdesktop "$tmp/natives-desktop.jar"
  stage_aar "$tmp/root" "$tmp/aar"
  zipdir "$tmp/natives-android.aar" "$tmp/aar"
  expect_pass "a complete natives-android AAR" "$tmp/out" bash "$check" package nativesandroid "$tmp/natives-android.aar"
  cd "$repo"
}

# 1.10.26ag AC10
# A one-architecture universal ffi library, a JVM darwin-aarch64/ library
# built for x86_64, an Android library of the wrong ABI or linked at 4 KB, and
# an iOS simulator slice each fail; the right ones pass, the JVM's two thin
# libraries without lipo. release-toml.sh expands nativesdesktop's osx row to
# both darwin directories, and a release-ffi-zip holds dist/ffi/<rid>/'s
# libraries and both headers.
test_arch_rejects_the_wrong_slice() {
  local tmp; tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' RETURN
  write_tools "$tmp/tools"
  write_natives "$tmp/root"
  export FILE_CMD="$tmp/tools/file" LIPO="$tmp/tools/lipo" OBJDUMP="$tmp/tools/objdump"
  local rid
  for rid in $rids; do
    expect_pass "the right $rid libraries" "$tmp/out" bash "$check" arch "$tmp/root" "$rid"
  done
  if grep -q darwin- "$tmp/tools/lipo.calls"; then echo "FAIL: a thin JVM library was handed to lipo"; return 1; fi

  cp -R "$tmp/root" "$tmp/one-arch"
  lib "$tmp/one-arch/dist/ffi/osx/liblingara_embed.dylib" "Mach-O universal binary with 1 architecture: [arm64]" \
    "#lipo Architectures in the fat file: x are: arm64"
  expect_fail "a one-architecture universal library" "want exactly x86_64 arm64" "$tmp/out" bash "$check" arch "$tmp/one-arch" osx

  cp -R "$tmp/root" "$tmp/jvm"
  lib "$tmp/jvm/dist/jvm/osx/darwin-aarch64/liblingara_embed_uniffi.dylib" "Mach-O 64-bit dynamically linked shared library x86_64"
  expect_fail "an x86_64 library under darwin-aarch64/" "not a thin arm64 library" "$tmp/out" bash "$check" arch "$tmp/jvm" osx

  cp -R "$tmp/root" "$tmp/abi"
  lib "$tmp/abi/dist/ffi/android-arm64/liblingara_embed.so" "ELF 64-bit LSB shared object, x86-64, dynamically linked" \
    "#objdump     LOAD off    0x0 vaddr 0x0 paddr 0x0 align 2**14"
  expect_fail "an Android library of the wrong ABI" "not an aarch64 ELF" "$tmp/out" bash "$check" arch "$tmp/abi" android-arm64

  cp -R "$tmp/root" "$tmp/4k"
  lib "$tmp/4k/dist/jvm/android-arm64/arm64-v8a/liblingara_embed_uniffi.so" "ELF 64-bit LSB shared object, ARM aarch64" \
    "#objdump     LOAD off    0x0 vaddr 0x0 paddr 0x0 align 2**14" "#objdump     LOAD off    0x1 vaddr 0x1 paddr 0x1 align 2**12"
  expect_fail "an Android library linked at 4 KB" "aligned below 16 KB" "$tmp/out" bash "$check" arch "$tmp/4k" android-arm64

  cp -R "$tmp/root" "$tmp/sim"
  lib "$tmp/sim/dist/ffi/ios-arm64/liblingara_embed.a" "current ar archive" "#objdump   platform iossim"
  expect_fail "an iOS simulator slice" "not a device slice" "$tmp/out" bash "$check" arch "$tmp/sim" ios-arm64

  [ "$(bash "$here/release-toml.sh" package nativesdesktop osx | tr '\n' ' ')" = "darwin-aarch64/ darwin-x86-64/ " ] ||
    { echo "FAIL: nativesdesktop's osx row did not expand to both darwin directories"; return 1; }

  mkdir -p "$tmp/root/ffi/include" "$tmp/root/unreal/LingaraEmbed/Source/ThirdParty/LingaraEmbedLibrary/include"
  echo "/* c */" > "$tmp/root/ffi/include/lingara_embed.h"
  echo "// c++" > "$tmp/root/unreal/LingaraEmbed/Source/ThirdParty/LingaraEmbedLibrary/include/lingara_embed.hpp"
  make -s -C "$tmp/root" -f "$repo/make/release.mk" release-ffi-zip RID=osx TAG=v9.9.9 > "$tmp/out" 2>&1 || { cat "$tmp/out"; echo "FAIL: release-ffi-zip"; return 1; }
  [ "$(unzip -Z1 "$tmp/root/dist/release/lingara-embed-ffi-v9.9.9-osx.zip" | sort | tr '\n' ' ')" = "liblingara_embed.dylib lingara_embed.h lingara_embed.hpp " ] ||
    { unzip -Z1 "$tmp/root/dist/release/lingara-embed-ffi-v9.9.9-osx.zip"; echo "FAIL: the ffi zip's contents"; return 1; }
  unset FILE_CMD LIPO OBJDUMP
}

test_package_rejects_each_defect
echo "✓ check_package_test.sh: test_package_rejects_each_defect"
test_arch_rejects_the_wrong_slice
echo "✓ check_package_test.sh: test_arch_rejects_the_wrong_slice"
