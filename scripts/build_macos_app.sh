#!/bin/bash
# Wrap the release binary + icon + Info.plist into a proper macOS .app
# bundle at `dist/Clauvolution.app`. That's what macOS needs to show our
# icon in the dock instead of the terminal's. Does NOT sign, notarise,
# or distribute — personal use only.
#
# Usage: ./scripts/build_macos_app.sh

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

APP_NAME="Clauvolution"
BUNDLE_ID="com.jonathanhitchcock.clauvolution"
APP_DIR="dist/${APP_NAME}.app"
ICNS="crates/clauvolution_app/assets/icons/clauvolution.icns"

if [[ ! -f "$ICNS" ]]; then
    echo "error: $ICNS not found. Generate it first via the iconset workflow." >&2
    exit 1
fi

echo "Building release binary..."
cargo build --release --bin clauvolution

echo "Assembling $APP_DIR"
rm -rf "$APP_DIR"
mkdir -p "$APP_DIR/Contents/MacOS"
mkdir -p "$APP_DIR/Contents/Resources"

cp target/release/clauvolution "$APP_DIR/Contents/MacOS/${APP_NAME}"
cp "$ICNS" "$APP_DIR/Contents/Resources/icon.icns"

# Copy the runtime assets the binary loads (fonts etc.). Bevy's AssetServer
# looks next to the executable under `assets/` when bundled this way.
if [[ -d "assets" ]]; then
    cp -R assets "$APP_DIR/Contents/MacOS/"
fi
if [[ -d "crates/clauvolution_app/assets" ]]; then
    # Prefer the app crate's assets — they're the ones the binary loads.
    cp -R crates/clauvolution_app/assets "$APP_DIR/Contents/MacOS/"
fi

# Info.plist — minimum fields macOS needs to treat this as a proper app
# and use our icon. Agent-free (it's a foreground GUI).
cat > "$APP_DIR/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>${APP_NAME}</string>
    <key>CFBundleDisplayName</key>
    <string>${APP_NAME}</string>
    <key>CFBundleIdentifier</key>
    <string>${BUNDLE_ID}</string>
    <key>CFBundleExecutable</key>
    <string>${APP_NAME}</string>
    <key>CFBundleIconFile</key>
    <string>icon</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleVersion</key>
    <string>0.1.0</string>
    <key>CFBundleShortVersionString</key>
    <string>0.1.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
    <key>NSSupportsAutomaticGraphicsSwitching</key>
    <true/>
</dict>
</plist>
PLIST

echo "Done. Open with: open $APP_DIR"
