# Rachael's Transcriber

Transcribe audio files to text using OpenAI's Whisper models — fully offline, no API keys, no data leaves your machine.

Uses [faster-whisper](https://github.com/SYSTRAN/faster-whisper) for up to 4x faster transcription than the original Whisper, with INT8 quantization and low memory usage.

## Features

- **Offline & private** — runs entirely on your computer, no internet needed after model download
- **GUI & CLI** — graphical interface with CustomTkinter, or command-line for scripting
- **Live Recording** — record from microphone and transcribe in real-time
- **English only** — optimized `small.en` (accurate) and `base.en` (fast) models
- **VAD filtering** — Voice Activity Detection skips silence for cleaner transcripts
- **Auto-save** — transcripts always saved to a file, no checkbox needed
- **Incremental output** — partial results written during long recordings/transcriptions
- **Cross-platform** — Windows, macOS, Linux
- **Low resource** — runs on 2-core Intel i3 with 4–8 GB RAM

## Requirements

- Python 3.11+
- [ffmpeg](https://ffmpeg.org/) (for audio decoding)
- 1–2 GB free RAM

## Quick Start

### macOS

```bash
# Install ffmpeg
brew install ffmpeg

# Setup
chmod +x setup_mac.sh
./setup_mac.sh

# Run GUI
source .venv/bin/activate
python3 gui.py
```

### Windows

```powershell
# 1. Install Python 3.12 from https://python.org
# 2. Install ffmpeg:   choco install ffmpeg
# 3. Run setup:
pip install -r requirements.txt
# 4. Launch GUI:
python gui.py
```

### Linux

```bash
# Install ffmpeg
sudo apt install ffmpeg    # Debian/Ubuntu
sudo dnf install ffmpeg    # Fedora

# Setup and run
python3 -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt
python3 gui.py
```

## Usage

### GUI

```bash
source .venv/bin/activate
python3 gui.py
```

**File Transcription:**
1. Click **Browse** and select an audio file (mp3, wav, m4a, flac, ogg, etc.)
2. Choose **Accurate** (`small.en`) or **Fast** (`base.en`)
3. Toggle **VAD** and **Word timestamps** as needed
4. Set the auto-save path (defaults beside the audio file)
5. Click **Transcribe** (button turns red with Cancel while running)

**Live Recording:**
1. Set **Save Audio** path for the WAV file
2. Set **Save Transcript** path for the output text
3. Click **Record** (button turns red with Stop while recording)
4. Partial transcript appears in real-time; click Stop to finalize

### CLI

```bash
# Transcribe a file
python3 transcribe.py audio.mp3

# Specify model and options
python3 transcribe.py recording.mp3 --model small --vad --word-timestamps

# Custom output path (always writes .txt)
python3 transcribe.py audio.mp3 -o transcript.txt

# Record from microphone (5-second silence timeout)
python3 transcribe.py --record

# Full options
python3 transcribe.py --help
```

### CLI Options

| Flag | Default | Description |
|---|---|---|
| `audio` | — | Path to audio file (required for file mode) |
| `--model` | `base` | Model: `base` (fast) or `small` (accurate) |
| `-o, --output` | input + `_transcript.txt` | Output file path |
| `--vad` | off | Voice Activity Detection (skip silence) |
| `--word-timestamps` | off | Include word-level timestamps |
| `--device` | `auto` | Compute device: auto, cpu, cuda |
| `--record` | off | Record from microphone instead of file |

## Model Comparison

| Model | Size | Speed | Accuracy | RAM |
|---|---|---|---|---|
| `base.en` | 141 MB | fast | good | ~1 GB |
| `small.en` | 464 MB | moderate | better | ~2 GB |

Start with `base.en`. If accuracy is insufficient, try `small.en`.

Models are downloaded to `models/` via `download_models.py` during setup.

## Project Structure

```
rachaels-transcriber/
├── engine.py              # Core transcription (faster-whisper wrapper)
├── gui.py                 # CustomTkinter graphical interface
├── transcribe.py          # CLI entry point
├── recorder.py            # AudioRecorder (sounddevice, VAD, ring buffer)
├── download_models.py     # Pre-downloads models to models/
├── theme.json             # CustomTkinter theme (macOS blue accent)
├── requirements.txt       # Python dependencies
├── setup_mac.sh           # macOS setup script
├── build_mac.sh           # macOS PyInstaller script
├── build_win.bat          # Windows build script (cmd)
├── build_win.ps1          # Windows build script (PowerShell)
├── run.sh                 # macOS launcher
├── run.bat                # Windows launcher
├── models/                # Downloaded model files (bundled by PyInstaller)
├── test_audio/            # Sample test files
└── .github/workflows/     # CI build workflow
```

## Building a Windows Executable

### Option 1: GitHub Actions (automatic)

Push the project to GitHub. The included workflow (`.github/workflows/build.yml`) builds the `.exe` on a free Windows runner. Download from **Actions > Build Windows .exe**:

```bash
git init && git add -A && git commit -m "Initial commit"
gh repo create rachaels-transcriber --public --push
# Then visit: https://github.com/YOUR_USER/rachaels-transcriber/actions
```

### Option 2: Build on Windows directly

```powershell
pip install -r requirements.txt
pip install pyinstaller
pyinstaller --onefile --windowed --name "RachaelsTranscriber" --icon assets\icon.ico gui.py
```

Output: `dist/RachaelsTranscriber.exe` — a single file. Transfer it to any Windows machine (requires [ffmpeg](https://ffmpeg.org/) installed separately).

## License

MIT
