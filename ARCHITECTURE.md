# Architecture — Rachael's Transcriber

## Overview

Local offline audio transcription app. Two modes:
- **GUI**: Tauri v2 desktop window (React + Vite frontend, Rust backend)
- **CLI**: `cargo run -- --file input.wav` (pure Rust, no window)

All processing happens on-device with no internet dependency after the initial model download.

---

## System Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Tauri v2 Shell                        │
│  ┌────────────────────┐   ┌──────────────────────────┐  │
│  │  Rust Backend       │   │  React Frontend          │  │
│  │                     │   │                          │  │
│  │  ┌───────────────┐  │   │  ┌────────────────────┐  │  │
│  │  │  commands.rs   │◄─┼───┼─►│  invoke()          │  │  │
│  │  │  (12 commands) │  │   │  │  lib/commands.ts   │  │  │
│  │  └───────┬───────┘  │   │  └────────────────────┘  │  │
│  │          │           │   │                          │  │
│  │  ┌───────▼───────┐  │   │  ┌────────────────────┐  │  │
│  │  │  AppState      │  │   │  │  listen()          │  │  │
│  │  │  (shared state)│◄─┼───┼─►│  lib/events.ts    │  │  │
│  │  └───────┬───────┘  │   │  └────────────────────┘  │  │
│  │          │           │   │                          │  │
│  │  ┌───────▼───────┐  │   │  ┌────────────────────┐  │  │
│  │  │  Engine        │  │   │  │  App.tsx           │  │  │
│  │  │  Recorder      │  │   │  │  (Context + Views) │  │  │
│  │  │  Download      │  │   │  └────────────────────┘  │  │
│  │  │  Config        │  │   │                          │  │
│  │  └───────────────┘  │   │                          │  │
│  └────────────────────┘   └──────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

---

## Backend Modules (`src-tauri/src/`)

### `lib.rs` — AppState + Tauri Builder

Defines `AppState` — the shared state managed by Tauri:

| Field | Type | Purpose |
|---|---|---|
| `cancel_flag` | `Arc<AtomicBool>` | Cancel signal for long-running ops |
| `engine` | `Mutex<Option<WhisperEngine>>` | Loaded Whisper model (one at a time) |
| `recorder` | `Mutex<Option<AudioRecorder>>` | Active audio recorder |
| `save_audio_path` | `Mutex<Option<PathBuf>>` | Where to save recording .wav |
| `save_transcript_path` | `Mutex<Option<PathBuf>>` | Where to save transcript .txt |
| `vad_enabled` | `Arc<AtomicBool>` | VAD toggle |
| `samples_processed` | `Arc<AtomicUsize>` | Raw interleaved samples consumed by recording thread |

**`run()`** builds the Tauri app — sets model dir in `.setup()`, registers all 12 commands, generates context from `tauri.conf.json`.

### `main.rs` — CLI Entry Point

Uses `clap` for arg parsing. Two paths:
1. `--file <path>` → `run_cli()`: downloads model if missing, decodes audio, transcribes, prints/saves result
2. No `--file` or `--gui` → `rachaels_transcriber::run()` (launches Tauri)

### `config.rs` — Constants

Key constants:
- `SAMPLE_RATE = 16000` — Whisper target sample rate
- `CHUNK_SEC = 30.0` — File transcription chunk size (long files split into 30s chunks)
- `CHUNK_OVERLAP_SEC = 2.0` — Overlap between chunks to avoid cutting words
- `WINDOW_SEC = 4.0` — Duration of recording segments for live transcription
- `TICK_SEC = 2.0` — Polling interval for recording thread
- `MAX_BUFFER_SEC = 30.0` — Max audio buffer for the circular buffer

Model filenames:
- `ACCURATE = "ggml-small.en.bin"` (~465 MB)
- `FAST = "ggml-base.en.bin"` (~141 MB)

Model directory stored in `OnceLock<PathBuf>`, set during Tauri setup to `app.path().app_data_dir()/models/`. Falls back to `./models` for CLI mode.

### `commands.rs` — Tauri Commands (12)

**File Transcription Flow:**
1. Frontend invokes `transcribe_file(path, model)`
2. Command spawns a thread
3. Thread loads WhisperEngine → decodes audio via `chunk::prepare_audio()` → calls `engine.transcribe()` with progress callback
4. Emits `transcribe-progress` events during transcription
5. Emits `transcribe-done` with full text when complete
6. Emits `transcribe-error` on failure

