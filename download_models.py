#!/usr/bin/env python3
import argparse
import sys
from pathlib import Path

from huggingface_hub import snapshot_download

MODELS = {
    "small.en": "Systran/faster-whisper-small.en",
    "base.en": "Systran/faster-whisper-base.en",
}

HERE = Path(__file__).parent.resolve()
MODELS_DIR = HERE / "models"


def download(name):
    repo_id = MODELS[name]
    dest = MODELS_DIR / name

    if dest.is_dir() and any(dest.iterdir()):
        print(f"  [{name}] already exists at {dest}")
        return

    print(f"  [{name}] downloading {repo_id} ...")
    dest.mkdir(parents=True, exist_ok=True)
    snapshot_download(
        repo_id=repo_id,
        local_dir=str(dest),
    )
    print(f"  [{name}] saved to {dest}")


def main():
    parser = argparse.ArgumentParser(
        description="Pre-download whisper models for offline use."
    )
    parser.add_argument("--accurate", action="store_true",
                        help="Download small.en (466 MB, accurate)")
    parser.add_argument("--fast", action="store_true",
                        help="Download base.en (142 MB, fast)")
    args = parser.parse_args()

    if not (args.accurate or args.fast):
        args.accurate = args.fast = True

    if args.accurate:
        download("small.en")
    if args.fast:
        download("base.en")

    print("Done.")


if __name__ == "__main__":
    main()
