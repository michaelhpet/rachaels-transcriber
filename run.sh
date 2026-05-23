#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

# Use venv if available, otherwise system python
if [ -d .venv ]; then
    source .venv/bin/activate
    PYTHON="python3"
else
    PYTHON="python3.12"
fi

# Check if running in CLI or GUI mode
if [ $# -ge 1 ]; then
    exec $PYTHON transcribe.py "$@"
else
    exec $PYTHON gui.py
fi
