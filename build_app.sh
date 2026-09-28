#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")"

cargo build --release

APP_NAME="Spine"
APP_DIR="dist/${APP_NAME}.app"
BIN_NAME="spine"
LOGO="assets/logo.svg"
BACKGROUND_SVG="assets/dmg-background.svg"
VERSION="$(grep '^version' Cargo.toml | head -1 | sed -E 's/version = "(.*)"/\1/')"

rm -rf "$APP_DIR"
mkdir -p "$APP_DIR/Contents/MacOS" "$APP_DIR/Contents/Resources"

cp "target/release/${BIN_NAME}" "$APP_DIR/Contents/MacOS/${APP_NAME}"
chmod +x "$APP_DIR/Contents/MacOS/${APP_NAME}"

# --- App icon: pad the source logo to a square canvas, then build a standard
# .iconset and compile it to .icns with the built-in iconutil. ---
if [ -f "$LOGO" ] && command -v ffmpeg >/dev/null && command -v iconutil >/dev/null; then
    ICON_TMP="$(mktemp -d)"
    trap 'rm -rf "$ICON_TMP"' EXIT

    LOGO_PNG="$ICON_TMP/logo.png"
    ffmpeg -y -loglevel error -i "$LOGO" -vf "scale=1024:1024" "$LOGO_PNG"

    LOGO_W=$(sips -g pixelWidth "$LOGO_PNG" | awk '/pixelWidth/{print $2}')
    LOGO_H=$(sips -g pixelHeight "$LOGO_PNG" | awk '/pixelHeight/{print $2}')
    SIDE=$((LOGO_W > LOGO_H ? LOGO_W : LOGO_H))
    ffmpeg -y -loglevel error -i "$LOGO_PNG" \
        -vf "format=rgba,pad=${SIDE}:${SIDE}:(${SIDE}-${LOGO_W})/2:(${SIDE}-${LOGO_H})/2:color=0x00000000" \
        "$ICON_TMP/icon-square.png"

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
    <string>${VERSION}</string>
    <key>CFBundleShortVersionString</key>
    <string>${VERSION}</string>
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

# --- Distributable .dmg: Spine.app + an Applications drop link, a background
# image with an arrow explaining the drag-and-drop, laid out via create-dmg. ---
DMG_NAME="${APP_NAME}-v${VERSION}"
DMG_PATH="dist/${DMG_NAME}.dmg"
rm -f "$DMG_PATH"

STAGING="dist/dmg-staging"
rm -rf "$STAGING"
mkdir -p "$STAGING"
cp -R "$APP_DIR" "$STAGING/"

if [ -f "$BACKGROUND_SVG" ] && command -v qlmanage >/dev/null; then
    BG_TMP="$(mktemp -d)"
    qlmanage -t -s 660 -o "$BG_TMP" "$BACKGROUND_SVG" >/dev/null 2>&1
    ffmpeg -y -loglevel error -i "$BG_TMP/$(basename "$BACKGROUND_SVG").png" \
        -vf "crop=660:400:0:0" -update 1 "$BG_TMP/background.png"
    BACKGROUND_PNG="$BG_TMP/background.png"
else
    BACKGROUND_PNG=""
fi

create-dmg \
    --volname "$APP_NAME" \
    --volicon "$APP_DIR/Contents/Resources/AppIcon.icns" \
    ${BACKGROUND_PNG:+--background "$BACKGROUND_PNG"} \
    --window-pos 200 120 \
    --window-size 660 400 \
    --icon-size 128 \
    --icon "${APP_NAME}.app" 180 170 \
    --hide-extension "${APP_NAME}.app" \
    --app-drop-link 480 170 \
    --no-internet-enable \
    "$DMG_PATH" \
    "$STAGING"

[ -f "$BACKGROUND_PNG" ] && rm -rf "$(dirname "$BACKGROUND_PNG")"
rm -rf "$STAGING"

echo "Disk image created at: $DMG_PATH"
