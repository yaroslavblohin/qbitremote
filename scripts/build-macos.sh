#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
[[ "$(uname -s)" == Darwin ]] || { echo "Run this script on macOS" >&2; exit 1; }
usage() {
  echo "Usage: $0 [--amd64 | --arm64 | --universal]" >&2
  exit 1
}
[[ $# -le 1 ]] || usage
case "${1:-$(uname -m)}" in
  --arm64|arm64) architectures=(arm64); package_arch=arm64 ;;
  --amd64|x86_64) architectures=(x86_64); package_arch=amd64 ;;
  --universal) architectures=(arm64 x86_64); package_arch=universal ;;
  *) usage ;;
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
output_dir=dist-macos
if [[ "$package_arch" != universal ]]; then
  output_dir="$output_dir/$package_arch"
fi
output="$output_dir/qBit Remote.app"
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
  [[ "$(lipo -archs "$app/Contents/MacOS/qbit-remote")" == "${architectures[0]}" ]]
fi
for arch in "${architectures[@]}"; do
  lipo "$app/Contents/MacOS/qbit-remote" -verify_arch "$arch"
done
make_icon() {
  local source="$1" destination="$2" size double
  local iconset="$work/$(basename "$source" .png).iconset"
  mkdir -p "$iconset"
  for size in 16 32 128 256 512; do
    sips -z "$size" "$size" "$source" --out "$iconset/icon_${size}x${size}.png" >/dev/null
    double=$((size * 2))
    sips -z "$double" "$double" "$source" --out "$iconset/icon_${size}x${size}@2x.png" >/dev/null
  done
  iconutil -c icns "$iconset" -o "$destination"
}
make_icon assets/logo-master.png "$app/Contents/Resources/AppIcon.icns"
make_icon assets/file-icons/magnet.png "$app/Contents/Resources/FileIcon.icns"

document_types() {
  cat <<'EOF'
<key>CFBundleDocumentTypes</key>
<array><dict>
  <key>CFBundleTypeName</key><string>BitTorrent Document</string>
  <key>CFBundleTypeRole</key><string>Viewer</string>
  <key>CFBundleTypeIconFile</key><string>FileIcon.icns</string>
  <key>LSHandlerRank</key><string>Alternate</string>
  <key>LSItemContentTypes</key><array><string>org.bittorrent.torrent</string></array>
</dict><dict>
  <key>CFBundleTypeName</key><string>Magnet Shortcut</string>
  <key>CFBundleTypeRole</key><string>Viewer</string>
  <key>CFBundleTypeIconFile</key><string>FileIcon.icns</string>
  <key>LSHandlerRank</key><string>Alternate</string>
  <key>LSItemContentTypes</key><array><string>org.qbitremote.magnet-file</string></array>
</dict></array>
EOF
}

bundle_header() {
  local identifier="$1" executable="$2" icon="$3"
  cat <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleIdentifier</key><string>$identifier</string>
<key>CFBundleName</key><string>qBit Remote</string>
<key>CFBundleDisplayName</key><string>qBit Remote</string>
<key>CFBundleExecutable</key><string>$executable</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>$version</string>
<key>CFBundleVersion</key><string>$version</string>
<key>CFBundleIconFile</key><string>$icon</string>
<key>LSMinimumSystemVersion</key><string>11.0</string>
EOF
}

{
  bundle_header org.qbitremote.desktop qbit-remote AppIcon
  document_types
  cat <<'EOF'
<key>NSHighResolutionCapable</key><true/>
<key>NSLocalNetworkUsageDescription</key>
<string>qBit Remote подключается к Web UI qBittorrent в вашей локальной сети.</string>
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
<key>UTExportedTypeDeclarations</key>
<array><dict>
  <key>UTTypeIdentifier</key><string>org.qbitremote.magnet-file</string>
  <key>UTTypeDescription</key><string>Magnet Shortcut</string>
  <key>UTTypeConformsTo</key><array><string>public.data</string><string>public.content</string></array>
  <key>UTTypeTagSpecification</key>
  <dict>
    <key>public.filename-extension</key><array><string>magnet</string></array>
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
} > "$app/Contents/Info.plist"

# Keep every variant sealed and use the same resource IDs as the icon picker.
arch_flags=()
for arch in "${architectures[@]}"; do arch_flags+=(-arch "$arch"); done
executables=("$app/Contents/MacOS/qbit-remote")
resource=3
for icon in document blue-arrow green-arrow green-tile blue-circle; do
  helper="$app/Contents/Helpers/qBit Remote $resource.app"
  mkdir -p "$helper/Contents/MacOS" "$helper/Contents/Resources"
  executable="$helper/Contents/MacOS/qbit-file-handler"
  xcrun clang "${arch_flags[@]}" -fobjc-arc -Wall -Wextra -framework AppKit \
    -DQBIT_FILE_ICON_RESOURCE="$resource" src/macos_file_handler.m -o "$executable"
  executables+=("$executable")
  make_icon "assets/file-icons/$icon.png" "$helper/Contents/Resources/FileIcon.icns"
  {
    bundle_header "org.qbitremote.desktop.file-icon-$resource" qbit-file-handler FileIcon
    document_types
    printf '%s\n' '<key>LSUIElement</key><true/>' '</dict></plist>'
  } > "$helper/Contents/Info.plist"
  plutil -lint "$helper/Contents/Info.plist"
  codesign --force --sign - "$helper"
  resource=$((resource + 1))
done

# macOS uses the main executable UUID to attribute local-network access to an app.
uuid_count=$(xcrun dwarfdump --uuid "${executables[@]}" | awk '/^UUID:/{print $2}' | sort -u | wc -l)
expected_uuids=$(( ${#executables[@]} * ${#architectures[@]} ))
[[ "$uuid_count" -eq "$expected_uuids" ]] || {
  echo "Missing or duplicate Mach-O UUID in qBit Remote" >&2
  exit 1
}

plutil -lint "$app/Contents/Info.plist"
codesign --force --sign - "$app"
codesign --verify --deep --strict "$app"
archive="qbit-remote-$version-$package_arch.zip"
ditto -c -k --sequesterRsrc --keepParent "$app" "$work/$archive"
mkdir -p "$output_dir"
if [[ -d "$output" ]]; then
  mv "$output" "$work/previous.app"
fi
mv "$app" "$output_dir/"
mv "$work/$archive" dist-macos/
shasum -a 256 "dist-macos/$archive"
