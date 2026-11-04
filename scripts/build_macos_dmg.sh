#!/usr/bin/env bash
set -e

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )/.." && pwd )"
cd "$DIR"

echo "Building macOS Anti-Pet.app bundle first..."
./scripts/build_macos_app.sh "$1"

APP_DIR="dist/Anti-Pet.app"
DMG_OUT="${2:-dist/Anti-Pet-macOS.dmg}"

echo "Assembling standard Drag-and-Drop DMG installer..."
DMG_STAGING="dist/dmg_staging"
rm -rf "$DMG_STAGING"
mkdir -p "$DMG_STAGING"
cp -r "$APP_DIR" "$DMG_STAGING/"
ln -s /Applications "$DMG_STAGING/Applications"

hdiutil create -volname "Anti-Pet" -srcfolder "$DMG_STAGING" -ov -format UDZO "$DMG_OUT"
rm -rf "$DMG_STAGING"

echo "DMG successfully built at $DMG_OUT"
shasum -a 256 "$DMG_OUT"
