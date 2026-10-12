#!/usr/bin/env bash
# The Apple wrapping `make godot-dist` uses (ADR 1.10.26ab D6, D7).
#
#   package-apple.sh osx <out-dir> <dylib>...   lipo the darwin builds into
#       liblingara_godot.framework (universal when given both arches), with
#       its Info.plist, ad-hoc signed; Godot's macOS export re-signs it.
#   package-apple.sh ios <out-dir> <dylib>      wrap the aarch64-apple-ios
#       build as lingara_godot.framework inside lingara_godot.xcframework.
set -euo pipefail

kind=${1:?osx or ios}
out=${2:?the output directory}
shift 2
[ "$#" -ge 1 ] || { echo "✗ no dylib given" >&2; exit 1; }

# plist <path> <executable> <bundle-id> <min-os-key> <min-os>
plist() {
  cat > "$1" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key><string>en</string>
  <key>CFBundleExecutable</key><string>$2</string>
  <key>CFBundleIdentifier</key><string>$3</string>
  <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
  <key>CFBundleName</key><string>$2</string>
  <key>CFBundlePackageType</key><string>FMWK</string>
  <key>CFBundleShortVersionString</key><string>1.0</string>
  <key>CFBundleVersion</key><string>1</string>
  <key>$4</key><string>$5</string>
</dict>
</plist>
EOF
}

case "$kind" in
  osx)
    fw="$out/liblingara_godot.framework"
    rm -rf "$fw" && mkdir -p "$fw/Resources"
    lipo -create "$@" -output "$fw/liblingara_godot"
    install_name_tool -id "@rpath/liblingara_godot.framework/liblingara_godot" "$fw/liblingara_godot"
    plist "$fw/Resources/Info.plist" liblingara_godot com.getlingara.godot LSMinimumSystemVersion 11.0
    codesign --force --sign - "$fw" >/dev/null 2>&1 || codesign --force --sign - "$fw/liblingara_godot"
    lipo -info "$fw/liblingara_godot"
    ;;
  ios)
    work=$(mktemp -d)
    trap 'rm -rf "$work"' EXIT
    fw="$work/lingara_godot.framework"
    mkdir -p "$fw"
    cp "$1" "$fw/lingara_godot"
    install_name_tool -id "@rpath/lingara_godot.framework/lingara_godot" "$fw/lingara_godot"
    plist "$fw/Info.plist" lingara_godot com.getlingara.godot MinimumOSVersion 13.0
    rm -rf "$out/lingara_godot.xcframework" && mkdir -p "$out"
    xcodebuild -create-xcframework -framework "$fw" -output "$out/lingara_godot.xcframework" >/dev/null
    lipo -info "$out/lingara_godot.xcframework/ios-arm64/lingara_godot.framework/lingara_godot"
    ;;
  *) echo "✗ the first argument is osx or ios" >&2; exit 1 ;;
esac
