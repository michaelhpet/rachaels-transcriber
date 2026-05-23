#!/usr/bin/env python3
import argparse
import sys
import time
from pathlib import Path

from engine import TranscriptionEngine


def build_parser():
    parser = argparse.ArgumentParser(
        description="Transcribe audio files to text using faster-whisper."
    )
    parser.add_argument("audio", help="Path to the audio file")
    parser.add_argument("--model", default="small.en",
                        choices=["small.en", "base.en"],
                        help="Whisper model: small.en (default, more accurate) or base.en (faster)")
    parser.add_argument("--language", default="en",
                        help="Language code (default: en).")
    parser.add_argument("--output", "-o", default=None,
                        help="Output file path (default: <audio>.txt)")
    parser.add_argument("--vad", action="store_true",
                        help="Enable voice activity detection to filter silence")
    parser.add_argument("--word-timestamps", action="store_true",
                        help="Include word-level timestamps in output")
    parser.add_argument("--device", default="auto",
                        choices=["auto", "cpu", "cuda"],
                        help="Compute device (default: auto)")
    parser.add_argument("--chunk-minutes", type=int, default=3,
                        help="Split audio into N-minute chunks for long files")
    parser.add_argument("--record", action="store_true",
                        help="Record from microphone instead of transcribing a file")
    return parser


def main():
    parser = build_parser()
    args = parser.parse_args()

    if args.record:
        _run_recording_cli(args)
        return

    audio_path = Path(args.audio)
    if not audio_path.exists():
        print(f"Error: file not found: {audio_path}", file=sys.stderr)
        sys.exit(1)

    if args.output:
        output_path = Path(args.output)
    else:
        output_path = audio_path.with_suffix(".txt")

    engine = TranscriptionEngine()
    start = time.time()

    def progress(status):
        if status["status"] == "loading":
            print(f"  {status.get('message', 'Loading...')}", file=sys.stderr)
        elif status["status"] == "chunk_start":
            print(f"\r  {status.get('message', '')}   ", file=sys.stderr)
        elif status["status"] == "transcribing":
            pct = int(status["progress"] * 100)
            chunk_i = status.get("chunk_index", -1)
            chunk_n = status.get("chunk_total", 0)
            if chunk_n > 0:
                print(f"\r  Chunk {chunk_i + 1}/{chunk_n} — {pct}%",
                      file=sys.stderr, end="", flush=True)
            else:
                print(f"\r  Progress: {pct}%", file=sys.stderr, end="", flush=True)

    try:
        kw = dict(
            audio_path=str(audio_path),
            model_name=args.model,
            language=args.language or None,
            vad=args.vad,
            word_timestamps=args.word_timestamps,
            device=args.device,
            progress_callback=progress,
            output_path=str(output_path),
        )

        if args.chunk_minutes > 0:
            result = engine.transcribe_chunked(**kw, chunk_minutes=args.chunk_minutes)
        else:
            result = engine.transcribe(**kw)

        print(file=sys.stderr)

        output_path.write_text(result["text"], encoding="utf-8")

        elapsed = time.time() - start
        dur = result.get("audio_duration", 0)
        lang = result.get("detected_language", "?")
        ratio = dur / elapsed if elapsed > 0 else 0
        print(f"  Done in {elapsed:.1f}s ({ratio:.1f}x real-time)", file=sys.stderr)
        print(f"  Language: {lang}  |  Audio duration: {dur:.1f}s", file=sys.stderr)
        print(f"  Saved to: {output_path}")

    except Exception as e:
        print(f"\nError: {e}", file=sys.stderr)
        sys.exit(1)


def _run_recording_cli(args):
    from recorder import AudioRecorder, WINDOW_SEC, TICK_SEC, OVERLAP_SEC
    from datetime import datetime

    engine = TranscriptionEngine()
    output_path = Path(args.output or f"recording_{datetime.now():%Y%m%d_%H%M%S}.txt")
    wav_path = output_path.with_suffix(".wav")

    print(f"  Recording to {wav_path}", file=sys.stderr)
    print(f"  Press Ctrl+C to stop", file=sys.stderr)

    recorder = AudioRecorder()
    recorder.start(wav_path=str(wav_path))

    full_text = ""
    first_window = True
    start = time.time()

    try:
        while True:
            if recorder.elapsed < WINDOW_SEC:
                time.sleep(0.5)
                continue

            audio = recorder.get_buffer(WINDOW_SEC)
            if audio is None:
                time.sleep(0.5)
                continue
            if not recorder.has_speech(audio):
                print(f"\r  {_fmt_elapsed(recorder.elapsed)}  (silent)",
                      file=sys.stderr, end="", flush=True)
                time.sleep(TICK_SEC)
                continue

            result = engine.transcribe_buffer(
                audio, model_name=args.model, language=args.language or None,
                vad=args.vad,
            )

            if first_window:
                new_segs = result["segments"]
                first_window = False
            else:
                new_segs = [s for s in result["segments"]
                            if s["end"] > OVERLAP_SEC]

            if new_segs:
                new_text = " ".join(s["text"] for s in new_segs)
                full_text = (full_text + " " + new_text).strip()
                output_path.write_text(full_text, encoding="utf-8")

            print(f"\r  {_fmt_elapsed(recorder.elapsed)}  {len(full_text)} chars",
                  file=sys.stderr, end="", flush=True)

            time.sleep(TICK_SEC)

    except KeyboardInterrupt:
        pass
    finally:
        recorder.stop()

    elapsed = time.time() - start
    print(file=sys.stderr)
    print(f"  Recorded {_fmt_elapsed(recorder.elapsed)} in {elapsed:.1f}s", file=sys.stderr)
    print(f"  Transcript: {output_path}", file=sys.stderr)
    print(f"  Audio: {wav_path}", file=sys.stderr)
    if full_text:
        print(f"\n{full_text}")


def _fmt_elapsed(seconds):
    m, s = divmod(int(seconds), 60)
    h, m = divmod(m, 60)
    if h:
        return f"{h}:{m:02d}:{s:02d}"
    return f"{m}:{s:02d}"


if __name__ == "__main__":
    main()
