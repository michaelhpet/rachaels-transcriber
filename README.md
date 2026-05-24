# Rachael's Transcriber

Local offline audio transcription powered by Whisper.

Transcribe audio files or record from your microphone — all processing happens on-device with no internet dependency after the initial model download.

## Usage

```sh
# Transcribe a file
rachaels-transcriber --file speech.mp3

# Use accurate model
rachaels-transcriber --file speech.mp3 --model Accurate

# Save transcript
rachaels-transcriber --file speech.mp3 --output transcript.txt

# Launch GUI
rachaels-transcriber --gui
```

Models (GGML format) are downloaded from HuggingFace on first run.

## Build

```sh
cargo build --release
```

Binary is at `target/release/rachaels-transcriber` (or `target/debug/rachaels-transcriber` without `--release`).
