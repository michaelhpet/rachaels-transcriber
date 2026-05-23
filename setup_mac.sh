#!/usr/bin/env bash
set -euo pipefail

echo "Setting up Rachael's Transcriber for macOS..."
echo ""

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

# Check for Homebrew Python
PYTHON=""
for cmd in python3.12 python3.13 python3.11 python3; do
    if command -v "$cmd" &> /dev/null; then
        VER=$($cmd --version 2>&1 | grep -oE '[0-9]+\.[0-9]+')
        MAJOR=${VER%.*}
        MINOR=${VER#*.}
        if [ "$MAJOR" -ge 3 ] && [ "$MINOR" -ge 11 ]; then
            PYTHON="$cmd"
            break
        fi
    fi
done

if [ -z "$PYTHON" ]; then
    echo "Installing Python 3.12 via Homebrew..."
    brew install python@3.12
    PYTHON="python3.12"
fi

echo "Using: $($PYTHON --version)"

# Ensure ffmpeg
if ! command -v ffmpeg &> /dev/null; then
    echo "Installing ffmpeg..."
    brew install ffmpeg
fi

# Ensure tkinter support
if ! $PYTHON -c "import tkinter; tkinter.Tk().destroy()" 2>/dev/null; then
    echo "Installing tkinter support..."
    brew install python-tk@3.12
fi

# Create venv
echo "Creating virtual environment..."
$PYTHON -m venv .venv
source .venv/bin/activate

echo "Installing dependencies..."
pip install -r requirements.txt
pip install pyinstaller

echo ""
echo "Downloading speech models..."
python3 download_models.py

echo ""
echo "================================================"
echo "  Setup complete!"
echo ""
echo "  Run GUI:   source .venv/bin/activate && python3 gui.py"
echo "  or:        ./run.sh"
echo ""
echo "  Run CLI:   source .venv/bin/activate && python3 transcribe.py audio.mp3"
echo "  or:        ./run.sh audio.mp3"
echo "================================================"
