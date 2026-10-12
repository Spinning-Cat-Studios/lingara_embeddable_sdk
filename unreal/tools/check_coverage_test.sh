#!/usr/bin/env bash
# check_coverage.sh passes on the committed decoder and encoder, and fails,
# naming the tag, on a schema holding a tag either side lacks (ADR 1.10.26ac
# AC10).
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../.." && pwd)
schema=$root/ffi/schema/messages.schema.json
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

# A copy of the schema with one more tag in one union.
with_tag() {
  jq --arg u "$1" --arg t "$2" '."$defs"[$u].oneOf += [{"properties": {"type": {"const": $t}}}]' "$schema" > "$work/$1.json"
  echo "$work/$1.json"
}

# Runs the check; it must fail and name the tag.
expect_fails_naming() {
  local fixture=$1 tag=$2 out
  if out=$(bash "$here/check_coverage.sh" "$fixture" 2>&1); then
    echo "✗ a schema with '$tag' passed"
    return 1
  fi
  case "$out" in *"'$tag'"*) ;; *) echo "✗ the failure does not name '$tag': $out"; return 1;; esac
}

# 1.10.26ac AC10
test_a_tag_missing_from_the_decoder_or_encoder_fails() {
  bash "$here/check_coverage.sh" > /dev/null
  expect_fails_naming "$(with_tag EmbedEvent weather_changed)" weather_changed
  expect_fails_naming "$(with_tag Command dialogue_jump)" dialogue_jump
}

test_a_tag_missing_from_the_decoder_or_encoder_fails
echo "✓ check_coverage_test: the committed sources pass; a missing event or command tag fails by name"
