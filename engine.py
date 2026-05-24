import os
import subprocess
import sys
import tempfile
from pathlib import Path

import faster_whisper

LANGUAGES = {
    "English": "en",
}

MODELS = ["small.en", "base.en"]
CHUNK_OVERLAP = 2


def get_persistent_base():
    if getattr(sys, "frozen", False):
        return Path(sys.executable).parent
    return Path(__file__).parent


def format_as_txt(result):
    return result["text"]


FORMATTERS = {"txt": format_as_txt}


class IncrementalFileWriter:
    def __init__(self, path):
        self.path = Path(path)
        self.file = None

    def __enter__(self):
        self.file = open(self.path, "w", encoding="utf-8")
        return self

    def __exit__(self, *args):
        self.file.close()

    def write_segments(self, segments, offset=0):
        for seg in segments:
            adjusted = {
                "start": seg["start"] + offset,
                "end": seg["end"] + offset,
                "text": seg["text"],
            }
            if "words" in seg:
                adjusted["words"] = [
                    {"word": w["word"], "start": w["start"] + offset, "end": w["end"] + offset}
                    for w in seg["words"]
                ]
            self.file.write(adjusted["text"] + "\n")


def get_audio_duration(audio_path):
    result = subprocess.run(
        ["ffprobe", "-v", "error", "-show_entries",
         "format=duration", "-of", "default=noprint_wrappers=1:nokey=1",
         str(audio_path)],
        capture_output=True, text=True, timeout=30,
    )
    return float(result.stdout.strip())


def extract_chunk(audio_path, start, duration, output_path):
    subprocess.run(
        ["ffmpeg", "-y", "-ss", str(start), "-t", str(duration),
         "-i", str(audio_path), "-ar", "16000", "-ac", "1",
         "-sample_fmt", "s16", str(output_path)],
        capture_output=True, timeout=300,
    )


def deduplicate_segments(new_segments, prev_chunk_end):
    kept = []
    for seg in new_segments:
        if seg["end"] > prev_chunk_end:
            kept.append(seg)
    return kept


