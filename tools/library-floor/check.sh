#!/usr/bin/env bash
# One library floor for the whole trio (ADR 1.10.26y D1; roadmap D3).
#
# LIBRARY_FLOOR at the root holds the first `lingara` library release the
# kits depend on: the one carrying world.practice_completed and
# send_dialogue_turn. Each manifest that depends on a library spells it in
# its ecosystem's idiom; this reads each spelling back and fails when one
# declares another floor, or none. K1 writes the Rust row; the server kits
# (V1, ADR 1.10.26af D1) add the other six, in the app kits' spellings
# (RubyGems' `.pre.` form normalised).
#
# Usage: tools/library-floor/check.sh [root]   (root defaults to the repo)
# After moving the floor, `make lock-library-floor` refreshes the lockfiles
# that follow it (ADR 30.10.26a).
set -euo pipefail

root="${1:-$(cd "$(dirname "$0")/../.." && pwd)}"
floor_file="$root/LIBRARY_FLOOR"
[ -f "$floor_file" ] || { echo "✗ $floor_file is missing"; exit 1; }
floor="$(tr -d '[:space:]' < "$floor_file")"
semver='[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.]+)?'

# RubyGems writes 0.2.0-alpha.1 as 0.2.0.pre.alpha.1.
from_rubygems() { sed -E 's/^([0-9]+\.[0-9]+\.[0-9]+)\.pre\./\1-/'; }

# The first version string on the first line matching $2 in file $1.
first_version() {
  local file="$1" pattern="$2" re="${3:-$semver}"
  [ -f "$file" ] || { echo "MISSING"; return; }
  grep -E "$pattern" "$file" | head -n 1 | grep -oE "$re" | head -n 1 || true
}

# The same, inside the JSON object named $3 only (the npm peer, not its
# devDependencies twin).
object_version() {
  local file="$1" pattern="$2" object="$3"
  [ -f "$file" ] || { echo "MISSING"; return; }
  sed -n "/\"$object\"[[:space:]]*:/,/}/p" "$file" | grep -E "$pattern" | head -n 1 | grep -oE "$semver" | head -n 1 || true
}

failures=0
rows=0
check() {
  local label="$1" got="$2"
  rows=$((rows + 1))
  if [ "$got" = "MISSING" ]; then
    echo "✗ $label: manifest not found"; failures=$((failures + 1))
  elif [ -z "$got" ]; then
    echo "✗ $label: declares no lingara library floor"; failures=$((failures + 1))
  elif [ "$got" != "$floor" ]; then
    echo "✗ $label: floor $got, LIBRARY_FLOOR is $floor"; failures=$((failures + 1))
  else
    echo "✓ $label: $got"
  fi
}

check "core/Cargo.toml" "$(first_version "$root/core/Cargo.toml" '^lingara[[:space:]]*=')"
check "typescript/package.json" \
  "$(object_version "$root/typescript/package.json" '"@lingara/api"[[:space:]]*:[[:space:]]*"\^' peerDependencies)"
check "go/go.mod" \
  "$(first_version "$root/go/go.mod" 'github.com/Spinning-Cat-Studios/lingara_api_clients/go[[:space:]]+v')"
check "java-server/build.gradle.kts" \
  "$(first_version "$root/java-server/build.gradle.kts" '"com\.getlingara:lingara-java:')"
check "kotlin-server/build.gradle.kts" \
  "$(first_version "$root/kotlin-server/build.gradle.kts" '"com\.getlingara:lingara-kotlin:')"
ruby_raw="$(first_version "$root/ruby/lingara-embed.gemspec" 'add_dependency[[:space:]]*\(?"lingara"' '[0-9]+\.[0-9]+\.[0-9]+(\.pre\.[0-9A-Za-z.]+)?')"
check "ruby/lingara-embed.gemspec" "$( [ "$ruby_raw" = MISSING ] && echo MISSING || printf '%s' "$ruby_raw" | from_rubygems)"
check "php/composer.json" \
  "$(first_version "$root/php/composer.json" '"spinningcatstudios/lingara"[[:space:]]*:[[:space:]]*"\^')"

if [ "$failures" -ne 0 ]; then
  echo "✗ $failures manifest(s) disagree with LIBRARY_FLOOR ($floor)"
  exit 1
fi
echo "✓ $rows manifest(s), one library floor: $floor"
