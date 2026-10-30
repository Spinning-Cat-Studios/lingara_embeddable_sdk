#!/usr/bin/env bash
# The lockfile floor check over a scratch tree (ADR 30.10.26a D4).
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
locks="$here/locks.sh"
quality_mk="$here/../../make/quality.mk"
go_lib='github.com/Spinning-Cat-Studios/lingara_api_clients/go'

# The six Go and PHP lockfiles under $1, each naming $3; LIBRARY_FLOOR is $2.
# One nested go.mod uses the `require (` block, the other the single line.
write_tree() {
  local root="$1" floor="$2" f="$3"
  mkdir -p "$root/go/conformance-es" "$root/snippets/server/go" "$root/php"
  printf '%s\n' "$floor" > "$root/LIBRARY_FLOOR"
  printf 'module a\n\ngo 1.23\n\nrequire (\n\t%s v%s\n\tgithub.com/x/kit v0.0.0\n)\n' "$go_lib" "$f" > "$root/go/conformance-es/go.mod"
  printf 'module b\n\ngo 1.23\n\nrequire %s v%s\n' "$go_lib" "$f" > "$root/snippets/server/go/go.mod"
  local sum
  for sum in go/go.sum go/conformance-es/go.sum snippets/server/go/go.sum; do
    printf '%s v%s h1:abc=\n%s v%s/go.mod h1:def=\n' "$go_lib" "$f" "$go_lib" "$f" > "$root/$sum"
  done
  printf '{\n    "packages": [\n        {\n            "name": "psr/http-message",\n            "version": "2.0"\n        },\n        {\n            "name": "spinningcatstudios/lingara",\n            "version": "v%s"\n        }\n    ]\n}\n' "$f" > "$root/php/composer.lock"
}

# 30.10.26a AC1
# Every Go and PHP lockfile naming the floor passes, one row each.
test_locks_at_the_floor_pass() {
  local tmp; tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' RETURN
  write_tree "$tmp/same" 0.2.0-alpha.6 0.2.0-alpha.6
  bash "$locks" "$tmp/same" > "$tmp/same.out" || { cat "$tmp/same.out"; echo "FAIL: locks at the floor were refused"; return 1; }
  [ "$(grep -cE '^✓ [a-z/-]+/go\.(mod|sum)|^✓ php/composer\.lock' "$tmp/same.out")" = 6 ] || { cat "$tmp/same.out"; echo "FAIL: not six rows"; return 1; }
}

# 30.10.26a AC2
# A nested go.mod require, a go.sum or composer.lock on another version
# fails naming its row; so does a missing lockfile.
test_a_stale_lock_fails_naming_its_row() {
  local tmp; tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' RETURN
  local file
  for file in go/conformance-es/go.mod snippets/server/go/go.mod go/go.sum go/conformance-es/go.sum snippets/server/go/go.sum php/composer.lock; do
    write_tree "$tmp/drift" 0.2.0-alpha.6 0.2.0-alpha.6
    sed -i.bak 's/0\.2\.0-alpha\.6/0.2.0-alpha.1/' "$tmp/drift/$file"
    if bash "$locks" "$tmp/drift" > "$tmp/drift.out"; then echo "FAIL: a stale $file passed"; return 1; fi
    grep -q "^✗ $file: " "$tmp/drift.out" || { cat "$tmp/drift.out"; echo "FAIL: $file not named"; return 1; }
    [ "$(grep -c '^✗ [a-z]' "$tmp/drift.out")" = 1 ] || { cat "$tmp/drift.out"; echo "FAIL: $file is not the one failing row"; return 1; }
    rm -rf "$tmp/drift"

    write_tree "$tmp/gone" 0.2.0-alpha.6 0.2.0-alpha.6
    rm "$tmp/gone/$file"
    if bash "$locks" "$tmp/gone" > "$tmp/gone.out"; then echo "FAIL: a missing $file passed"; return 1; fi
    grep -q "^✗ $file: lockfile not found" "$tmp/gone.out" || { cat "$tmp/gone.out"; echo "FAIL: missing $file not named"; return 1; }
    rm -rf "$tmp/gone"
  done
}

# 30.10.26a AC3
# check-library-floor (and so check-publishable) runs the lock half.
test_check_library_floor_runs_the_lock_half() {
  local recipe
  recipe="$(sed -n '/^check-library-floor:/,/^$/p' "$quality_mk")"
  printf '%s\n' "$recipe" | grep -q 'tools/library-floor/locks_test.sh' || { echo "FAIL: check-library-floor does not run locks_test.sh"; return 1; }
  printf '%s\n' "$recipe" | grep -q 'tools/library-floor/locks.sh' || { echo "FAIL: check-library-floor does not run locks.sh"; return 1; }
  grep -qE '^check-publishable:.*check-library-floor' "$quality_mk" || { echo "FAIL: check-publishable does not run check-library-floor"; return 1; }
}

test_locks_at_the_floor_pass
echo "✓ locks_test.sh: test_locks_at_the_floor_pass"
test_a_stale_lock_fails_naming_its_row
echo "✓ locks_test.sh: test_a_stale_lock_fails_naming_its_row"
test_check_library_floor_runs_the_lock_half
echo "✓ locks_test.sh: test_check_library_floor_runs_the_lock_half"
