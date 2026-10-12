#!/usr/bin/env bash
# `make godot-addon-zip` (ADR 1.10.26ab D6): godot/addon/dist/
# lingara-godot-addon-<semver>.zip holding addons/lingara/** and nothing
# else. It refuses while any K8 rid's bin/ directory is empty, so a local
# zip is never mistaken for a release one. K8's build-godot leg fills bin/
# from the five natives-<rid> artefacts first.
#
#   make-addon-zip.sh <godot/addon> "<rid> <rid> …"
set -euo pipefail

root=${1:?the addon root (godot/addon)}
rids=${2:?the rids}
addon="$root/addons/lingara"

missing=""
for rid in $rids; do
  if [ -z "$(ls -A "$addon/bin/$rid" 2>/dev/null)" ]; then missing="$missing $rid"; fi
done
if [ -n "$missing" ]; then
  echo "✗ no library for:$missing (K8 fills bin/ from natives-<rid>; a release zip carries all five)" >&2
  exit 1
fi
[ -f "$addon/LICENSE" ] || { echo "✗ $addon/LICENSE is missing" >&2; exit 1; }

version=$(sed -n 's/^version *= *"\(.*\)"$/\1/p' "$addon/plugin.cfg")
[ -n "$version" ] || { echo "✗ no version in $addon/plugin.cfg" >&2; exit 1; }
zip="dist/lingara-godot-addon-$version.zip"

mkdir -p "$root/dist"
rm -f "$root/$zip"
(cd "$root" && zip -qry "$zip" addons/lingara -x '*.DS_Store')
echo "✓ $root/$zip"
