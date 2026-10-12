#!/usr/bin/env bash
# Every K1 tag has a literal on the engine side (ADR 1.10.26ac D10, AC10):
# each EmbedEvent `type` in ffi/schema/messages.schema.json appears as a
# string literal in LingaraEventDecoder.cpp, and each Command `type` in
# LingaraCommandEncoder.cpp. It proves a mention, not a correct decode, but it
# moves "K1 added a type" from the next engine smoke to the PR that bumps the
# core.
#
#   check_coverage.sh [schema] [decoder.cpp] [encoder.cpp]
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
private=$root/unreal/LingaraEmbed/Source/LingaraEmbed/Private
schema=${1:-$root/ffi/schema/messages.schema.json}
decoder=${2:-$private/LingaraEventDecoder.cpp}
encoder=${3:-$private/LingaraCommandEncoder.cpp}
missing=0

# One union's tags against one file's literals.
check() {
  local kind=$1 union=$2 file=$3 tags
  # tr: jq on Windows ends its lines with CRLF.
  tags=$(jq -r --arg u "$union" '."$defs"[$u].oneOf[].properties.type.const' "$schema" | tr -d '\r')
  if [ -z "$tags" ]; then
    echo "✗ $schema lists no $union tags"
    missing=1
    return
  fi
  for tag in $tags; do
    if ! grep -qF "\"$tag\"" "$file"; then
      echo "✗ the $kind tag '$tag' has no literal in $(basename "$file")"
      missing=1
    fi
  done
}

check event EmbedEvent "$decoder"
check command Command "$encoder"
if [ "$missing" -eq 0 ]; then
  echo "✓ every event tag is in the decoder, every command tag in the encoder"
fi
exit "$missing"
