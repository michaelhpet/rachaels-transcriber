import threading
import time
from collections import deque
from pathlib import Path

import numpy as np

SAMPLE_RATE = 16000
CHANNELS = 1
DTYPE = "int16"
FRAMES_PER_BUFFER = int(SAMPLE_RATE * 0.1)  # 100ms

WINDOW_SEC = 4
TICK_SEC = 2
OVERLAP_SEC = WINDOW_SEC - TICK_SEC  # 2s overlap
MAX_BUFFER_SEC = 30


class AudioRecorder:
    def __init__(self):
        self.buffer = deque(maxlen=SAMPLE_RATE * MAX_BUFFER_SEC)
        self.recording = False
        self.stream = None
        self.wav_file = None
        self.start_time = None
        self._lock = threading.Lock()
        self._vad = None
        self._init_vad()

    def _init_vad(self):
        try:
            import webrtcvad
            self._vad = webrtcvad.Vad(2)
        except ImportError:
            self._vad = None

    def start(self, wav_path=None):
        if self.recording:
            return
        try:
            import sounddevice as sd
        except Exception as e:
            raise RuntimeError(
                "Audio recording is not available on this system.\n"
                f"sounddevice/PortAudio failed to load: {e}"
            )
        self.buffer.clear()
        self.recording = True
        self.start_time = time.time()

        if wav_path:
            import wave
            self.wav_file = wave.open(str(wav_path), "wb")
            self.wav_file.setnchannels(CHANNELS)
            self.wav_file.setsampwidth(2)
            self.wav_file.setframerate(SAMPLE_RATE)

        self.stream = sd.InputStream(
            samplerate=SAMPLE_RATE,
            channels=CHANNELS,
            dtype=DTYPE,
            callback=self._callback,
            blocksize=FRAMES_PER_BUFFER,
        )
        self.stream.start()

    def stop(self):
        if not self.recording:
            return
        self.recording = False
        self.start_time = None
        if self.stream:
            self.stream.stop()
            self.stream.close()
            self.stream = None
        if self.wav_file:
            self.wav_file.close()
            self.wav_file = None

    @property
    def elapsed(self):
        if self.start_time is None:
            return 0.0
        return time.time() - self.start_time

    def _callback(self, indata, frames, time_info, status):
        if not self.recording:
            return
        mono = indata[:, 0]
        with self._lock:
            self.buffer.extend(mono.tolist())
            if self.wav_file:
                self.wav_file.writeframes(mono.tobytes())

    def get_buffer(self, seconds=WINDOW_SEC):
        n = int(SAMPLE_RATE * seconds)
        with self._lock:
            if len(self.buffer) < n:
                return None
            data = list(self.buffer)[-n:]
        arr = np.array(data, dtype=np.float32) / 32768.0
        return arr

    def has_speech(self, audio, threshold=0.2):
        if self._vad is None:
            return True
        int16 = (audio * 32768).astype(np.int16)
        frame_len = int(SAMPLE_RATE * 0.03)
        speech_frames = 0
        total = 0
        for i in range(0, len(int16) - frame_len + 1, frame_len):
            frame = int16[i:i + frame_len].tobytes()
            if self._vad.is_speech(frame, SAMPLE_RATE):
                speech_frames += 1
            total += 1
        return (speech_frames / total) >= threshold if total > 0 else False
