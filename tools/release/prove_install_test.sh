#!/usr/bin/env bash
# prove-install.sh's refusals and its deadline path, offline (ADR 1.10.26ag D7).
#
# release-manifest is a stand-in on PATH that prints the core's install line,
# so the test needs no installed tool and no registry: Cargo runs offline, so
# every attempt fails fast and the deadline path is what is measured.
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
prove="$here/prove-install.sh"

# 1.10.26ag AC11
# godot, unreal and an unknown id are refused with exit 2 before any attempt;
# offline, `core v0.0.0-nonexistent.1 --deadline 5s` exits 1 with
# `install proof failed: core …` within ten seconds.
test_store_and_unknown_ids_are_refused() {
  local tmp; tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' RETURN
  mkdir -p "$tmp/bin"
  printf '#!/bin/sh\n[ "$1" = install-line ] || exit 9\necho "cargo add lingara-embed@${3#v} --features server"\n' > "$tmp/bin/release-manifest"
  chmod +x "$tmp/bin/release-manifest"

  local id code
  for id in godot unreal cobol; do
    code=0
    PATH="$tmp/bin:$PATH" sh "$prove" "$id" v0.0.0 > "$tmp/out" 2>&1 || code=$?
    [ "$code" = 2 ] || { cat "$tmp/out"; echo "FAIL: $id exited $code, not 2"; return 1; }
  done
  grep -q "no proof for cobol" "$tmp/out" || { cat "$tmp/out"; echo "FAIL: the unknown id is not named"; return 1; }
  code=0
  sh "$prove" > "$tmp/out" 2>&1 || code=$?
  [ "$code" = 2 ] || { echo "FAIL: no arguments exited $code, not 2"; return 1; }

  local start elapsed
  start=$(date +%s)
  code=0
  PATH="$tmp/bin:$PATH" CARGO_NET_OFFLINE=true TMPDIR="$tmp" \
    sh "$prove" core v0.0.0-nonexistent.1 --deadline 5s > "$tmp/out" 2>&1 || code=$?
  elapsed=$(( $(date +%s) - start ))
  [ "$code" = 1 ] || { cat "$tmp/out"; echo "FAIL: an unreachable core exited $code, not 1"; return 1; }
  grep -q "^install proof failed: core cargo add lingara-embed@0.0.0-nonexistent.1" "$tmp/out" ||
    { cat "$tmp/out"; echo "FAIL: the failure line"; return 1; }
  [ "$elapsed" -le 10 ] || { echo "FAIL: the 5s deadline took ${elapsed}s"; return 1; }
}

test_store_and_unknown_ids_are_refused
echo "✓ prove_install_test.sh: test_store_and_unknown_ids_are_refused"
