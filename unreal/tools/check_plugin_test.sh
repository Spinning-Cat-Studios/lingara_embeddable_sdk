#!/usr/bin/env bash
# check_plugin.sh passes on the committed plugin, the snippet and a packaged
# zip, and fails on each shape AC11 names (ADR 1.10.26ac AC11).
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
marker="lingara"":begin"

# A fresh copy of the plugin and the snippet, outside any git tree.
fixture() {
  rm -rf "$work/plugin" "$work/snippet"
  cp -R "$root/unreal/LingaraEmbed" "$work/plugin"
  find "$work/plugin/Source/ThirdParty/LingaraEmbedLibrary/lib" -type f ! -name .gitignore -delete
  cp -R "$root/snippets/player/unreal" "$work/snippet"
}

expect_fails() {
  local why=$1
  shift
  if PLUGIN_DIR=$work/plugin SNIPPET_DIR=$work/snippet "$@" > "$work/out" 2>&1; then
    echo "✗ passed: $why"
    return 1
  fi
  # A check's own refusal, not a crash on the way to one.
  grep -q '^✗ ' "$work/out" || { echo "✗ failed without a finding: $why"; cat "$work/out"; return 1; }
}

settings_with() {
  fixture
  local header=$work/plugin/Source/LingaraEmbed/Public/LingaraEmbedSettings.h
  awk -v line="	UPROPERTY(config, EditAnywhere, Category = \"Lingara\") FString $1;" \
    '{ print } /int32 MaxEventsPerTick = 256;/ { print line }' "$header" > "$header.new" && mv "$header.new" "$header"
  expect_fails "a settings property named $1" bash "$here/check_plugin.sh"
}

descriptor_with() {
  fixture
  jq "$1" "$root/unreal/LingaraEmbed/LingaraEmbed.uplugin" > "$work/plugin/LingaraEmbed.uplugin"
  expect_fails "a descriptor with $1" bash "$here/check_plugin.sh"
}

# A good zip from package.sh over stand-in libraries.
good_zip() {
  mkdir -p "$work/dist"/{win-x64,osx,linux-x64,android-arm64,ios-arm64}
  for f in win-x64/lingara_embed.dll win-x64/lingara_embed.dll.lib osx/liblingara_embed.dylib \
           linux-x64/liblingara_embed.so android-arm64/liblingara_embed.so ios-arm64/liblingara_embed.a; do
    echo "stand-in" > "$work/dist/$f"
  done
  echo "1.0.0" > "$work/VERSION"
  VERSION_FILE=$work/VERSION bash "$here/package.sh" package "$work/dist" "$work/zips" "$root/unreal/engine-versions.txt" 5.6 > /dev/null
  echo "$work/zips/LingaraEmbed-1.0.0-UE5.6.zip"
}

# The good zip, unpacked, changed by $1, and packed again.
zip_with() {
  local change=$1 tree=$work/tree
  rm -rf "$tree" && mkdir -p "$tree"
  (cd "$tree" && cmake -E tar xf "$2")
  (cd "$tree/LingaraEmbed" && eval "$change")
  (cd "$tree" && cmake -E tar cf "$work/bad.zip" --format=zip LingaraEmbed)
  expect_fails "a zip after: $change" bash "$here/check_plugin.sh" --zip "$work/bad.zip"
}

# 1.10.26ac AC11
test_descriptor_settings_snippet_and_package_shape() {
  bash "$here/check_plugin.sh" > /dev/null
  descriptor_with '.Modules[0].PlatformAllowList += ["TVOS"]'
  descriptor_with '.Modules[0].PlatformAllowList -= ["IOS"]'
  for name in ClientSecret ClientId client_id Password AccessToken; do settings_with "$name"; done

  fixture
  echo "a library" > "$work/plugin/Source/ThirdParty/LingaraEmbedLibrary/lib/lingara_embed.dll"
  expect_fails "a file under lib/" bash "$here/check_plugin.sh"

  fixture
  sed -i.bak "/$marker practice/d" "$work/snippet/LingaraEmbedSnippet.cpp"
  expect_fails "a snippet without its practice region" bash "$here/check_plugin.sh"
  fixture
  echo "unzip" > "$work/snippet/install.sh"
  expect_fails "a snippet with an install file" bash "$here/check_plugin.sh"

  local zip
  zip=$(good_zip)
  bash "$here/check_plugin.sh" --zip "$zip" > /dev/null
  zip_with 'mkdir -p Binaries/Win64 && echo x > Binaries/Win64/x.dll' "$zip"
  zip_with 'mkdir -p Intermediate && echo x > Intermediate/x' "$zip"
  zip_with 'rm LICENSE' "$zip"
  zip_with 'rm Source/ThirdParty/LingaraEmbedLibrary/lib/IOS/liblingara_embed.a' "$zip"
  zip_with 'jq "del(.EngineVersion)" LingaraEmbed.uplugin > d && mv d LingaraEmbed.uplugin' "$zip"
}

test_descriptor_settings_snippet_and_package_shape
echo "✓ check_plugin_test: the plugin, snippet and a zip pass; each bad shape fails"
