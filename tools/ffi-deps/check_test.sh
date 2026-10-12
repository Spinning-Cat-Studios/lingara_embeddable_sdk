#!/usr/bin/env bash
# check.sh's own tests (ADR 1.10.26aa AC28): fixture import lists stand in for
# each platform's tool, so the verdicts are pinned on any host; then every
# artefact this host has built under dist/ffi/ is checked for real.
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../.." && pwd)"
scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT
failures=0

# expect <pass|fail> <rid> <fixture lines...>
expect() {
  local want="$1" rid="$2"
  shift 2
  printf '%s\n' "$@" > "$scratch/imports"
  if FFI_DEPS_IMPORTS="$scratch/imports" bash "$here/check.sh" "$rid" "$scratch" > "$scratch/out" 2>&1; then got=pass; else got=fail; fi
  if [ "$got" != "$want" ]; then
    echo "✗ $rid [$*]: expected $want, got $got"
    cat "$scratch/out"
    failures=$((failures + 1))
  fi
}

# 1.10.26aa AC28
test_each_artefact_imports_only_allowlisted_libraries() {
  expect pass win-x64 "NEEDED KERNEL32.dll" "NEEDED ntdll.dll" "NEEDED WS2_32.dll"
  expect fail win-x64 "NEEDED KERNEL32.dll" "NEEDED VCRUNTIME140.dll"
  expect pass osx "NEEDED @rpath/liblingara_embed.dylib" "NEEDED /usr/lib/libSystem.B.dylib" "NEEDED /usr/lib/libiconv.2.dylib"
  expect fail osx "NEEDED /usr/lib/libSystem.B.dylib" "NEEDED /opt/homebrew/opt/openssl@3/lib/libssl.3.dylib"
  expect pass linux-x64 "NEEDED libc.so.6" "NEEDED libgcc_s.so.1" "GLIBC 2.17" "GLIBC 2.28"
  expect fail linux-x64 "NEEDED libc.so.6" "NEEDED libssl.so.3"
  expect fail linux-x64 "NEEDED libc.so.6" "GLIBC 2.29"
  expect fail linux-x64 "NEEDED libc.so.6" "GLIBC 2.34"
  expect pass android-arm64 "NEEDED libc.so" "NEEDED libdl.so" "NEEDED liblog.so" "GLIBC 2.99"
  expect fail android-arm64 "NEEDED libc.so" "NEEDED libc++_shared.so"
  expect pass ios-arm64 ""
  expect fail ios-arm64 "NEEDED /usr/lib/libSystem.B.dylib"

  # The real artefacts this host has built.
  local rid
  for rid in win-x64 osx linux-x64 android-arm64 ios-arm64; do
    if [ -d "$root/dist/ffi/$rid" ]; then
      bash "$here/check.sh" "$rid" "$root/dist/ffi/$rid" || failures=$((failures + 1))
    fi
  done
}

test_each_artefact_imports_only_allowlisted_libraries
if [ "$failures" -gt 0 ]; then
  echo "✗ ffi-deps: $failures check(s) failed"
  exit 1
fi
echo "✓ ffi-deps: check.sh verdicts hold on every fixture"
