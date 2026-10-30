#!/usr/bin/env bash
# The lockfile half of the library floor check (ADR 30.10.26a D4).
#
# check.sh reads LIBRARY_FLOOR back from the seven manifests; this reads it
# back from the Go and PHP lockfiles, the two ecosystems whose locks no other
# lane brings up to the manifest. Pure bash: no network, no toolchain, so it
# runs wherever check.sh does. `make lock-library-floor` is what makes it pass
# after a floor move.
#
# Usage: tools/library-floor/locks.sh [root]   (root defaults to the repo)
set -euo pipefail

root="${1:-$(cd "$(dirname "$0")/../.." && pwd)}"
floor_file="$root/LIBRARY_FLOOR"
[ -f "$floor_file" ] || { echo "✗ $floor_file is missing"; exit 1; }
floor="$(tr -d '[:space:]' < "$floor_file")"
semver='[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.]+)?'
go_lib='github.com/Spinning-Cat-Studios/lingara_api_clients/go'

failures=0
rows=0
fail() { echo "✗ $1"; failures=$((failures + 1)); }

# A nested module's require of the library, single-line or `require (` block
# form alike: both put the path and its version on one line.
check_go_mod() {
  local rel="$1" file="$root/$1" got
  rows=$((rows + 1))
  [ -f "$file" ] || { fail "$rel: lockfile not found"; return; }
  got="$(grep -E "^[[:space:]]*(require[[:space:]]+)?$go_lib[[:space:]]+v" "$file" | head -n 1 | grep -oE "v$semver" | head -n 1 || true)"
  if [ -z "$got" ]; then fail "$rel: requires no lingara library"
  elif [ "$got" != "v$floor" ]; then fail "$rel: requires ${got#v}, LIBRARY_FLOOR is $floor"
  else echo "✓ $rel: ${got#v}"; fi
}

# A go.sum holds the floor's module hash. Older versions' lines are not
# checked: lock-go's tidy drops them.
check_go_sum() {
  local rel="$1" file="$root/$1"
  rows=$((rows + 1))
  [ -f "$file" ] || { fail "$rel: lockfile not found"; return; }
  if grep -qF "$go_lib v$floor h1:" "$file"; then echo "✓ $rel: $floor"
  else fail "$rel: no $go_lib v$floor h1: line, LIBRARY_FLOOR is $floor"; fi
}

# composer.lock's spinningcatstudios/lingara version: the first "version"
# after that package's "name", leading v dropped.
check_composer_lock() {
  local rel="$1" file="$root/$1" got
  rows=$((rows + 1))
  [ -f "$file" ] || { fail "$rel: lockfile not found"; return; }
  got="$(sed -n '/"name":[[:space:]]*"spinningcatstudios\/lingara"/,/"version"/p' "$file" \
    | grep -E '"version"' | head -n 1 | grep -oE "$semver" | head -n 1 || true)"
  if [ -z "$got" ]; then fail "$rel: locks no lingara library"
  elif [ "$got" != "$floor" ]; then fail "$rel: locks $got, LIBRARY_FLOOR is $floor"
  else echo "✓ $rel: $got"; fi
}

check_go_mod go/conformance-es/go.mod
check_go_mod snippets/server/go/go.mod
check_go_sum go/go.sum
check_go_sum go/conformance-es/go.sum
check_go_sum snippets/server/go/go.sum
check_composer_lock php/composer.lock

if [ "$failures" -ne 0 ]; then
  echo "✗ $failures lockfile(s) disagree with LIBRARY_FLOOR ($floor): run make lock-library-floor"
  exit 1
fi
echo "✓ $rows lockfile(s), one library floor: $floor"
