#!/usr/bin/env bash
set -euo pipefail

APP_NAME="Rachael's Transcriber"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "Building $APP_NAME for macOS..."

cd "$SCRIPT_DIR"

# Install dependencies if needed
pip install -r requirements.txt
pip install pyinstaller

# Pre-download models
python3 download_models.py

# Build the .app bundle
pyinstaller \
    --onefile \
    --windowed \
    --name "$APP_NAME" \
    --add-data "engine.py:." \
    --add-data "models:models" \
    --add-data "assets:assets" \
    --icon "assets/icon.icns" \
    gui.py

echo ""
echo "Build complete! App bundle:"
echo "  dist/$APP_NAME.app"
echo ""
echo "To open: open dist/$APP_NAME.app"
