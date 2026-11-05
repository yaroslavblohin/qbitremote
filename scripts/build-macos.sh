#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
[[ "$(uname -s)" == Darwin ]] || { echo "Run this script on macOS" >&2; exit 1; }
export MACOSX_DEPLOYMENT_TARGET=11.0
cargo build --release --locked
version=$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -1)
app="dist-macos/qBit Remote.app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp target/release/qbit-remote "$app/Contents/MacOS/qbit-remote"
chmod +x "$app/Contents/MacOS/qbit-remote"
iconset=$(mktemp -d)/AppIcon.iconset
mkdir -p "$iconset"
for size in 16 32 128 256 512; do
  sips -z "$size" "$size" assets/logo.png --out "$iconset/icon_${size}x${size}.png" >/dev/null
  double=$((size * 2))
  sips -z "$double" "$double" assets/logo-master.png --out "$iconset/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$iconset" -o "$app/Contents/Resources/AppIcon.icns"
cat > "$app/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleIdentifier</key><string>org.qbitremote.desktop</string>
<key>CFBundleName</key><string>qBit Remote</string>
<key>CFBundleExecutable</key><string>qbit-remote</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>$version</string>
<key>CFBundleVersion</key><string>$version</string>
<key>CFBundleIconFile</key><string>AppIcon</string>
<key>NSHighResolutionCapable</key><true/>
<key>LSMinimumSystemVersion</key><string>11.0</string>
</dict></plist>
EOF
plutil -lint "$app/Contents/Info.plist"
codesign --force --deep --sign - "$app"
codesign --verify --deep --strict "$app"
ditto -c -k --sequesterRsrc --keepParent "$app" "dist-macos/qbit-remote-$version-$(uname -m).zip"
