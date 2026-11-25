#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
[[ "$(uname -s)" == Darwin ]] || { echo "Run this script on macOS" >&2; exit 1; }
case "${1:-}" in
  "") architectures=("$(uname -m)"); package_arch="${architectures[0]}" ;;
  --universal) architectures=(arm64 x86_64); package_arch=universal ;;
  *) echo "Usage: $0 [--universal]" >&2; exit 1 ;;
esac

export MACOSX_DEPLOYMENT_TARGET=11.0
target_dir="${CARGO_TARGET_DIR:-target}"
binaries=()
for arch in "${architectures[@]}"; do
  case "$arch" in
    arm64) target=aarch64-apple-darwin ;;
    x86_64) target=x86_64-apple-darwin ;;
    *) echo "Unsupported architecture: $arch" >&2; exit 1 ;;
  esac
  cargo build --release --locked --target "$target"
  binaries+=("$target_dir/$target/release/qbit-remote")
done

version=$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -1)
mkdir -p dist-macos
work=$(mktemp -d dist-macos/.build.XXXXXXXX)
output="dist-macos/qBit Remote.app"
cleanup() {
  if [[ -d "$work/previous.app" && ! -e "$output" ]]; then
    mv "$work/previous.app" "$output" || return
  fi
  rm -rf "$work"
}
trap cleanup EXIT
app="$work/qBit Remote.app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
if [[ "$package_arch" == universal ]]; then
  lipo -create "${binaries[@]}" -output "$app/Contents/MacOS/qbit-remote"
else
  cp "${binaries[0]}" "$app/Contents/MacOS/qbit-remote"
fi
for arch in "${architectures[@]}"; do
  lipo "$app/Contents/MacOS/qbit-remote" -verify_arch "$arch"
done
iconset="$work/AppIcon.iconset"
mkdir -p "$iconset"
for size in 16 32 128 256 512; do
  sips -z "$size" "$size" assets/logo-master.png --out "$iconset/icon_${size}x${size}.png" >/dev/null
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
<key>CFBundleDisplayName</key><string>qBit Remote</string>
<key>CFBundleExecutable</key><string>qbit-remote</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>$version</string>
<key>CFBundleVersion</key><string>$version</string>
<key>CFBundleIconFile</key><string>AppIcon</string>
<key>NSHighResolutionCapable</key><true/>
<key>LSMinimumSystemVersion</key><string>11.0</string>
<key>CFBundleDocumentTypes</key>
<array><dict>
  <key>CFBundleTypeName</key><string>BitTorrent Document</string>
  <key>CFBundleTypeRole</key><string>Viewer</string>
  <key>LSHandlerRank</key><string>Alternate</string>
  <key>LSItemContentTypes</key><array><string>org.bittorrent.torrent</string></array>
</dict></array>
<key>UTImportedTypeDeclarations</key>
<array><dict>
  <key>UTTypeIdentifier</key><string>org.bittorrent.torrent</string>
  <key>UTTypeDescription</key><string>BitTorrent Document</string>
  <key>UTTypeConformsTo</key><array><string>public.data</string></array>
  <key>UTTypeTagSpecification</key>
  <dict>
    <key>public.filename-extension</key><array><string>torrent</string></array>
    <key>public.mime-type</key><string>application/x-bittorrent</string>
  </dict>
</dict></array>
<key>CFBundleURLTypes</key>
<array><dict>
  <key>CFBundleURLName</key><string>org.qbitremote.magnet</string>
  <key>CFBundleTypeRole</key><string>Viewer</string>
  <key>CFBundleURLSchemes</key><array><string>magnet</string></array>
</dict></array>
</dict></plist>
EOF
plutil -lint "$app/Contents/Info.plist"
codesign --force --sign - "$app"
codesign --verify --deep --strict "$app"
archive="qbit-remote-$version-$package_arch.zip"
ditto -c -k --sequesterRsrc --keepParent "$app" "$work/$archive"
if [[ -d "$output" ]]; then
  mv "$output" "$work/previous.app"
fi
mv "$app" dist-macos/
mv "$work/$archive" dist-macos/
shasum -a 256 "dist-macos/$archive"
