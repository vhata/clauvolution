#!/bin/bash
# Regenerate the macOS .icns from the 1024px source PNG.
# Run this if you replace helix_1024.png with a new design.
#
# Usage: ./scripts/build_icon.sh

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT/crates/clauvolution_app/assets/icons"

SRC="helix_1024.png"
ICONSET="clauvolution.iconset"
ICNS="clauvolution.icns"

if [[ ! -f "$SRC" ]]; then
    echo "error: $SRC not found (expected a 1024x1024 PNG)" >&2
    exit 1
fi

rm -rf "$ICONSET"
mkdir -p "$ICONSET"

for size in 16 32 128 256 512; do
    sips -Z "$size" "$SRC" --out "$ICONSET/icon_${size}x${size}.png" > /dev/null
    sips -Z "$((size * 2))" "$SRC" --out "$ICONSET/icon_${size}x${size}@2x.png" > /dev/null
done

iconutil -c icns "$ICONSET" -o "$ICNS"
rm -rf "$ICONSET"

echo "Generated $ICNS"