**Live Recording Flow:**
1. Frontend invokes `start_recording(model, ...opts)`
2. Command loads engine, creates `AudioRecorder`, starts capturing via `cpal`
3. Spawns a polling thread:
   - Every `TICK_SEC` (2s), checks if `recorder.raw_len()` has enough audio for a `WINDOW_SEC` (4s) chunk
   - Reads chunk via `recorder.read_chunk(consumed, WINDOW_SEC)` — returns processed mono audio at 16kHz + raw samples consumed
   - Optionally runs VAD (`recorder.has_speech()`)
   - Transcribes chunk with Whisper
   - Emits `record-segment` (partial text) + `record-progress` (elapsed time)
   - Advances `samples_processed` by raw samples consumed
4. Frontend invokes `stop_recording()`:
   - Sets cancel flag → recording thread exits
   - Calls `recorder.stop()` → returns full resampled mono audio
   - Transcribes entire audio from scratch
   - Optionally saves .wav and .txt
   - Emits `transcribe-done` with clean full transcript
   - Frontend **replaces** live segments with this final text

**VAD (Voice Activity Detection):**
- Uses `webrtc-vad` crate
- Toggled via `vad_enabled` flag in AppState
- When enabled, skips transcription of chunks without speech (above 0.5 threshold)
- Reduces CPU usage during silence

**Cancel Signal:**
- `cancel_flag` atomic bool checked by long-running operations
- `cancel()` command sets it to `true`
- Recording thread, file transcription, and model download all check and abort

### `download_models.rs`

- Downloads GGML model files from `huggingface.co/ggerganov/whisper.cpp`
- Sequential: Fast first (~141 MB), then Accurate (~465 MB)
- Progress callback reports combined 0-100% (weighted by file size: 23.3% Fast + 76.7% Accurate)
- Downloads to `models_dir()` with `.tmp` extension, renames on completion
- Cancellation support via `AtomicBool`
- Uses `reqwest` with `rustls-tls` (no native OpenSSL dependency)

### `engine/whisper.rs` — WhisperEngine

- Wraps `whisper_rs` (Rust bindings for whisper.cpp)
- Public API:
  - `new(model_path, language)` — loads GGML model
  - `transcribe(audio, cancel, progress)` — full audio transcription with segment timestamps
  - `transcribe_chunked(audio, chunk_sec, overlap_sec, cancel, progress)` — splits long audio into overlapping chunks, deduplicates at boundaries
- `segments_to_text()` — joins segment text with spaces
- Segments contain `start`, `end`, `text` fields (timestamps in seconds)

### `engine/chunk.rs` — Audio Processing

- `decode_audio(path)` — uses `symphonia` to decode any supported format (mp3, wav, flac, m4a, ogg, aac, wma, etc.)
- `convert_to_mono(samples, channels)` — averages multi-channel to mono
- `resample_to_16khz(samples, from_rate)` — uses `rubato` sinc resampler for high-quality conversion
- `prepare_audio(path)` — decode + mono + resample pipeline
- `duration_from_path()` / `extract_chunk()` — utility functions

### `recorder/mod.rs` — AudioRecorder

- Uses `cpal` for cross-platform audio capture
- **Raw audio buffer**: `Arc<Mutex<Vec<f32>>>` — append-only capture buffer
- **Circular buffer**: `Arc<Mutex<VecDeque<f32>>>` — for `get_buffer()` (used by legacy overlap-based approach)
- Key methods:
  - `start(wav_path)` — opens default input device, captures audio into both buffers
  - `stop()` — returns full captured audio (converted to mono, resampled to 16kHz)
  - `raw_len()` — number of raw interleaved samples captured
  - `read_chunk(start_raw, duration_sec)` — reads a slice of raw audio starting at a specific position, covering `duration_sec` seconds at the actual mic sample rate. Computes frames from actual sample rate (not assumed 16kHz), downmixes to mono, resamples to 16kHz. Returns `(audio_chunk, raw_consumed)`.
  - `has_speech(audio, threshold)` — VAD check on processed audio
  - `elapsed()` — recording duration

### `recorder/vad.rs` — Voice Activity Detection

