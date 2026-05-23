import queue
import sys
import threading
import time
from datetime import datetime
from pathlib import Path

import customtkinter as ctk
import tkinter as tk
from tkinter import filedialog, messagebox

from engine import TranscriptionEngine, MODELS


class _StreamRedirector:
    """Redirects stdout/stderr to a tkinter Text widget from a thread."""
    def __init__(self, text_widget):
        self.text = text_widget

    def write(self, msg):
        self.text.after(0, lambda m=msg: (
            self.text.insert("end", m),
            self.text.see("end"),
        ))

    def flush(self):
        pass

MODEL_LABELS = {"Accurate": "small.en", "Fast": "base.en"}
from recorder import AudioRecorder, OVERLAP_SEC, WINDOW_SEC, TICK_SEC

try:
    ctk.set_appearance_mode("system")
except Exception:
    ctk.set_appearance_mode("dark")
ctk.set_default_color_theme("theme.json")


class TranscriberApp(ctk.CTk):
    SIDEBAR_WIDTH = 260

    def __init__(self):
        super().__init__()
        self.title("Rachael's Transcriber")
        self._set_window_icon()
        self._check_and_download_models()
        self.geometry("1000x700")
        self.minsize(800, 500)

        self.engine = TranscriptionEngine()
        self.queue = queue.Queue()
        self.latest_result = None
        self._current_view = None

        # File transcription state
        self.running = False
        self.cancel_event = threading.Event()

        # Recording state
        self.recording = False
        self.record_stop = threading.Event()
        self.recorder = None
        self.recording_wav_path = None
        self.recording_text = ""

        self._build_ui()
        self._poll_queue()

    def _set_window_icon(self):
        base = Path(getattr(sys, "_MEIPASS", Path(__file__).parent))
        for name in ("icon.png", "icon.ico"):
            p = base / "assets" / name
            if p.exists():
                try:
                    icon = tk.PhotoImage(file=str(p))
                    self.iconphoto(True, icon)
                except Exception:
                    pass
                return

    def _check_and_download_models(self):
        base = Path(getattr(sys, "_MEIPASS", Path(__file__).parent))
        models_dir = base / "models"
        missing = [m for m in MODELS if not (models_dir / m).is_dir()]
        if not missing:
            return

        self.withdraw()

        dialog = ctk.CTkToplevel(self)
        dialog.title("Setting up")
        dialog.geometry("560x420")
        dialog.transient(self)
        dialog.grab_set()
        dialog.resizable(False, False)

        ctk.CTkLabel(dialog, text="Downloading speech models",
                     font=("", 15, "bold")).pack(pady=(20, 4))
        ctk.CTkLabel(dialog,
                     text="~600 MB download. Internet required.",
                     font=("", 11)).pack(pady=(0, 12))

        text_frame = ctk.CTkFrame(dialog)
        text_frame.pack(fill="both", expand=True, padx=16, pady=(0, 12))

        text = tk.Text(text_frame, wrap="word", font=("Consolas", 10),
                       height=14, padx=8, pady=8)
        text.pack(side="left", fill="both", expand=True)

        scrollbar = tk.Scrollbar(text_frame, orient="vertical",
                                 command=text.yview)
        scrollbar.pack(side="right", fill="y")
        text.configure(yscrollcommand=scrollbar.set)

        cancel_btn = ctk.CTkButton(dialog, text="Cancel",
                                   command=dialog.destroy)
        cancel_btn.pack(pady=(0, 16))

        failed = []

        def run():
            try:
                from download_models import download as dl
                for model in missing:
                    text.after(0, lambda m=model: text.insert(
                        "end", f"[{m}] Downloading...\n"))
                    old_out = sys.stdout
                    old_err = sys.stderr
                    sys.stdout = _StreamRedirector(text)
                    sys.stderr = _StreamRedirector(text)
                    try:
                        dl(model)
                    finally:
                        sys.stdout = old_out
                        sys.stderr = old_err
            except Exception as e:
                failed.append(e)
            finally:
                if not failed:
                    dialog.after(0, dialog.destroy)

        thread = threading.Thread(target=run, daemon=True)
        thread.start()

        self.wait_window(dialog)
        self.deiconify()

        if failed:
            retry = messagebox.askretrycancel(
                "Download Failed",
                "Failed to download speech models.\n\n"
                "Check your internet connection and try again."
            )
            if retry:
                self._check_and_download_models()
            else:
                self.quit()
                sys.exit(1)

    # ── Layout ────────────────────────────────────────────

    def _build_ui(self):
        self.content_area = ctk.CTkFrame(self)
        self.content_area.pack(fill="both", expand=True)
        self.content_area.pack_propagate(False)

        self.landing_frame = ctk.CTkFrame(self.content_area, fg_color="transparent")
        self.file_frame = ctk.CTkFrame(self.content_area)
        self.record_frame = ctk.CTkFrame(self.content_area)

        self._build_landing()
        self._build_file_view()
        self._build_record_view()

        self.progress_bar = ctk.CTkProgressBar(self.content_area, height=3)
        self.progress_bar.pack(fill="x", side="bottom")
        self.progress_bar.set(0)

        self._show_view("landing")

    def _build_text_area(self, parent):
        is_dark = ctk.get_appearance_mode() == "Dark"
        text_bg = "#1E1E1E" if is_dark else "#FFFFFF"
        text_fg = "#E0E0E0" if is_dark else "#000000"
        cursor = "#FFFFFF" if is_dark else "#000000"

        wrapper = ctk.CTkFrame(parent, fg_color="transparent")
        wrapper.pack(fill="both", expand=True)

        self._text_scrollbar = tk.Scrollbar(wrapper, orient="vertical", width=8)

        def auto_scroll(first, last, scrollbar):
            scrollbar.set(first, last)
            if float(last) >= 1.0:
                scrollbar.pack_forget()
            else:
                scrollbar.pack(side="right", fill="y")

        text = tk.Text(wrapper, wrap="word", font=("", 14),
                       bd=0, highlightthickness=0,
                       padx=12, pady=12,
                       bg=text_bg, fg=text_fg,
                       insertbackground=cursor,
                       yscrollcommand=lambda f, l: auto_scroll(f, l, self._text_scrollbar))
        text.pack(side="left", fill="both", expand=True)

        self.status_bar = tk.Label(wrapper, text="", anchor="w", padx=12,
                                    bg=text_bg, fg=text_fg,
                                    font=("", 11))
        self.status_bar.pack(side="bottom", fill="x")
        return text

    # ── View Switching ────────────────────────────────────

    def _show_view(self, name):
        for f in (self.landing_frame, self.file_frame, self.record_frame):
            f.pack_forget()

        frame = {"landing": self.landing_frame,
                 "file": self.file_frame,
                 "record": self.record_frame}[name]
        frame.pack(fill="both", expand=True)
        self._current_view = name

        if name == "landing":
            self.progress_bar.pack_forget()
        else:
            self.progress_bar.pack(side="bottom", fill="x")

    def _go_back(self):
        if self.running:
            return
        if self.recording:
            self._stop_recording()
        self._show_view("landing")

    # ── Landing Page ──────────────────────────────────────

    def _build_landing(self):
        f = self.landing_frame

        container = ctk.CTkFrame(f, fg_color="transparent")
        container.pack(expand=True, pady=(0, 80))

        row = ctk.CTkFrame(container, fg_color="transparent")
        row.pack()

        CARD_BG = ("#F2F2F2", "#2B2B2B")
        CARD_BD = ("#D0D0D0", "#404040")
        CARD_HOVER = ("#E0E0E0", "#222222")

        self.file_card = ctk.CTkFrame(
            row, width=280, height=210, corner_radius=16,
            fg_color=CARD_BG, border_width=1, border_color=CARD_BD,
        )
        self.file_card.pack(side="left", padx=(0, 10))
        self.file_card.pack_propagate(False)

        def file_click(e):
            self._switch_to_file()
        def file_enter(e):
            self.file_card.configure(fg_color=CARD_HOVER)
        def file_leave(e):
            x, y = self.file_card.winfo_pointerxy()
            cx, cy, cw, ch = (
                self.file_card.winfo_rootx(),
                self.file_card.winfo_rooty(),
                self.file_card.winfo_width(),
                self.file_card.winfo_height(),
            )
            if not (cx <= x < cx + cw and cy <= y < cy + ch):
                self.file_card.configure(fg_color=CARD_BG)
        self.file_card.bind("<Enter>", file_enter)
        self.file_card.bind("<Leave>", file_leave)
        self.file_card.bind("<Button-1>", file_click)

        group = ctk.CTkFrame(self.file_card, fg_color="transparent")
        group.pack(expand=True)

        icon_frame = ctk.CTkFrame(group, width=64, height=64,
                                  corner_radius=16, fg_color=("#007AFF", "#0A84FF"))
        icon_frame.pack(pady=(0, 12))
        icon_frame.pack_propagate(False)
        ctk.CTkLabel(icon_frame, text="\u25A3", font=("", 28),
                     text_color="#FFFFFF").pack(expand=True)

        ctk.CTkLabel(group, text="File Transcription",
                     font=("", 18, "bold")).pack()

        for w in self.file_card.winfo_children():
            w.bind("<Button-1>", file_click)
            for c in w.winfo_children():
                c.bind("<Button-1>", file_click)

        self.record_card = ctk.CTkFrame(
            row, width=280, height=210, corner_radius=16,
            fg_color=CARD_BG, border_width=1, border_color=CARD_BD,
        )
        self.record_card.pack(side="left", padx=(10, 0))
        self.record_card.pack_propagate(False)

        def record_click(e):
            self._switch_to_record()
        def record_enter(e):
            self.record_card.configure(fg_color=CARD_HOVER)
        def record_leave(e):
            x, y = self.record_card.winfo_pointerxy()
            cx, cy, cw, ch = (
                self.record_card.winfo_rootx(),
                self.record_card.winfo_rooty(),
                self.record_card.winfo_width(),
                self.record_card.winfo_height(),
            )
            if not (cx <= x < cx + cw and cy <= y < cy + ch):
                self.record_card.configure(fg_color=CARD_BG)
        self.record_card.bind("<Enter>", record_enter)
        self.record_card.bind("<Leave>", record_leave)
        self.record_card.bind("<Button-1>", record_click)

        group2 = ctk.CTkFrame(self.record_card, fg_color="transparent")
        group2.pack(expand=True)

        icon_frame2 = ctk.CTkFrame(group2, width=64, height=64,
                                   corner_radius=16, fg_color="#b33a3a")
        icon_frame2.pack(pady=(0, 12))
        icon_frame2.pack_propagate(False)
        ctk.CTkLabel(icon_frame2, text="\u25C9", font=("", 28),
                     text_color="#FFFFFF").pack(expand=True)

        ctk.CTkLabel(group2, text="Live Recording",
                     font=("", 18, "bold")).pack()

        for w in self.record_card.winfo_children():
            w.bind("<Button-1>", record_click)
            for c in w.winfo_children():
                c.bind("<Button-1>", record_click)

    def _switch_to_file(self):
        if self.recording:
            self._stop_recording()
        self._show_view("file")

    def _switch_to_record(self):
        if self.running:
            return
        self._show_view("record")

    # ── File View ─────────────────────────────────────────

    def _build_file_view(self):
        parent = self.file_frame
        self._build_file_sidebar(parent)
        self._build_file_main(parent)

    def _build_file_sidebar(self, parent):
        sidebar = ctk.CTkFrame(parent, width=self.SIDEBAR_WIDTH, corner_radius=0)
        sidebar.pack(side="left", fill="y")
        sidebar.pack_propagate(False)

        inner = ctk.CTkFrame(sidebar, fg_color="transparent")
        inner.pack(fill="both", expand=True, padx=12, pady=12)

        header_row = ctk.CTkFrame(inner, fg_color="transparent")
        header_row.pack(fill="x", pady=(0, 10))

        self.back_btn = ctk.CTkButton(
            header_row, text="\u2190  Back", width=60, height=24,
            command=self._go_back, fg_color="transparent",
            text_color=("gray10", "gray90"), hover_color=("#d0d0d0", "#404040"),
            font=("", 13),
        )
        self.back_btn.pack(side="left")

        ctk.CTkLabel(header_row, text="File Transcription",
                     font=("", 13, "bold")).pack(side="left", padx=(8, 0))

        ctk.CTkFrame(inner, height=1, fg_color=("gray85", "gray30")
                     ).pack(fill="x", pady=(0, 24))

        ctk.CTkLabel(inner, text="File").pack(anchor="w", pady=(0, 6))

        file_row = ctk.CTkFrame(inner, fg_color="transparent")
        file_row.pack(fill="x", pady=(0, 12))

        self.file_entry = ctk.CTkEntry(file_row, placeholder_text="No file selected",
                                       height=36)
        self.file_entry.pack(side="left", fill="x", expand=True, padx=(0, 6))

        self.browse_btn = ctk.CTkButton(file_row, text="Choose",
                                        command=self._browse_file,
                                        fg_color=("#D4D4D4", "#3A3A3A"),
                                        hover_color=("#C0C0C0", "#4A4A4A"),
                                        text_color=("gray10", "gray90"))
        self.browse_btn.pack(side="right")

        ctk.CTkFrame(inner, height=1, fg_color=("gray85", "gray30")
                     ).pack(fill="x", pady=(0, 12))

        ctk.CTkLabel(inner, text="Save as").pack(anchor="w", pady=(0, 6))

        self.save_path_row = ctk.CTkFrame(inner, fg_color="transparent")
        self.save_path_row.pack(fill="x", pady=(0, 12))

        self.save_path_entry = ctk.CTkEntry(self.save_path_row, height=36,
                                            placeholder_text="Enter location")
        self.save_path_entry.pack(side="left", fill="x", expand=True, padx=(0, 6))

        self.save_path_btn = ctk.CTkButton(self.save_path_row, text="Choose",
                                            command=self._browse_save_path,
                                            fg_color=("#D4D4D4", "#3A3A3A"),
                                            hover_color=("#C0C0C0", "#4A4A4A"),
                                            text_color=("gray10", "gray90"))
        self.save_path_btn.pack(side="right")

        ctk.CTkFrame(inner, height=1, fg_color=("gray85", "gray30")
                     ).pack(fill="x", pady=(0, 12))

        self._file_setting_label(inner, "Model")
        self.model_var = ctk.StringVar(value="Accurate")
        self.model_segment = ctk.CTkSegmentedButton(inner, variable=self.model_var,
                                                     values=list(MODEL_LABELS.keys()),
                                                     height=36)
        self.model_segment.pack(fill="x", pady=(0, 8))

        ctk.CTkFrame(inner, height=1, fg_color=("gray85", "gray30")
                     ).pack(fill="x", pady=(0, 10))

        self.vad_var = ctk.BooleanVar(value=True)
        self.vad_switch = ctk.CTkSwitch(inner, text="VAD (skip silence)",
                                        variable=self.vad_var)
        self.vad_switch.pack(anchor="w", pady=(0, 4))

        self.wts_var = ctk.BooleanVar(value=False)
        self.wts_switch = ctk.CTkSwitch(inner, text="Word timestamps",
                                        variable=self.wts_var)
        self.wts_switch.pack(anchor="w", pady=(0, 10))

        ctk.CTkFrame(inner, height=1, fg_color=("gray85", "gray30")
                     ).pack(fill="x", pady=(0, 12))

        self.transcribe_btn = ctk.CTkButton(
            inner, text="Transcribe", command=self._toggle_transcription,
            height=40, font=("", 14, "bold"), fg_color="#2b7a3e",
            hover_color="#1f5c2e",
        )
        self.transcribe_btn.pack(fill="x", pady=(0, 4))

    def _file_setting_label(self, parent, text):
        lbl = ctk.CTkLabel(parent, text=text)
        lbl.pack(anchor="w", pady=(0, 2))

    def _build_file_main(self, parent):
        main = ctk.CTkFrame(parent, corner_radius=0)
        main.pack(side="right", fill="both", expand=True)

        self.file_output_text = self._build_text_area(main)

    # ── File Helpers ──────────────────────────────────────

    def _browse_file(self):
        path = filedialog.askopenfilename(
            title="Select Audio File",
            filetypes=[
                ("Audio Files", "*.mp3 *.wav *.m4a *.flac *.ogg *.aac *.wma"),
                ("All Files", "*.*"),
            ],
        )
        if path:
            self.file_entry.delete(0, "end")
            self.file_entry.insert(0, path)
            self.status_bar.configure(text=Path(path).name)
            self._auto_populate_save_path(path)

    def _browse_save_path(self):
        path = filedialog.asksaveasfilename(
            title="Save as",
            defaultextension=".txt",
            filetypes=[("All Files", "*.*")],
        )
        if path:
            self.save_path_entry.delete(0, "end")
            self.save_path_entry.insert(0, path)

    def _auto_populate_save_path(self, audio_path):
        audio = Path(audio_path)
        save_path = audio.with_suffix(".txt")
        self.save_path_entry.delete(0, "end")
        self.save_path_entry.insert(0, str(save_path))

    # ── File Transcription ────────────────────────────────

    def _toggle_transcription(self):
        if self.running:
            self._cancel_transcription()
        else:
            self._start_transcription()

    def _set_settings_enabled(self, enabled):
        state = "normal" if enabled else "disabled"
        self.model_segment.configure(state=state)
        self.vad_switch.configure(state=state)
        self.wts_switch.configure(state=state)
        self.browse_btn.configure(state=state)
        self.file_entry.configure(state=state)
        self.save_path_entry.configure(state=state)
        self.save_path_btn.configure(state=state)

    def _start_transcription(self):
        audio_path = self.file_entry.get().strip()
        if not audio_path:
            messagebox.showwarning("No File", "Select an audio file first.")
            return
        if not Path(audio_path).exists():
            messagebox.showerror("Error", "File does not exist.")
            return

        self.running = True
        self.cancel_event = threading.Event()
        self.latest_result = None

        self.transcribe_btn.configure(
            text="Cancel", fg_color="#b33a3a", hover_color="#8a2b2b",
        )
        self.progress_bar.set(0)
        self.status_bar.configure(text="Starting...")
        self.file_output_text.delete("0.0", "end")
        self._set_settings_enabled(False)

        model = MODEL_LABELS[self.model_var.get()]
        language = "en"
        vad = self.vad_var.get()
        word_ts = self.wts_var.get()
        chunk_minutes = 3

        save_path = None
        sp = self.save_path_entry.get().strip()
        if sp:
            save_path = sp

        thread = threading.Thread(
            target=self._run_transcription,
            args=(audio_path, model, language, vad, word_ts, chunk_minutes, save_path),
            daemon=True,
        )
        thread.start()

    def _cancel_transcription(self):
        self.cancel_event.set()
        self.transcribe_btn.configure(
            text="Cancelling...", state="disabled",
            fg_color="#555555", hover_color="#555555",
        )
        self.status_bar.configure(text="Cancelling...")

    def _run_transcription(self, audio_path, model, language, vad, word_ts,
                           chunk_minutes=0, save_path=None):
        try:
            kw = dict(
                audio_path=audio_path,
                model_name=model,
                language=language,
                vad=vad,
                word_timestamps=word_ts,
                progress_callback=lambda s: self.queue.put(s),
                cancel_event=self.cancel_event,
                output_path=save_path,
            )
            if chunk_minutes > 0:
                self.engine.transcribe_chunked(**kw, chunk_minutes=chunk_minutes)
            else:
                self.engine.transcribe(**kw)
        except Exception as e:
            self.queue.put({"status": "error", "error": str(e)})

    def _reset_transcribe_btn(self):
        self.transcribe_btn.configure(
            state="normal", text="Transcribe",
            fg_color="#2b7a3e", hover_color="#1f5c2e",
        )
        self._set_settings_enabled(True)

    # ── Record View ───────────────────────────────────────

    def _build_record_view(self):
        parent = self.record_frame
        self._build_record_sidebar(parent)
        self._build_record_main(parent)

    def _build_record_sidebar(self, parent):
        sidebar = ctk.CTkFrame(parent, width=self.SIDEBAR_WIDTH, corner_radius=0)
        sidebar.pack(side="left", fill="y")
        sidebar.pack_propagate(False)

        inner = ctk.CTkFrame(sidebar, fg_color="transparent")
        inner.pack(fill="both", expand=True, padx=12, pady=12)

        header_row = ctk.CTkFrame(inner, fg_color="transparent")
        header_row.pack(fill="x", pady=(0, 10))

        self.back_btn = ctk.CTkButton(
            header_row, text="\u2190  Back", width=60, height=24,
            command=self._go_back, fg_color="transparent",
            text_color=("gray10", "gray90"), hover_color=("#d0d0d0", "#404040"),
            font=("", 13),
        )
        self.back_btn.pack(side="left")

        ctk.CTkLabel(header_row, text="Live Recording",
                     font=("", 13, "bold")).pack(side="left", padx=(8, 0))

        ctk.CTkFrame(inner, height=1, fg_color=("gray85", "gray30")
                     ).pack(fill="x", pady=(0, 24))

        ctk.CTkLabel(inner, text="Save audio as").pack(anchor="w", pady=(0, 6))

        save_audio_row = ctk.CTkFrame(inner, fg_color="transparent")
        save_audio_row.pack(fill="x", pady=(0, 12))

        self.record_audio_entry = ctk.CTkEntry(save_audio_row, height=36,
                                               placeholder_text="recording.wav")
        self.record_audio_entry.pack(side="left", fill="x", expand=True, padx=(0, 6))

        self.record_audio_btn = ctk.CTkButton(save_audio_row, text="Choose",
                                               command=self._browse_record_audio,
                                               width=80,
                                               fg_color=("#D4D4D4", "#3A3A3A"),
                                               hover_color=("#C0C0C0", "#4A4A4A"),
                                               text_color=("gray10", "gray90"))
        self.record_audio_btn.pack(side="right")

        ctk.CTkFrame(inner, height=1, fg_color=("gray85", "gray30")
                     ).pack(fill="x", pady=(0, 12))

        ctk.CTkLabel(inner, text="Save transcript as").pack(anchor="w", pady=(0, 6))

        save_transcript_row = ctk.CTkFrame(inner, fg_color="transparent")
        save_transcript_row.pack(fill="x", pady=(0, 12))

        self.record_transcript_entry = ctk.CTkEntry(save_transcript_row, height=36,
                                                     placeholder_text="transcript.txt")
        self.record_transcript_entry.pack(side="left", fill="x", expand=True, padx=(0, 6))

        self.record_transcript_btn = ctk.CTkButton(save_transcript_row, text="Choose",
                                                    command=self._browse_record_transcript,
                                                    width=80,
                                                    fg_color=("#D4D4D4", "#3A3A3A"),
                                                    hover_color=("#C0C0C0", "#4A4A4A"),
                                                    text_color=("gray10", "gray90"))
        self.record_transcript_btn.pack(side="right")

        ctk.CTkFrame(inner, height=1, fg_color=("gray85", "gray30")
                     ).pack(fill="x", pady=(0, 12))

        self._file_setting_label(inner, "Model")
        self.record_model_var = ctk.StringVar(value="Accurate")
        self.record_model_segment = ctk.CTkSegmentedButton(
            inner, variable=self.record_model_var,
            values=list(MODEL_LABELS.keys()),
            height=36,
        )
        self.record_model_segment.pack(fill="x", pady=(0, 8))

        ctk.CTkFrame(inner, height=1, fg_color=("gray85", "gray30")
                     ).pack(fill="x", pady=(0, 10))

        self.record_vad_var = ctk.BooleanVar(value=True)
        self.record_vad_switch = ctk.CTkSwitch(
            inner, text="VAD (skip silence)", variable=self.record_vad_var,
        )
        self.record_vad_switch.pack(anchor="w", pady=(0, 4))

        ctk.CTkFrame(inner, height=1, fg_color=("gray85", "gray30")
                     ).pack(fill="x", pady=(0, 12))

        self.record_btn = ctk.CTkButton(
            inner, text="\u25cf  Record", command=self._toggle_recording,
            height=44, font=("", 15, "bold"), fg_color="#2b7a3e",
            hover_color="#1f5c2e",
        )
        self.record_btn.pack(fill="x", pady=(0, 4))

        self.record_indicator = ctk.CTkLabel(
            inner, text="", font=("", 13, "bold"), text_color="#b33a3a",
        )
        self.record_indicator.pack(anchor="w")

    def _build_record_main(self, parent):
        main = ctk.CTkFrame(parent, corner_radius=0)
        main.pack(side="right", fill="both", expand=True)

        self.record_output_text = self._build_text_area(main)

    # ── Record Path Helpers ──────────────────────────────

    def _browse_record_audio(self):
        path = filedialog.asksaveasfilename(
            title="Save Recording Audio",
            defaultextension=".wav",
            filetypes=[("WAV Audio", "*.wav"), ("All Files", "*.*")],
        )
        if path:
            self.record_audio_entry.delete(0, "end")
            self.record_audio_entry.insert(0, path)
            p = Path(path)
            self.record_transcript_entry.delete(0, "end")
            self.record_transcript_entry.insert(0, str(p.with_suffix(".txt")))

    def _browse_record_transcript(self):
        path = filedialog.asksaveasfilename(
            title="Save Transcript",
            defaultextension=".txt",
            filetypes=[("Text Files", "*.txt"), ("All Files", "*.*")],
        )
        if path:
            self.record_transcript_entry.delete(0, "end")
            self.record_transcript_entry.insert(0, path)
            p = Path(path)
            self.record_audio_entry.delete(0, "end")
            self.record_audio_entry.insert(0, str(p.with_suffix(".wav")))

    # ── Recording ─────────────────────────────────────────

    def _toggle_recording(self):
        if self.recording:
            self._stop_recording()
        else:
            self._start_recording()

    def _set_record_settings_enabled(self, enabled):
        state = "normal" if enabled else "disabled"
        self.record_model_segment.configure(state=state)
        self.record_vad_switch.configure(state=state)
        self.record_audio_entry.configure(state=state)
        self.record_audio_btn.configure(state=state)
        self.record_transcript_entry.configure(state=state)
        self.record_transcript_btn.configure(state=state)

    def _start_recording(self):
        self.recording = True
        self.record_stop = threading.Event()
        self.recording_text = ""

        audio_path = self.record_audio_entry.get().strip()
        transcript_path = self.record_transcript_entry.get().strip()
        if not audio_path:
            ts = datetime.now().strftime("%Y%m%d_%H%M%S")
            audio_path = str(Path.home() / f"recording_{ts}.wav")
        if not transcript_path:
            ts = datetime.now().strftime("%Y%m%d_%H%M%S")
            transcript_path = str(Path.home() / f"recording_{ts}.txt")

        self.recording_wav_path = audio_path
        self.recording_transcript_path = transcript_path

        self.record_btn.configure(
            text="\u25a0  Stop", fg_color="#b33a3a", hover_color="#8a2b2b",
        )
        self.record_indicator.configure(text="Starting...")
        self.record_output_text.delete("0.0", "end")
        self.status_bar.configure(text="Recording")
        self._set_record_settings_enabled(False)

        thread = threading.Thread(
            target=self._run_recording,
            daemon=True,
        )
        thread.start()

    def _stop_recording(self):
        self.record_stop.set()
        self.record_btn.configure(
            text="\u25cf  Record", state="disabled",
            fg_color="#555555", hover_color="#555555",
        )
        self.record_indicator.configure(text="Finalising...")

    def _run_recording(self):
        try:
            model = MODEL_LABELS[self.record_model_var.get()]
            language = "en"
            vad = self.record_vad_var.get()

            self.recorder = AudioRecorder()
            self.recorder.start(wav_path=self.recording_wav_path)

            self.queue.put({
                "status": "recording_start",
                "wav_path": self.recording_wav_path,
            })

            full_text = ""
            first_window = True
            transcript_path = Path(self.recording_transcript_path)

            while True:
                if self.record_stop.wait(timeout=TICK_SEC):
                    break

                audio = self.recorder.get_buffer(WINDOW_SEC)
                if audio is None:
                    continue
                if not self.recorder.has_speech(audio):
                    continue

                result = self.engine.transcribe_buffer(
                    audio,
                    model_name=model,
                    language=language,
                    vad=vad,
                    cancel_event=self.record_stop,
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
                    transcript_path.write_text(full_text, encoding="utf-8")
                    self.queue.put({
                        "status": "recording_transcript",
                        "text": full_text,
                        "elapsed": self.recorder.elapsed,
                    })

            recorder = self.recorder
            self.recorder = None
            if recorder:
                recorder.stop()

            target = Path(self.recording_transcript_path)
            if full_text:
                target.write_text(full_text, encoding="utf-8")

            self.queue.put({
                "status": "recording_stop",
                "text": full_text,
                "wav_path": self.recording_wav_path,
                "transcript_path": self.recording_transcript_path,
            })

        except Exception as e:
            self.queue.put({"status": "error", "error": str(e)})

    # ── Queue Polling ─────────────────────────────────────

    def _poll_queue(self):
        try:
            while True:
                status = self.queue.get_nowait()

                # ── File transcription statuses ──────

                if status["status"] == "loading":
                    self.status_bar.configure(
                        text=status.get("message", "Loading..."))

                elif status["status"] == "chunk_start":
                    self.status_bar.configure(
                        text=status.get("message", "Chunking..."))

                elif status["status"] == "transcribing":
                    self.progress_bar.set(status["progress"])
                    text = status.get("text", "")
                    self.file_output_text.delete("0.0", "end")
                    self.file_output_text.insert("0.0", text)
                    self.file_output_text.see("end")
                    pct = int(status["progress"] * 100)
                    chunk_i = status.get("chunk_index", -1)
                    chunk_n = status.get("chunk_total", 0)
                    if chunk_n > 0:
                        self.status_bar.configure(
                            text=f"Chunk {chunk_i + 1}/{chunk_n} \u2014 {pct}%")
                    else:
                        self.status_bar.configure(text=f"{pct}%")

                elif status["status"] == "done":
                    result = status.get("result", {})
                    self.latest_result = result
                    self.file_output_text.delete("0.0", "end")
                    self.file_output_text.insert("0.0", result.get("text", ""))
                    self.progress_bar.set(1.0)
                    dur = result.get("audio_duration", 0)
                    lang = result.get("detected_language", "?")
                    self.status_bar.configure(
                        text=f"{lang}  |  {dur:.0f}s  |  {result.get('model', '?')}")
                    self.running = False
                    self._reset_transcribe_btn()

                elif status["status"] == "cancelled":
                    result = status.get("result", {})
                    self.latest_result = result
                    self.file_output_text.delete("0.0", "end")
                    partial = result.get("text", "")
                    self.file_output_text.insert(
                        "0.0",
                        f"\u23f9 Cancelled \u2014 partial transcript below:\n\n{partial}"
                        if partial else "\u23f9 Transcription cancelled.")
                    dur = result.get("audio_duration", 0)
                    lang = result.get("detected_language", "?")
                    self.status_bar.configure(
                        text=f"{lang}  |  {dur:.0f}s  |  Cancelled")
                    self.running = False
                    self._reset_transcribe_btn()

                # ── Recording statuses ────────────────

                elif status["status"] == "recording_start":
                    self.record_indicator.configure(
                        text=f"\u25cf  {self._fmt_elapsed(0)}")
                    self.status_bar.configure(
                        text=f"Recording to {Path(status['wav_path']).name}")

                elif status["status"] == "recording_transcript":
                    elapsed = status.get("elapsed", 0)
                    text = status.get("text", "")
                    self.recording_text = text
                    self.record_indicator.configure(
                        text=f"\u25cf  {self._fmt_elapsed(elapsed)}")
                    self.record_output_text.delete("0.0", "end")
                    self.record_output_text.insert("0.0", text)
                    self.record_output_text.see("end")

                elif status["status"] == "recording_stop":
                    text = status.get("text", "")
                    self.recording_text = text
                    self.record_output_text.delete("0.0", "end")
                    self.record_output_text.insert("0.0", text)
                    self.record_indicator.configure(text="Stopped")
                    wav = Path(status["wav_path"]).name
                    txt = Path(status.get("transcript_path", "")).name
                    self.status_bar.configure(
                        text=f"{wav}  |  {txt}" if txt else wav)
                    self.recording = False
                    self._reset_record_btn()

                # ── Error ───────────────────────────

                elif status["status"] == "error":
                    err = status.get("error", "Unknown error")
                    target = self._current_view
                    out = (self.file_output_text if target == "file"
                           else self.record_output_text)
                    out.delete("0.0", "end")
                    out.insert("0.0", f"Error: {err}")
                    if target == "file":
                        self.status_bar.configure(text="Error")
                        self.running = False
                        self._reset_transcribe_btn()
                    else:
                        self.record_indicator.configure(text="Error")
                        self.recording = False
                        self._reset_record_btn()
                    messagebox.showerror("Error", err)

        except queue.Empty:
            pass

        self.after(100, self._poll_queue)

    def _fmt_elapsed(self, seconds):
        m, s = divmod(int(seconds), 60)
        h, m = divmod(m, 60)
        if h:
            return f"{h}:{m:02d}:{s:02d}"
        return f"{m}:{s:02d}"

    def _reset_record_btn(self):
        self.record_btn.configure(
            state="normal", text="\u25cf  Record",
            fg_color="#2b7a3e", hover_color="#1f5c2e",
        )
        self._set_record_settings_enabled(True)

    # ── Actions (shared) ─────────────────────────────────


def main():
    app = TranscriberApp()
    app.mainloop()


if __name__ == "__main__":
    main()