class TranscriptionEngine:
    def __init__(self):
        self._models = {}

    def _get_model_path(self, model_name):
        candidates = []
        candidates.append(get_persistent_base() / "models" / model_name)
        if hasattr(sys, "_MEIPASS"):
            candidates.append(Path(sys._MEIPASS) / "models" / model_name)
        for p in candidates:
            if p.is_dir():
                return str(p)
        return model_name

    def _get_model(self, model_name, device="auto"):
        if model_name not in self._models:
            model_path = self._get_model_path(model_name)
            self._models[model_name] = faster_whisper.WhisperModel(
                model_path, device=device, compute_type="int8"
            )
        return self._models[model_name]

    def transcribe(
        self,
        audio_path,
        model_name="base",
        language=None,
        vad=False,
        word_timestamps=False,
        device="auto",
        progress_callback=None,
        cancel_event=None,
        output_path=None,
    ):
        def _emit(status):
            if progress_callback:
                progress_callback(status)

        _emit({"status": "loading", "message": f"Loading model '{model_name}'..."})
        model = self._get_model(model_name, device=device)

        _emit({"status": "loading", "message": "Processing audio..."})

        segments, info = model.transcribe(
            audio_path,
            language=language,
            vad_filter=vad,
            word_timestamps=word_timestamps,
            beam_size=5,
        )

        audio_duration = info.duration
        detected_language = info.language
        language_probability = info.language_probability

        writer = None
        if output_path:
            writer = IncrementalFileWriter(output_path)
            writer.__enter__()

        full_text = ""
        all_segments = []
        was_cancelled = False

        try:
            for segment in segments:
                if cancel_event and cancel_event.is_set():
                    was_cancelled = True
                    break

                seg_data = {
                    "start": segment.start,
                    "end": segment.end,
                    "text": segment.text.strip(),
                }

                if word_timestamps and segment.words:
                    seg_data["words"] = [
                        {"word": w.word, "start": w.start, "end": w.end}
                        for w in segment.words
                    ]

                all_segments.append(seg_data)
                full_text += segment.text + " "

                if writer:
                    writer.write_segments([seg_data])

                progress = segment.end / audio_duration if audio_duration > 0 else 0

                _emit({
                    "status": "transcribing",
                    "progress": min(progress, 0.99),
                    "text": full_text.strip(),
                })
        finally:
            if writer:
                writer.__exit__()

        result = {
            "text": full_text.strip(),
            "segments": all_segments,
            "detected_language": detected_language,
            "language_probability": language_probability,
            "audio_duration": audio_duration,
            "model": model_name,
        }

        if was_cancelled:
            _emit({"status": "cancelled", "result": result})
        else:
            _emit({"status": "done", "result": result})
        return result

    def transcribe_chunked(
        self,
        audio_path,
        model_name="base",
        language=None,
        vad=False,
        word_timestamps=False,
        device="auto",
        progress_callback=None,
        cancel_event=None,
        chunk_minutes=5,
        output_path=None,
    ):
        def _emit(status):
            if progress_callback:
                progress_callback(status)

        if chunk_minutes <= 0:
            raise ValueError("chunk_minutes must be > 0")

        duration = get_audio_duration(audio_path)

        if duration <= chunk_minutes * 60:
            return self.transcribe(
                audio_path=audio_path,
                model_name=model_name,
                language=language,
                vad=vad,
                word_timestamps=word_timestamps,
                device=device,
                progress_callback=progress_callback,
                cancel_event=cancel_event,
                output_path=output_path,
            )

        _emit({"status": "loading",
               "message": f"Loading model '{model_name}'..."})
        model = self._get_model(model_name, device=device)

        chunk_sec = chunk_minutes * 60
        chunk_count = int((duration + chunk_sec - 1) // chunk_sec)
        total_segments = []
        full_text = ""
        detected_language = "?"
        language_probability = 0
        was_cancelled = False
        prev_chunk_end = 0

        writer = None
        if output_path:
            writer = IncrementalFileWriter(output_path)

        if writer:
            writer.__enter__()

        try:
            for chunk_idx in range(chunk_count):
                if cancel_event and cancel_event.is_set():
                    was_cancelled = True
                    break

                chunk_start = chunk_idx * chunk_sec
                chunk_dur = min(chunk_sec + CHUNK_OVERLAP,
                                duration - chunk_start)

                _emit({
                    "status": "chunk_start",
                    "chunk_index": chunk_idx,
                    "chunk_total": chunk_count,
                    "message": f"Chunk {chunk_idx + 1}/{chunk_count}",
                })

                fd, chunk_path = tempfile.mkstemp(suffix=".wav")
                os.close(fd)
                try:
                    extract_chunk(audio_path, chunk_start, chunk_dur,
                                  chunk_path)

                    segments, info = model.transcribe(
                        chunk_path,
                        language=language,
                        vad_filter=vad,
                        word_timestamps=word_timestamps,
                        beam_size=5,
                    )

                    if chunk_idx == 0:
                        detected_language = info.language
                        language_probability = info.language_probability

                    chunk_segments = []
                    chunk_text = ""

                    for segment in segments:
                        if cancel_event and cancel_event.is_set():
                            was_cancelled = True
                            break

                        seg_data = {
                            "start": segment.start + chunk_start,
                            "end": segment.end + chunk_start,
                            "text": segment.text.strip(),
                        }

                        if word_timestamps and segment.words:
                            seg_data["words"] = [
                                {
                                    "word": w.word,
                                    "start": w.start + chunk_start,
                                    "end": w.end + chunk_start,
                                }
                                for w in segment.words
                            ]

                        chunk_segments.append(seg_data)
                        chunk_text += segment.text + " "

                    if was_cancelled:
                        break

                    chunk_segments = deduplicate_segments(
                        chunk_segments, prev_chunk_end
                    )

                    total_segments.extend(chunk_segments)
                    full_text += chunk_text

                    if writer:
                        writer.write_segments(chunk_segments)

                    overall_progress = min(
                        (chunk_idx + 1) / chunk_count, 0.99
                    ) if chunk_count > 0 else 1

                    _emit({
                        "status": "transcribing",
                        "progress": min(overall_progress, 0.99),
                        "text": full_text.strip(),
                        "chunk_index": chunk_idx,
                        "chunk_total": chunk_count,
                    })

                finally:
                    if os.path.exists(chunk_path):
                        os.unlink(chunk_path)

                prev_chunk_end = chunk_start + chunk_sec

        finally:
            if writer:
                writer.__exit__()

        result = {
            "text": full_text.strip(),
            "segments": total_segments,
            "detected_language": detected_language,
            "language_probability": language_probability,
            "audio_duration": duration,
            "model": model_name,
        }

        if was_cancelled:
            _emit({"status": "cancelled", "result": result})
        else:
            _emit({"status": "done", "result": result})
        return result

    def transcribe_buffer(
        self,
        audio,
        sample_rate=16000,
        model_name="tiny",
        language=None,
        vad=False,
        word_timestamps=False,
        device="auto",
        cancel_event=None,
    ):
        model = self._get_model(model_name, device=device)

        segments, info = model.transcribe(
            audio,
            language=language,
            vad_filter=vad,
            word_timestamps=word_timestamps,
            beam_size=5,
        )

        audio_duration = len(audio) / sample_rate
        segments_list = []
        text = ""

        for s in segments:
            if cancel_event and cancel_event.is_set():
                break

            seg = {
                "start": s.start,
                "end": s.end,
                "text": s.text.strip(),
            }
            if word_timestamps and s.words:
                seg["words"] = [
                    {"word": w.word, "start": w.start, "end": w.end}
                    for w in s.words
                ]

            segments_list.append(seg)
            text += s.text + " "

        return {
            "text": text.strip(),
            "segments": segments_list,
            "detected_language": info.language,
            "language_probability": info.language_probability,
            "audio_duration": audio_duration,
            "model": model_name,
        }
