#!/usr/bin/env bash
set -e

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )/.." && pwd )"
cd "$DIR"

echo "Building macOS Anti-Pet.app bundle (v0.5.1 NextGen Core)..."

APP_DIR="dist/Anti-Pet.app"
rm -rf "$APP_DIR"
mkdir -p "$APP_DIR/Contents/MacOS"
mkdir -p "$APP_DIR/Contents/Resources"

# 1. Locate compiled binary (or build if cargo is available)
BIN_PATH="${1:-desktop/target/release/anti-pet-desktop}"
if [ ! -f "$BIN_PATH" ]; then
    if [ -f "desktop/target/aarch64-apple-darwin/release/anti-pet-desktop" ]; then
        BIN_PATH="desktop/target/aarch64-apple-darwin/release/anti-pet-desktop"
    elif [ -f "desktop/target/x86_64-apple-darwin/release/anti-pet-desktop" ]; then
        BIN_PATH="desktop/target/x86_64-apple-darwin/release/anti-pet-desktop"
    elif command -v cargo >/dev/null 2>&1; then
        echo "Compiling Pure CPU 2D release binary via cargo..."
        cargo build --release --manifest-path desktop/Cargo.toml --features cpu-2d
    fi
fi

if [ ! -f "$BIN_PATH" ]; then
    echo "Error: Binary not found at $BIN_PATH"
    exit 1
fi

echo "Using binary: $BIN_PATH"
cp "$BIN_PATH" "$APP_DIR/Contents/MacOS/anti-pet-desktop"

# 2. Copy Resources (icon and skins)
cp assets/icon.icns "$APP_DIR/Contents/Resources/"
cp -r skins "$APP_DIR/Contents/Resources/"

# 3. Create Info.plist
cat << 'EOF' > "$APP_DIR/Contents/Info.plist"
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleName</key>
    <string>Anti-Pet</string>
    <key>CFBundleDisplayName</key>
    <string>大肥猫桌宠挂件</string>
    <key>CFBundleIdentifier</key>
    <string>com.tianyayou.anti-pet</string>
    <key>CFBundleVersion</key>
    <string>0.5.1</string>
    <key>CFBundleShortVersionString</key>
    <string>0.5.1</string>
    <key>CFBundleExecutable</key>
    <string>anti-pet-desktop</string>
    <key>CFBundleIconFile</key>
    <string>icon</string>
    <key>LSUIElement</key>
    <true/>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
EOF

# 4. Ad-hoc sign for local macOS execution
if command -v codesign >/dev/null 2>&1; then
    codesign --force --deep --sign - "$APP_DIR" >/dev/null 2>&1 || true
fi

echo "Anti-Pet.app (v0.5.1 NextGen Core) successfully assembled at $APP_DIR"
