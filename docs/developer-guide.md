# Developer Guide — Rachael's Transcriber

## Overview

Rachael's Transcriber is a desktop application that transcribes audio to text using OpenAI's Whisper models via the `faster-whisper` library. It runs fully offline — models are downloaded once and stored locally in a `models/` directory alongside the application.

The application has two interfaces:
- **GUI** — built with [CustomTkinter](https://github.com/TomSchimansky/CustomTkinter) (a tkinter wrapper with modern theming)
- **CLI** — built with `argparse`

Two modes of transcription:
- **File mode** — select an audio file, transcribe it (chunked at 3-minute intervals for long files)
- **Recording mode** — capture from the microphone via `sounddevice`, transcribe in 2-second sliding windows

English only. Two models: `small.en` (Accurate, 464 MB) and `base.en` (Fast, 141 MB).

---

## Project Structure

```
rachaels-transcriber/
├── engine.py              # Core transcription logic
├── gui.py                 # CustomTkinter GUI application
├── transcribe.py          # CLI entry point
├── recorder.py            # Microphone audio capture
├── download_models.py     # Model download utility
├── theme.json             # Complete CustomTkinter theme
├── requirements.txt       # Python dependencies
├── setup_mac.sh           # macOS virtualenv + install
├── build_mac.sh           # macOS PyInstaller bundle
├── build_win.bat          # Windows build (cmd)
├── build_win.ps1          # Windows build (PowerShell)
├── run.sh                 # macOS launcher
├── run.bat                # Windows launcher
├── models/                # Downloaded model snapshots
├── test_audio/            # Sample audio for testing
└── docs/
    └── developer-guide.md # This file
```

---

## Module Reference

### `engine.py` — Transcription Engine

**Constants:**

| Constant | Value | Description |
|---|---|---|
| `LANGUAGES` | `{"English": "en"}` | Single-language map (English only) |
| `MODELS` | `["small.en", "base.en"]` | Available model names (match faster-whisper HuggingFace repo names) |
| `CHUNK_OVERLAP` | `2` | Seconds of overlap between adjacent chunks (prevents word-splitting) |
| `FORMATTERS` | `{"txt": format_as_txt}` | Single format (txt-only output) |

**`format_as_txt(result)`** — Extracts `result["text"]` from a transcription result dict. The format registry is a dict for extensibility but only `txt` is used.

**`IncrementalFileWriter`** — Context manager that writes segments to a `.txt` file incrementally as they arrive. Used by both chunked file transcription and recording. Methods:
- `__init__(path)` — Opens `path` for writing on `__enter__`.
- `write_segments(segments, offset=0)` — Appends text from each segment (with optional timestamp offset) to the file. Each segment's text goes on its own line.
- `__enter__` / `__exit__` — Standard context manager for file lifecycle. Note: used programmatically (`.__enter__()` / `.__exit__()` calls in `transcribe_chunked`) rather than `with` blocks because the writer is conditionally created.

**`get_audio_duration(audio_path)`** — Uses `ffprobe` to read audio duration in seconds. Raises on failure or timeout (30s).

**`extract_chunk(audio_path, start, duration, output_path)`** — Calls `ffmpeg` to extract a segment of audio, resampled to 16 kHz mono 16-bit WAV. Raises on failure.

**`deduplicate_segments(new_segments, prev_chunk_end)`** — Filters out segments whose `end` time is ≤ `prev_chunk_end`. Used to discard the overlap region from the previous chunk.

**`TranscriptionEngine`** — Main engine class.

**Model caching:** `_models` dict caches loaded models by name. `_get_model(model_name, device)` loads the model if not cached. `_get_model_path(model_name)` resolves the local `models/` directory — checks `sys._MEIPASS` (PyInstaller bundle) first, falls back to `Path(__file__).parent`.

**`transcribe(audio_path, model_name, language, vad, word_timestamps, device, progress_callback, cancel_event, output_path)`** — Transcribes a single audio file in one pass. Streams segments from faster-whisper via a generator. Emits progress callbacks at each segment. Optionally writes incrementally to `output_path` via `IncrementalFileWriter`. Returns a result dict with `text`, `segments`, `detected_language`, `language_probability`, `audio_duration`, `model`.

Progress callback format:
```python
{"status": "loading"|"transcribing"|"cancelled"|"done",
 "message": str,           # loading only
 "progress": float,        # transcribing only, 0.0–1.0
 "text": str,              # transcribing only, cumulative
 "result": dict,           # cancelled/done only, full result
}
```

**`transcribe_chunked(audio_path, ...)`** — Splits long audio into fixed-size chunks (default 3 min = `chunk_minutes`) with 2s overlap, transcribes each sequentially, deduplicates overlap, and concatenates results. Uses `tempfile.mkstemp` for chunk WAV files. Falls through to `transcribe()` if the file fits in a single chunk. Emits additional `chunk_start` statuses with `chunk_index` and `chunk_total`.

**`transcribe_buffer(audio, sample_rate=16000, ...)`** — Transcribes raw PCM float32 audio (numpy array), such as from microphone capture. Does not support streaming/progress callbacks. Returns same result format (without overlap dedup).

---

### `recorder.py` — Audio Capture

**Constants:**

| Constant | Value | Description |
|---|---|---|
| `SAMPLE_RATE` | `16000` | Audio sample rate (Hz) |
| `CHANNELS` | `1` | Mono capture |
| `DTYPE` | `"int16"` | Sounddevice data type |
| `FRAMES_PER_BUFFER` | `1600` (~100ms) | Sounddevice block size |
| `WINDOW_SEC` | `4` | Sliding window size for transcription |
| `TICK_SEC` | `2` | Interval between transcription calls |
| `OVERLAP_SEC` | `2` | `WINDOW_SEC - TICK_SEC` — overlap between consecutive windows |
| `MAX_BUFFER_SEC` | `30` | Maximum ring buffer size (30 seconds of audio kept) |

**`AudioRecorder`** — Captures microphone audio via `sounddevice.InputStream`.

**Thread safety:** All buffer access is protected by `self._lock` (threading.Lock).

**`_init_vad()`** — Attempts to import `webrtcvad`. Falls back to `None` if unavailable (VAD is effectively disabled — `has_speech` returns `True`).

**`start(wav_path=None)`** — Clears the ring buffer, optionally opens a WAV file for simultaneous writing (`wave.open` with 16-bit mono 16 kHz), creates and starts the InputStream.

**`stop()`** — Stops and closes the InputStream, closes the WAV file if open, clears state.

**`elapsed`** — Property returning seconds since `start()` was called.

**`_callback(indata, frames, time_info, status)`** — Sounddevice callback. Extracts the first channel, appends to the ring buffer, and optionally writes to the WAV file.

**`get_buffer(seconds=WINDOW_SEC)`** — Returns the last `N` seconds of audio as a `numpy.ndarray` of `float32` normalized to [-1, 1] (converted from int16). Returns `None` if the buffer doesn't have enough data yet.

**`has_speech(audio, threshold=0.2)`** — Runs webrtcvad on 30ms frames. Returns `True` if ≥20% of frames contain speech. Always returns `True` if webrtcvad is unavailable.

---

### `transcribe.py` — CLI

**Entry points:** `main()` -> `_run_recording_cli(args)` (if `--record`) or file transcription.

**File mode:**
- Resolves input/output paths
- Creates `TranscriptionEngine`
- Calls `transcribe_chunked()` (always chunked, default 3 min) with a `progress_callback` that writes progress to stderr
- Writes final result to the output path
- Prints stats (elapsed time, speed ratio, language)

**Recording mode (`_run_recording_cli`):**
- Creates `AudioRecorder` and starts it with a WAV path
- Loops every `TICK_SEC` (2s):
  - Gets a 4-second window via `recorder.get_buffer()`
  - Skips if silent (via `has_speech`)
  - Calls `engine.transcribe_buffer()` on the audio
  - Deduplicates against the previous window using `OVERLAP_SEC`
  - Writes the full accumulated text to the output file
- On Ctrl+C, stops the recorder and prints final stats

**`_fmt_elapsed(seconds)`** — Formats as `H:MM:SS` or `M:SS`.

---

### `gui.py` — CustomTkinter GUI (879 lines)

#### Architecture

The GUI is a single `TranscriberApp` class extending `ctk.CTk`. It uses a **view-switching** pattern with three frames stacked in `self.content_area`:
- `landing_frame` — Two cards (File Transcription / Live Recording)
- `file_frame` — Sidebar + text output for file transcription
- `record_frame` — Sidebar + text output for live recording

Only one view is visible at a time via `_show_view()`.

**Threading model:** Transcription and recording run in daemon threads. Progress updates are sent to the main thread via `queue.Queue`, polled by `_poll_queue()` every 100ms.

#### Key State Variables

| Variable | Type | Purpose |
|---|---|---|
| `self._current_view` | `str` | `"landing"`, `"file"`, or `"record"` |
| `self.running` | `bool` | File transcription in progress |
| `self.cancel_event` | `threading.Event` | Signal to cancel file transcription |
| `self.recording` | `bool` | Live recording in progress |
| `self.record_stop` | `threading.Event` | Signal to stop recording |
| `self.recorder` | `AudioRecorder` or `None` | Active recorder instance |
| `self.recording_text` | `str` | Accumulated transcript text |
| `self.latest_result` | `dict` or `None` | Most recent transcription result |
| `self.queue` | `queue.Queue` | Thread→GUI communication channel |

#### UI Layout

**Landing page:**
- Two cards (`CTkFrame`, 280×210) centered in a `CTkFrame` with `pack(expand=True)`
- File card has a blue (#007AFF) icon frame with a grid symbol (▣); Record card has a red (#b33a3a) icon frame with a record symbol (◉)
- Hover darkens the card background (no border highlight)
- Click binding propagates to all child widgets via recursive `bind("<Button-1>")`

**File sidebar (260px wide, `pack(side="left", fill="y")`):**
- Back button (transparent, text-only, `fg_color="transparent"`)
- Mode title ("File Transcription")
- Separator (1px `CTkFrame`)
- "File" label + row `[CTkEntry ← "Choose" button]`
- "Save as" label + row `[CTkEntry ← "Choose" button]`
- Separator
- "Model" label + `CTkSegmentedButton` ["Accurate", "Fast"]
- Separator (thinner: `pady=(0, 10)`)
- `CTkSwitch` for "VAD (skip silence)"
- `CTkSwitch` for "Word timestamps"
- Separator
- "Transcribe" button (green `#2b7a3e`, turns red `#b33a3a` during transcription, disabled grey during cancel)

**Record sidebar:**
- Same back button + title structure
- "Save audio as" + `[CTkEntry ← "Choose" button]`
- "Save transcript as" + `[CTkEntry ← "Choose" button]`
- Separator
- "Model" label + `CTkSegmentedButton`
- Separator (thinner)
- "VAD" `CTkSwitch`
- Separator
- "Record" button (green `#2b7a3e`, turns red `#b33a3a` while recording, grey while finalising)
- Record indicator text (red, shows elapsed time)

**Text area** — Native `tk.Text` with auto-hiding `tk.Scrollbar`:
- Wrapped in a `CTkFrame` with `fg_color="transparent"`
- Scrollbar only shows when content exceeds the viewport (custom `auto_scroll` function calls `pack_forget()` / `pack()`)
- Bottom `tk.Label` serves as a status bar (filename, duration, model info, chunk progress)

**Progress bar** — 3px `CTkProgressBar` at the absolute bottom of `self.content_area`, hidden on the landing page, visible on file/record views.

#### Queue-Based Status Handling (`_poll_queue`)

The `_poll_queue` method runs every 100ms via `self.after(100, self._poll_queue)`. It processes status dicts sent from background threads:

| Status | Handler Actions |
|---|---|
| `"loading"` | Update status bar with message |
| `"chunk_start"` | Update status bar with chunk progress |
| `"transcribing"` | Update progress bar, replace text content, update status bar |
| `"done"` | Display final text, set progress to 100%, reset transcribe button |
| `"cancelled"` | Show partial text with ⏹ prefix, reset button |
| `"recording_start"` | Initialize indicator timer, update status bar |
| `"recording_transcript"` | Update text content in real-time, update elapsed indicator |
| `"recording_stop"` | Display final text, reset record button |
| `"error"` | Show error in text area, show messagebox, reset appropriate button |

#### File Transcription Flow

1. User clicks **Transcribe** → `_toggle_transcription()` → `_start_transcription()`
2. Validates file exists, sets `self.running=True`
3. Switches button to red "Cancel"
4. Reads settings (model, VAD, word timestamps) from widgets
5. Spawns `_run_transcription()` in a daemon thread
6. Thread calls `engine.transcribe_chunked()` with `progress_callback` that puts statuses on the queue
7. Cancel button → `_cancel_transcription()` sets `self.cancel_event`
8. On completion/cancel/error → `_reset_transcribe_btn()` restores green state

#### Recording Flow

1. User clicks **Record** → `_toggle_recording()` → `_start_recording()`
2. Sets `self.recording=True`, resolves paths (defaults to `~/recording_TIMESTAMP.wav/.txt`)
3. Switches button to red "Stop"
4. Spawns `_run_recording()` in a daemon thread
5. Thread creates `AudioRecorder`, starts capture
6. Loops every `TICK_SEC` (2s):
   - Gets 4s audio window
   - Checks VAD (skips if silent)
   - Calls `engine.transcribe_buffer()`
   - Deduplicates via `OVERLAP_SEC`
   - Writes accumulated text to transcript file
   - Sends transcript update to queue
7. Stop button → `_stop_recording()` sets `self.record_stop`; thread finishes current window, stops recorder, writes final file
8. Completion → `_reset_record_btn()` restores green state

---

### `download_models.py` — Model Download

Downloads Whisper model snapshots from HuggingFace Hub to the `models/` directory.

- `MODELS` dict maps internal names (`"small.en"`, `"base.en"`) to HuggingFace repo IDs (`"Systran/faster-whisper-small.en"`, etc.)
- `download(name)` uses `huggingface_hub.snapshot_download` to save to `models/<name>/`
- Skips download if the target directory already exists and is non-empty
- CLI flags `--accurate` and `--fast` select which models to download; both are downloaded by default

---

## Theming

`theme.json` is a complete CustomTkinter theme loaded via `ctk.set_default_color_theme("theme.json")`. Key design decisions:

- **macOS blue accent** — `#007AFF` (light) / `#0A84FF` (dark) for primary CTkButton background, switch progress, slider button, segmented button selected
- **6px corner radius** on CTkButton, CTkSegmentedButton
- **8px corner radius** on CTkFrame, CTkEntry, CTkOptionMenu, CTkTextbox
- **Auto-hiding scrollbar** is handled in code (tk.Scrollbar pack/unpack), not in the theme
- **No hand cursor** — `"cursor": ""` on CTkButton, CTkOptionMenu, CTkSlider
- All required theme sections are present because CustomTkinter replaces the entire theme on load (not a deep merge)

---

## Threading & Concurrency

| Thread | Purpose | Created by | Communicates via |
|---|---|---|---|
| File transcription | Runs `engine.transcribe_chunked()` | `_start_transcription()` | `queue.Queue` |
| Recording | Runs `recorder.start()` + transcription loop | `_start_recording()` | `queue.Queue` |

**Important:** The GUI must never call blocking operations on the main thread. Background threads must only touch tkinter widgets indirectly through the queue.

**Cancel vs Stop:**
- File transcription: `cancel_event` is checked between segments and between chunks. The current segment finishes, then the loop breaks.
- Recording: `record_stop` is used as the `wait()` timeout on the tick interval. When set, the loop exits immediately.

---

## Path Resolution

The application needs to find the `models/` directory containing downloaded Whisper snapshots. Two paths are tried in `engine.py:_get_model_path()`:

```python
base = Path(getattr(sys, "_MEIPASS", Path(__file__).parent))
local = base / "models" / model_name
```

- `sys._MEIPASS` is set by PyInstaller at runtime to the extraction directory of the bundled executable
- `Path(__file__).parent` is used when running from source

This allows models to be bundled inside the PyInstaller `--onefile` executable or placed beside the script in development.

---

## Dependencies

| Package | Version | Purpose |
|---|---|---|
| `faster-whisper` | ≥1.0.0 | Whisper model inference via CTranslate2 |
| `customtkinter` | ≥5.2.0 | Modern tkinter wrapper |
| `Pillow` | ≥9.0.0 | Image support for CustomTkinter |
| `sounddevice` | ≥0.5.0 | Microphone capture |
| `webrtcvad` | ≥2.0.10 | Voice activity detection (optional — graceful fallback) |
| `pydub` | ≥0.25.1 | Audio format conversion |

System dependency: **ffmpeg** (for `engine.py:extract_chunk` and `get_audio_duration`).

---

## Build & Distribution

### macOS (`build_mac.sh`)
1. Create virtualenv, install dependencies
2. Download models via `download_models.py`
3. Run PyInstaller: `--onefile --windowed gui.py`
4. Output: `dist/RachaelsTranscriber.app`

### Windows (`build_win.bat` / `build_win.ps1`)
1. Install dependencies
2. Download models via `download_models.py`
3. Run PyInstaller: `--onefile --windowed gui.py`
4. Output: `dist/RachaelsTranscriber.exe`

**Note for bundling:** Models are explicitly downloaded to `models/` before building. PyInstaller includes them via `--add-data` or automatic collection. The `_get_model_path()` fallback checks `sys._MEIPASS` first.

---

## Extension Points

### Adding a new model variant

1. Add the config to `MODELS` in `engine.py`
2. Add to `MODEL_LABELS` in `gui.py`
3. Add to `MODELS` and the CLI `choices` in `transcribe.py`
4. Add to `MODELS` in `download_models.py`
5. Add to `MODEL_LABELS` in `gui.py` (for the `model_var` -> model name mapping)

### Adding a new output format

1. Create a formatter function like `format_as_txt()` in `engine.py`
2. Add it to the `FORMATTERS` dict
3. The `IncrementalFileWriter` currently writes plain text; add format-aware write methods if needed
4. Update the CLI output logic in `transcribe.py`
5. Add a format selector to the GUI sidebar

### Adding a new sidebar control

1. Add the widget creation inside the appropriate `_build_*_sidebar()` method
2. Add the corresponding `*_var` state variable with a descriptive name
3. Update `_set_*_settings_enabled()` to disable the control during operations
4. Read the value in `_start_transcription()` / `_start_recording()`
5. Pass it through to the relevant `engine.transcribe*()` call

### Adding a new language

- `LANGUAGES` dict in `engine.py` — add the language code
- Model must support the language (`small.en` / `base.en` are English-only; switch to multilingual variants like `small` / `base` for other languages)
- Update the UI if a language selector is added
- Update `transcribe.py` CLI help text

---

## Common Pitfalls

- **darkdetect segfault** on some macOS + Python combinations: caught by `try/except` around `ctk.set_appearance_mode("system")` → falls back to `"dark"`.
- **System Python 3.9 (macOS):** tkinter is broken on `/usr/bin/python3` (3.9.6). Must use Python 3.11+ from Homebrew or python.org.
- **ffmpeg required:** `extract_chunk()` and `get_audio_duration()` call `ffmpeg`/`ffprobe` as subprocesses. The application will crash at runtime if ffmpeg is not installed.
- **webrtcvad optional:** If not installed, `has_speech()` always returns `True` (no silence detection). No crash, but VAD becomes a no-op.
- **PyInstaller path resolution:** When bundling, `Path(__file__).parent` points to inside the bundle's extraction directory (`sys._MEIPASS`). Models must be included in the PyInstaller spec or collected automatically.
- **Threaded tkinter access:** Only the queue pattern should be used for thread→GUI communication. Direct widget access from background threads will cause crashes or data corruption.