- Wraps `webrtc-vad` with silero-inspired energy-based VAD
- Splits audio into 30ms frames, checks RMS energy against adaptive threshold

---

## Frontend Modules (`src/`)

### `App.tsx` — Context & View Router

- `AppCtx` provides: `view`, `setView`, `models`, `status`, `setStatus`, `outputText`, `setOutputText`, `modelChoice`, `setModelChoice`
- On mount: `checkModels()` → if any missing, auto-download via `downloadModels()`
- Download progress shown as full-screen overlay with `Progress` bar
- Three views rendered conditionally:
  - `"landing"` → `<Landing />`
  - `"file"` → `<FileTranscribe />`
  - `"record"` → `<LiveRecord />`

### `components/Layout.tsx` — Shared Sidebar Shell

- Common wrapper for both transcription views
- Uses shadcn `SidebarProvider` + `Sidebar` (variant="floating") + `SidebarInset`
- Sidebar has Back button (`ArrowLeft`) + view label in header
- `SidebarContent` receives sidebar children
- SidebarFooter shows error messages
- Main content area has `SidebarTrigger` + "Transcription" header
- Sidebar width: `14rem` via CSS custom property

### `components/Landing.tsx`

- Two large cards: "File transcription" (blue icon) and "Live recording" (red icon)
- Clicking sets view via `setView()`

### `components/FileTranscribe.tsx`

**Sidebar:**
- File picker button → `pickAudioFile()`
- Model select dropdown
- Transcribe button / Cancel button + progress bar

**Main content:**
- Progress indicator during transcription
- Read-only `<Textarea>` with transcript output
- "Save to File" button → `pickSaveFile()` + `saveTextFile()`

### `components/LiveRecord.tsx`

**Sidebar:**
- Model select
- "Save audio as" → `pickAudioSaveFile()` (.wav filter)
- "Save transcript as" → `pickSaveFile()`
- VAD switch toggle
- Start/Stop button (red "Stop" with square + red timestamp during recording)

**Main content:**
- Live segment text shown as italic subtitle during recording
- Read-only `<Textarea>` with accumulated transcript
- `onRecordSegment` appends text during recording (live preview)
- `onTranscribeDone` **replaces** full text with clean final transcript

### `lib/commands.ts`

- TypeScript `invoke()` wrappers for all 12 Tauri commands
- Interfaces for payload types: `MissingModels`, `DownloadProgress`, `TranscribeProgress`, `RecordProgress`

### `lib/events.ts`

- `listen()` wrappers for all 7 event types
- `UnlistenFn` re-exported for cleanup
- Event types: `download-progress`, `download-done`, `transcribe-progress`, `transcribe-done`, `transcribe-error`, `record-progress`, `record-segment`

### `index.css` — Styling

- Tailwind v4 `@import "tailwindcss"` (no config file)
- CSS custom properties for light and dark themes via `prefers-color-scheme`
- `@theme inline` block maps CSS vars to Tailwind utility classes
- `color-scheme: light dark` on `:root`
- No `.dark` class — purely system theme following

---

## Data Flow Diagrams

### File Transcription
```
User clicks "Select Audio File" → pickAudioFile() → rfd file dialog
User selects model → modelChoice state
User clicks "Transcribe" → transcribeFile(path, model)
  → Rust thread: load engine → decode audio → transcribe
    → periodically emits transcribe-progress { progress, text }
    → on completion emits transcribe-done { text }
    → on error emits transcribe-error { message }
  → Frontend: onTranscribeProgress updates progress bar
  → Frontend: onTranscribeDone sets outputText + status("done")
```

### Live Recording
```
User clicks "Start" → startRecording(model, ...opts)
  → Rust: load engine → create AudioRecorder → start cpal stream → spawn polling thread
  → Polling thread (every 2s):
    → read_chunk(consumed, 4s) → raw audio → mono → resample → whisper → text
    → emit record-segment { text }
    → frontend: appends text to outputText (live preview)
  → User clicks "Stop" → stopRecording()
    → set cancel flag → thread exits
    → recorder.stop() → full mono 16kHz audio
    → transcribe entire audio → emit transcribe-done { text }
    → optionally save .wav + .txt
  → Frontend: onTranscribeDone REPLACES outputText with clean final text
```
