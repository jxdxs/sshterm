#!/usr/bin/env bash
# Generate placeholder icons for Tauri app
# Run: bash scripts/gen-icons.sh

set -e

ICONS_DIR="src-tauri/icons"
mkdir -p "$ICONS_DIR"

# Generate simple SVG placeholder icon
cat > /tmp/sshterm-icon.svg << 'SVGEOF'
<svg xmlns="http://www.w3.org/2000/svg" width="512" height="512" viewBox="0 0 512 512">
  <rect width="512" height="512" rx="80" fill="#1a1b26"/>
  <text x="256" y="290" text-anchor="middle" font-size="280" font-family="monospace" font-weight="bold" fill="#7aa2f7">&gt;_</text>
</svg>
SVGEOF

# Convert to PNG using ImageMagick if available
if command -v convert &>/dev/null; then
  echo "Generating icons with ImageMagick..."
  convert /tmp/sshterm-icon.svg -resize 32x32 "$ICONS_DIR/32x32.png"
  convert /tmp/sshterm-icon.svg -resize 128x128 "$ICONS_DIR/128x128.png"
  convert /tmp/sshterm-icon.svg -resize 256x256 "$ICONS_DIR/128x128@2x.png"
  convert /tmp/sshterm-icon.svg -resize 256x256 "$ICONS_DIR/icon.png"
  
  # macOS .icns (requires png2icns or iconutil on macOS)
  if command -v iconutil &>/dev/null; then
    echo "Generating .icns..."
    mkdir -p /tmp/sshterm-icon.iconset
    convert /tmp/sshterm-icon.svg -resize 16x16 /tmp/sshterm-icon.iconset/icon_16x16.png
    convert /tmp/sshterm-icon.svg -resize 32x32 /tmp/sshterm-icon.iconset/icon_16x16@2x.png
    convert /tmp/sshterm-icon.svg -resize 32x32 /tmp/sshterm-icon.iconset/icon_32x32.png
    convert /tmp/sshterm-icon.svg -resize 64x64 /tmp/sshterm-icon.iconset/icon_32x32@2x.png
    convert /tmp/sshterm-icon.svg -resize 128x128 /tmp/sshterm-icon.iconset/icon_128x128.png
    convert /tmp/sshterm-icon.svg -resize 256x256 /tmp/sshterm-icon.iconset/icon_128x128@2x.png
    convert /tmp/sshterm-icon.svg -resize 256x256 /tmp/sshterm-icon.iconset/icon_256x256.png
    convert /tmp/sshterm-icon.svg -resize 512x512 /tmp/sshterm-icon.iconset/icon_256x256@2x.png
    convert /tmp/sshterm-icon.svg -resize 512x512 /tmp/sshterm-icon.iconset/icon_512x512.png
    convert /tmp/sshterm-icon.svg -resize 1024x1024 /tmp/sshterm-icon.iconset/icon_512x512@2x.png
    iconutil -c icns /tmp/sshterm-icon.iconset -o "$ICONS_DIR/icon.icns"
  fi

  # Windows .ico
  if command -v magick &>/dev/null; then
    magick convert /tmp/sshterm-icon.svg -resize 256x256 "$ICONS_DIR/icon.ico"
  elif command -v convert &>/dev/null; then
    convert /tmp/sshterm-icon.svg -resize 256x256 "$ICONS_DIR/icon.ico"
  fi
  
  echo "Icons generated successfully!"
else
  echo "ImageMagick not found. Creating placeholder PNG files..."
  # Create minimal valid PNG files as placeholders
  for size in 32x32 128x128; do
    cp /tmp/sshterm-icon.svg "$ICONS_DIR/${size}.svg"
  done
  # Tauri needs .ico and .icns — just copy SVG as fallback
  cp /tmp/sshterm-icon.svg "$ICONS_DIR/icon.ico" 2>/dev/null || true
  cp /tmp/sshterm-icon.svg "$ICONS_DIR/icon.icns" 2>/dev/null || true
  echo "Placeholder icons created. Install ImageMagick for proper generation."
fi

echo "Done!"
