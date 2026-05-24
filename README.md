# Rachael's Transcriber

Local offline audio transcription powered by Whisper. Transcribe audio files or record from your microphone — all on-device with no internet dependency after the initial model download.

**Tech stack**: Tauri v2 • Rust • React 19 • TypeScript • Vite 8 • Tailwind v4 • shadcn/ui • Biome

---

## Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [pnpm](https://pnpm.io/installation) (corepack: `corepack enable pnpm`)
- macOS: Xcode Command Line Tools (`xcode-select --install`)

## Quick Start

```sh
# Install frontend dependencies
pnpm install

# Build Rust backend (first time downloads crates)
cargo build

# Run in development mode (Tauri window + Vite HMR)
pnpm tauri dev
```

## CLI Usage

```sh
# Transcribe a file with default (Fast) model
cargo run -- --file speech.mp3

# Use accurate model
cargo run -- --file speech.mp3 --model Accurate

# Save transcript
cargo run -- --file speech.mp3 --output transcript.txt

# Launch GUI (default when no --file)
cargo run -- --gui
```

## Build for Distribution

```sh
pnpm tauri build
```

Binary output at `src-tauri/target/release/rachaels-transcriber` and a `.dmg` bundle (macOS) in `src-tauri/target/release/bundle/`.

## Models

Two Whisper GGML models auto-downloaded from [HuggingFace](https://huggingface.co/ggerganov/whisper.cpp) on first launch:

| Model | File | Size |
|---|---|---|
| Fast (default) | `ggml-base.en.bin` | ~141 MB |
| Accurate | `ggml-small.en.bin` | ~465 MB |

Stored in the Tauri app data directory (`~/Library/Application Support/com.rachaels.transcriber/models/` on macOS).

## Development Scripts

| Command | Description |
|---|---|
| `pnpm dev` | Vite dev server (port 1420) |
| `pnpm build` | Build frontend |
| `pnpm tauri dev` | Full Tauri dev (Vite HMR + Rust) |
| `pnpm format` | Format + lint with Biome |
| `cargo build` | Build Rust backend only |
| `cargo clippy` | Rust linting |

## Project Structure

```
├── src/              # React frontend
│   ├── components/   # UI components (Landing, FileTranscribe, LiveRecord, Layout)
│   ├── lib/          # Tauri command wrappers + event listeners
│   └── hooks/        # shadcn auto-generated hooks
├── src-tauri/        # Rust backend
│   └── src/
│       ├── commands.rs       # 12 Tauri commands
│       ├── engine/           # Whisper inference + audio processing
│       ├── recorder/         # Audio capture (cpal) + VAD
│       ├── download_models.rs
│       └── config.rs
├── AGENTS.md         # Guidance for AI coding agents
└── ARCHITECTURE.md   # Detailed architecture documentation
```

## License

MIT
