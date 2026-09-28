#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")"

cargo build --release

APP_NAME="Spine"
APP_DIR="dist/${APP_NAME}.app"
BIN_NAME="spine"
LOGO="assets/logo.png"

rm -rf "$APP_DIR"
mkdir -p "$APP_DIR/Contents/MacOS" "$APP_DIR/Contents/Resources"

cp "target/release/${BIN_NAME}" "$APP_DIR/Contents/MacOS/${APP_NAME}"
chmod +x "$APP_DIR/Contents/MacOS/${APP_NAME}"

# --- App icon: crop the character/wolf square out of the wide source logo, then
# build a standard .iconset and compile it to .icns with the built-in iconutil. ---
if [ -f "$LOGO" ] && command -v ffmpeg >/dev/null && command -v iconutil >/dev/null; then
    ICON_TMP="$(mktemp -d)"
    trap 'rm -rf "$ICON_TMP"' EXIT

    ffmpeg -y -loglevel error -i "$LOGO" -vf "crop=2160:2160:650:0" -update 1 "$ICON_TMP/icon-square.png"

    ICONSET="$ICON_TMP/AppIcon.iconset"
    mkdir -p "$ICONSET"
    for size in 16 32 128 256 512; do
        sips -z "$size" "$size" "$ICON_TMP/icon-square.png" --out "$ICONSET/icon_${size}x${size}.png" >/dev/null
        double=$((size * 2))
        sips -z "$double" "$double" "$ICON_TMP/icon-square.png" --out "$ICONSET/icon_${size}x${size}@2x.png" >/dev/null
    done

    iconutil -c icns "$ICONSET" -o "$APP_DIR/Contents/Resources/AppIcon.icns"
    ICON_PLIST_ENTRY="    <key>CFBundleIconFile</key>
    <string>AppIcon</string>"
else
    echo "Warning: skipping app icon (missing $LOGO, ffmpeg, or iconutil)."
    ICON_PLIST_ENTRY=""
fi

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
    <string>com.valentin.spine</string>
    <key>CFBundleVersion</key>
    <string>1.0</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleExecutable</key>
    <string>${APP_NAME}</string>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>LSApplicationCategoryType</key>
    <string>public.app-category.utilities</string>
${ICON_PLIST_ENTRY}
</dict>
</plist>
PLIST

codesign --force --deep --sign - "$APP_DIR"

echo "App bundle created at: $APP_DIR"

# --- Distributable .dmg ---
DMG_PATH="dist/${APP_NAME}.dmg"
rm -f "$DMG_PATH"
hdiutil create -volname "$APP_NAME" -srcfolder "$APP_DIR" -ov -format UDZO "$DMG_PATH" >/dev/null
echo "Disk image created at: $DMG_PATH"
