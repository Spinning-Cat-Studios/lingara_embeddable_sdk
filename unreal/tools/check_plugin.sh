#!/usr/bin/env bash
# The plugin's shape, checked without the engine (ADR 1.10.26ac D7, D9,
# AC11).
#
#   check_plugin.sh             the committed descriptor, the settings
#                               header, lib/'s tracked files and the snippet
#   check_plugin.sh --zip Z...  each packaged zip: one LingaraEmbed/ root,
#                               no Binaries/ or Intermediate/, LICENSE, all
#                               five platforms' libraries, a stamped descriptor
#
# PLUGIN_DIR and SNIPPET_DIR point it at a fixture (check_plugin_test.sh).
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
plugin=${PLUGIN_DIR:-$root/unreal/LingaraEmbed}
snippet=${SNIPPET_DIR:-$root/snippets/player/unreal}
platforms="Android IOS Linux Mac Win64"
libs="Win64/lingara_embed.dll Win64/lingara_embed.dll.lib Mac/liblingara_embed.dylib Linux/liblingara_embed.so
      Android/arm64-v8a/liblingara_embed.so IOS/liblingara_embed.a"
lib_prefix=LingaraEmbed/Source/ThirdParty/LingaraEmbedLibrary/lib
problems=0

fail() {
  echo "✗ $*"
  problems=1
}

# jq on Windows ends its lines with CRLF.
field() { jq -r "$1" "$2" | tr -d '\r'; }

check_descriptor() {
  local file=$1 got
  got=$(field '[.Modules[].PlatformAllowList[]] | sort | join(" ")' "$file")
  [ "$got" = "$platforms" ] || fail "PlatformAllowList is '$got', not '$platforms'"
  [ "$(field '.CanContainContent' "$file")" = true ] || fail "CanContainContent is not true"
}

# Every property in the settings header, by name: nothing in DefaultGame.ini
# may look like a credential (D7).
check_settings() {
  local header=$plugin/Source/LingaraEmbed/Public/LingaraEmbedSettings.h name
  for name in $(sed -E 's/^[[:space:]]*UPROPERTY\(.*\)[[:space:]]*//' "$header" \
      | grep -E '^[[:space:]]*[A-Za-z_][A-Za-z0-9_:<>, ]*[[:space:]]+[A-Za-z_][A-Za-z0-9_]*[[:space:]]*(=[^;]*)?;' \
      | grep -v '(' | sed -E 's/[[:space:]]*(=[^;]*)?;.*$//' | awk '{print $NF}'); do
    if echo "$name" | grep -qiE 'secret|client_?id|password|token'; then
      fail "the settings property '$name' is named like a credential: a game ships its .ini"
    fi
  done
}

# lib/ is staged, never committed: only its .gitignore is tracked (D3).
check_lib() {
  local lib=$plugin/Source/ThirdParty/LingaraEmbedLibrary/lib files f
  if git -C "$lib" rev-parse --is-inside-work-tree > /dev/null 2>&1; then
    files=$(git -C "$lib" ls-files .)
  else
    files=$(cd "$lib" && find . -type f | sed 's|^\./||')
  fi
  for f in $files; do
    [ "$f" = .gitignore ] || fail "lib/ holds $f: staged libraries are never committed"
  done
}

# P1's four player regions, and no install file (D9). The marker is built by
# concatenation so this script is never itself a marked file.
check_snippet() {
  local marker="lingara"":begin" cpp=$snippet/LingaraEmbedSnippet.cpp keys
  keys=$(grep -E "^[[:space:]]*// $marker " "$cpp" | sed -E "s|.*$marker ||" | tr -d '\r' | LC_ALL=C sort -u | tr '\n' ' ')
  [ "$keys" = "dialogue poll practice tokenSource " ] || fail "the snippet's regions are '$keys', not P1's four"
  if grep -q "$marker" "$snippet/LingaraEmbedSnippetActor.h"; then
    fail "LingaraEmbedSnippetActor.h carries a region: one marked file per surface"
  fi
  if compgen -G "$snippet/install.*" > /dev/null; then
    fail "the snippet carries an install file: the plugin installs by hand (P1's prose)"
  fi
}

# The README's "Supported" line names engine-versions.txt's set (D1).
check_supported_line() {
  local versions=$root/unreal/engine-versions.txt line
  line=$(grep -E '^5\.[0-9]+$' "$versions" | tr -d '\r' | awk '{ v[NR] = $0 } END {
    for (i = 1; i <= NR; i++) printf "%s%s", v[i], (i == NR ? "" : (i == NR - 1 ? " and " : ", ")) }')
  grep -qF "Unreal Engine $line (" "$plugin/README.md" \
    || fail "the plugin README's Supported line does not name Unreal Engine $line, as engine-versions.txt does"
}

check_zip() {
  local zip list work engine lib
  zip=$(cd "$(dirname "$1")" && pwd)/$(basename "$1")
  list=$(cmake -E tar tf "$zip" | tr -d '\r')
  echo "$list" | grep -vq '^LingaraEmbed/' && fail "$zip: an entry outside LingaraEmbed/"
  echo "$list" | grep -qE '(^|/)(Binaries|Intermediate)/' && fail "$zip: holds Binaries/ or Intermediate/"
  echo "$list" | grep -qx 'LingaraEmbed/LICENSE' || fail "$zip: no LICENSE"
  for lib in $libs; do
    echo "$list" | grep -qx "$lib_prefix/$lib" || fail "$zip: no $lib"
  done
  work=$(mktemp -d)
  (cd "$work" && cmake -E tar xf "$zip")
  check_descriptor "$work/LingaraEmbed/LingaraEmbed.uplugin"
  engine=$(field '.EngineVersion // ""' "$work/LingaraEmbed/LingaraEmbed.uplugin")
  [[ "$engine" =~ ^5\.[0-9]+\.0$ ]] || fail "$zip: EngineVersion is unstamped ('$engine')"
  [ "$(field '.VersionName' "$work/LingaraEmbed/LingaraEmbed.uplugin")" != 0.0.0-dev ] || fail "$zip: VersionName is unstamped"
  rm -rf "$work"
}

if [ "${1:-}" = --zip ]; then
  shift
  for zip in "$@"; do check_zip "$zip"; done
else
  check_descriptor "$plugin/LingaraEmbed.uplugin"
  [ -z "$(field '.EngineVersion // ""' "$plugin/LingaraEmbed.uplugin")" ] \
    || fail "the committed descriptor names an EngineVersion: only package.sh stamps one"
  check_settings
  check_lib
  check_snippet
  check_supported_line
fi
[ "$problems" -eq 0 ] && echo "✓ check_plugin: ${1:-the committed plugin and snippet} hold their shape"
exit "$problems"
