#!/usr/bin/env bash
set -euo pipefail

APP_NAME="RachaelsTranscriber"
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
    --add-data "assets:assets" \
    --add-data "theme.json:." \
    --hidden-import download_models \
    --collect-data faster_whisper \
    --noconfirm \
    --icon "assets/icon.icns" \
    gui.py

echo ""
echo "Build complete!"
echo "  dist/$APP_NAME"
echo ""
echo "Models are downloaded on first launch (internet required)."
echo "To open: open dist/$APP_NAME"
