use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use eframe::egui;
use egui::RichText;

use crate::config;
use crate::download_models;
use crate::engine::chunk;
use crate::engine::whisper::{self, WhisperEngine};
use crate::recorder::AudioRecorder;

#[derive(Debug, Clone, PartialEq)]
enum AppView {
    Landing,
    FileTranscribe,
    LiveRecord,
}

#[derive(Debug, Clone, PartialEq)]
enum Status {
    Idle,
    Downloading { label: String, downloaded: u64, total: u64 },
    Transcribing { progress: f64, text: String },
    Recording { elapsed: f64, text: String },
    Done(String),
    Error(String),
}

pub struct TranscriberApp {
    // View state
    view: AppView,

    // File transcribe state
    file_path: Option<PathBuf>,
    save_path: Option<PathBuf>,
    model_choice: String,
    use_vad: bool,
    enable_wts: bool,
    transcription_handle: Option<std::thread::JoinHandle<()>>,
    cancel_flag: Arc<AtomicBool>,

    // Recording state
    recorder_stop_flag: Option<Arc<AtomicBool>>,
    recording_handle: Option<std::thread::JoinHandle<()>>,

    // Shared state
    status: Arc<Mutex<Status>>,
    output_text: String,
    needs_model_check: bool,
    model_check_done: bool,
    needs_download: Vec<String>,
    download_cancel: Arc<AtomicBool>,
}

impl Default for TranscriberApp {
    fn default() -> Self {
        Self {
            view: AppView::Landing,
            file_path: None,
            save_path: None,
            model_choice: "Accurate".to_string(),
            use_vad: true,
            enable_wts: false,
            transcription_handle: None,
            cancel_flag: Arc::new(AtomicBool::new(false)),
            recorder_stop_flag: None,
            recording_handle: None,
            status: Arc::new(Mutex::new(Status::Idle)),
            output_text: String::new(),
            needs_model_check: true,
            model_check_done: false,
            needs_download: Vec::new(),
            download_cancel: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl TranscriberApp {
    fn show_landing(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(ui.available_height() * 0.25);

                // Title
                ui.heading(RichText::new("Rachael's Transcriber").size(28.0));
                ui.add_space(16.0);
                ui.label("Transcribe audio files or record from your microphone");
                ui.add_space(40.0);

                // Two cards
                let available = ui.available_width();
                let card_width = 260.0;
                let spacing = 20.0;
                let total_width = card_width * 2.0 + spacing;
                let start_x = (available - total_width).max(0.0) * 0.5;

                ui.horizontal(|ui| {
                    ui.add_space(start_x);

                    // File Transcription card
                    let file_response = egui::Frame::none()
                        .fill(egui::Style::default().visuals.extreme_bg_color)
                        .stroke(egui::Stroke::new(1.0, egui::Color32::GRAY))
                        .rounding(16.0)
                        .show(ui, |ui| {
                            ui.set_min_size(egui::vec2(card_width, 210.0));
                            ui.vertical_centered(|ui| {
                                ui.add_space(50.0);
                                egui::Frame::none()
                                    .fill(egui::Color32::from_rgb(0x00, 0x7A, 0xFF))
                                    .rounding(16.0)
                                    .show(ui, |ui| {
                                        ui.set_min_size(egui::vec2(64.0, 64.0));
                                        ui.vertical_centered(|ui| {
                                            ui.add_space(16.0);
                                            ui.label(
                                                RichText::new("\u{25A3}").size(28.0).color(egui::Color32::WHITE),
                                            );
                                        });
                                    });
                                ui.add_space(12.0);
                                ui.label(RichText::new("File Transcription").size(18.0).strong());
                                ui.add_space(12.0);
                                ui.label("Transcribe existing\naudio files");
                            });
                        });

                    if file_response.response.clicked() {
                        self.switch_to(ViewMode::FileTranscribe);
                    }

                    ui.add_space(spacing);

                    // Recording card
                    let record_response = egui::Frame::none()
                        .fill(egui::Style::default().visuals.extreme_bg_color)
                        .stroke(egui::Stroke::new(1.0, egui::Color32::GRAY))
                        .rounding(16.0)
                        .show(ui, |ui| {
                            ui.set_min_size(egui::vec2(card_width, 210.0));
                            ui.vertical_centered(|ui| {
                                ui.add_space(50.0);
                                egui::Frame::none()
                                    .fill(egui::Color32::from_rgb(0xb3, 0x3a, 0x3a))
                                    .rounding(16.0)
                                    .show(ui, |ui| {
                                        ui.set_min_size(egui::vec2(64.0, 64.0));
                                        ui.vertical_centered(|ui| {
                                            ui.add_space(16.0);
                                            ui.label(
                                                RichText::new("\u{25C9}").size(28.0).color(egui::Color32::WHITE),
                                            );
                                        });
                                    });
                                ui.add_space(12.0);
                                ui.label(RichText::new("Live Recording").size(18.0).strong());
                                ui.add_space(12.0);
                                ui.label("Record from your\nmicrophone live");
                            });
                        });

                    if record_response.response.clicked() {
                        self.switch_to(ViewMode::LiveRecord);
                    }
                });
            });
        });
    }

    fn show_file_view(&mut self, ctx: &egui::Context) {
        let side_panel_width = 260.0;

        // Sidebar
        egui::SidePanel::left("file_sidebar")
            .resizable(false)
            .default_width(side_panel_width)
            .show(ctx, |ui| {
                self.show_file_sidebar(ui);
            });

        // Main area
        egui::CentralPanel::default().show(ctx, |ui| {
            self.show_file_main(ui);
        });
    }

    fn show_file_sidebar(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            // Back button + title
            ui.horizontal(|ui| {
                if ui.button("\u{2190}  Back").clicked() {
                    self.cancel_transcription();
                    self.switch_to(ViewMode::Landing);
                }
                ui.label(RichText::new("File Transcription").strong());
            });
            ui.separator();
            ui.add_space(8.0);

            // File selection
            ui.label("File");
            ui.horizontal(|ui| {
                let path_text = self
                    .file_path
                    .as_ref()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .unwrap_or("No file selected");
                let mut file_str = path_text.to_string();
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut file_str)
                        .hint_text("No file selected")
                        .desired_width(f32::INFINITY),
                );
                if resp.clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Audio", &["mp3", "wav", "m4a", "flac", "ogg", "aac"])
                        .pick_file()
                    {
                        self.file_path = Some(path.clone());
                        // Auto-populate save path
                        if self.save_path.is_none() {
                            self.save_path = Some(path.with_extension("txt"));
                        }
                    }
                }
                if ui.button("Choose").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Audio", &["mp3", "wav", "m4a", "flac", "ogg", "aac"])
                        .pick_file()
                    {
                        self.file_path = Some(path.clone());
                        self.save_path = Some(path.with_extension("txt"));
                    }
                }
            });

            ui.separator();
            ui.add_space(8.0);

            // Save path
            ui.label("Save as");
            ui.horizontal(|ui| {
                let mut save_str = self
                    .save_path
                    .as_ref()
                    .and_then(|p| p.to_str())
                    .unwrap_or("")
                    .to_string();
                ui.add(
                    egui::TextEdit::singleline(&mut save_str)
                        .hint_text("Enter location")
                        .desired_width(f32::INFINITY),
                );
                if ui.button("Choose").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_title("Save as")
                        .add_filter("Text", &["txt"])
                        .save_file()
                    {
                        self.save_path = Some(path);
                    }
                }
            });

            ui.separator();
            ui.add_space(8.0);

            // Model selection
            ui.label("Model");
            egui::ComboBox::new("model_select", "")
                .selected_text(&self.model_choice)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.model_choice, "Accurate".to_string(), "Accurate (small.en)");
                    ui.selectable_value(&mut self.model_choice, "Fast".to_string(), "Fast (base.en)");
                });

            ui.separator();
            ui.add_space(8.0);

            // VAD toggle
            ui.checkbox(&mut self.use_vad, "VAD (skip silence)");

            // Word timestamps
            ui.checkbox(&mut self.enable_wts, "Word timestamps");

            ui.separator();
            ui.add_space(12.0);

            // Transcribe / Cancel button
            let status = self.status.lock().unwrap().clone();
            match status {
                Status::Idle | Status::Done(_) | Status::Error(_) => {
                    if ui
                        .add_sized(
                            ui.available_size(),
                            egui::Button::new(RichText::new("Transcribe").size(16.0).strong())
                                .fill(egui::Color32::from_rgb(0x2b, 0x7a, 0x3e)),
                        )
                        .clicked()
                    {
                        self.start_transcription();
                    }
                }
                Status::Transcribing { .. } | Status::Downloading { .. } => {
                    if ui
                        .add_sized(
                            ui.available_size(),
                            egui::Button::new(RichText::new("Cancel").size(16.0).strong())
                                .fill(egui::Color32::from_rgb(0xb3, 0x3a, 0x3a)),
                        )
                        .clicked()
                    {
                        self.cancel_transcription();
                    }
                }
                Status::Recording { .. } => {}
            }
        });
    }

    fn show_file_main(&mut self, ui: &mut egui::Ui) {
        let status = self.status.lock().unwrap().clone();

        // Status bar at bottom
        egui::TopBottomPanel::bottom("file_status_bar")
            .resizable(false)
            .min_height(24.0)
            .show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    match &status {
                        Status::Idle => {
                            if let Some(path) = &self.file_path {
                                ui.label(path.file_name().and_then(|n| n.to_str()).unwrap_or(""));
                            }
                        }
                        Status::Downloading { label, downloaded, total } => {
                            let pct = if *total > 0 {
                                (*downloaded as f64 / *total as f64 * 100.0) as u64
                            } else {
                                0
                            };
                            ui.label(format!("Downloading {label}... {pct}%"));
                        }
                        Status::Transcribing { progress, text: _ } => {
                            ui.label(format!("Transcribing... {:.0}%", progress * 100.0));
                        }
                        Status::Done(ref text) => {
                            let lines: Vec<&str> = text.lines().collect();
                            let word_count = text.split_whitespace().count();
                            ui.label(format!("Done — {word_count} words, {} lines", lines.len()));
                        }
                        Status::Error(ref e) => {
                            ui.label(format!("Error: {e}"));
                        }
                        Status::Recording { .. } => {}
                    }
                });
            });

        // Text area
        egui::ScrollArea::both()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                let text = match &status {
                    Status::Transcribing { text, .. } => text.clone(),
                    Status::Done(text) => text.clone(),
                    Status::Error(e) => format!("Error: {e}"),
                    _ => self.output_text.clone(),
                };
                let mut display_text = text;
                ui.add_sized(
                    ui.available_size(),
                    egui::TextEdit::multiline(&mut display_text)
                        .desired_rows(30)
                        .font(egui::TextStyle::Monospace),
                );
            });
    }

    fn show_record_view(&mut self, ctx: &egui::Context) {
        let side_panel_width = 260.0;

        egui::SidePanel::left("record_sidebar")
            .resizable(false)
            .default_width(side_panel_width)
            .show(ctx, |ui| {
                self.show_record_sidebar(ui);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            let status = self.status.lock().unwrap().clone();

            // Status bar
            egui::TopBottomPanel::bottom("record_status_bar")
                .resizable(false)
                .min_height(24.0)
                .show_inside(ui, |ui| {
                    ui.horizontal(|ui| {
                        match &status {
                            Status::Recording { elapsed, text: _ } => {
                                let m = (*elapsed as u32) / 60;
                                let s = (*elapsed as u32) % 60;
                                ui.label(format!("\u{25cf} Recording {m}:{s:02}"));
                            }
                            Status::Done(_) => {
                                ui.label("Recording stopped");
                            }
                            Status::Error(e) => {
                                ui.label(format!("Error: {e}"));
                            }
                            _ => {}
                        }
                    });
                });

            // Text area
            egui::ScrollArea::both()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    let text = match &status {
                        Status::Recording { text, .. } => text.clone(),
                        Status::Done(text) => text.clone(),
                        Status::Error(e) => format!("Error: {e}"),
                        _ => String::new(),
                    };
                    let mut display_text = text;
                    ui.add_sized(
                        ui.available_size(),
                        egui::TextEdit::multiline(&mut display_text)
                            .desired_rows(30)
                            .font(egui::TextStyle::Monospace),
                    );
                });
        });
    }

    fn show_record_sidebar(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                if ui.button("\u{2190}  Back").clicked() {
                    self.stop_recording();
                    self.switch_to(ViewMode::Landing);
                }
                ui.label(RichText::new("Live Recording").strong());
            });
            ui.separator();
            ui.add_space(8.0);

            // Model selection
            ui.label("Model");
            egui::ComboBox::new("record_model_select", "")
                .selected_text(&self.model_choice)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.model_choice, "Accurate".to_string(), "Accurate (small.en)");
                    ui.selectable_value(&mut self.model_choice, "Fast".to_string(), "Fast (base.en)");
                });

            ui.separator();
            ui.add_space(8.0);

            ui.checkbox(&mut self.use_vad, "VAD (skip silence)");

            ui.separator();
            ui.add_space(12.0);

            let is_recording = self.is_recording();
            if !is_recording {
                if ui
                    .add_sized(
                        ui.available_size(),
                        egui::Button::new(RichText::new("\u{25cf}  Record").size(16.0).strong())
                            .fill(egui::Color32::from_rgb(0x2b, 0x7a, 0x3e)),
                    )
                    .clicked()
                {
                    self.start_recording();
                }
            } else {
                if ui
                    .add_sized(
                        ui.available_size(),
                        egui::Button::new(RichText::new("\u{25a0}  Stop").size(16.0).strong())
                            .fill(egui::Color32::from_rgb(0xb3, 0x3a, 0x3a)),
                    )
                    .clicked()
                {
                    self.stop_recording();
                }
            }
        });
    }

    fn switch_to(&mut self, target: ViewMode) {
        self.view = match target {
            ViewMode::Landing => AppView::Landing,
            ViewMode::FileTranscribe => AppView::FileTranscribe,
            ViewMode::LiveRecord => AppView::LiveRecord,
        };
    }

    fn is_recording(&self) -> bool {
        self.recording_handle.is_some()
    }

    fn start_transcription(&mut self) {
        let file_path = match &self.file_path {
            Some(p) => p.clone(),
            None => return,
        };

        if !file_path.exists() {
            let mut status = self.status.lock().unwrap();
            *status = Status::Error("File does not exist".to_string());
            return;
        }

        // Check if model is downloaded
        let model_label = self.model_choice.clone();
        if !download_models::is_model_downloaded(&model_label) {
            let mut status = self.status.lock().unwrap();
            *status = Status::Error(format!(
                "Model '{model_label}' not downloaded. Please download it first."
            ));
            return;
        }

        let model_path = download_models::model_path(&model_label);
        let _use_vad = self.use_vad;
        let save_path = self.save_path.clone();
        let cancel_flag = self.cancel_flag.clone();
        let status = self.status.clone();

        cancel_flag.store(false, Ordering::Relaxed);

        let handle = std::thread::spawn(move || {
            // Update status to transcribing
            {
                let mut s = status.lock().unwrap();
                *s = Status::Transcribing {
                    progress: 0.0,
                    text: String::new(),
                };
            }

            // Decode audio
            let audio = match chunk::prepare_audio(&file_path) {
                Ok(a) => a,
                Err(e) => {
                    let mut s = status.lock().unwrap();
                    *s = Status::Error(format!("Failed to decode audio: {e}"));
                    return;
                }
            };

            // Load model
            let engine = match WhisperEngine::new(&model_path, "en") {
                Ok(e) => e,
                Err(e) => {
                    let mut s = status.lock().unwrap();
                    *s = Status::Error(format!("Failed to load model: {e}"));
                    return;
                }
            };

            // Transcribe
            let result = engine.transcribe_chunked(
                audio,
                config::CHUNK_SEC,
                config::CHUNK_OVERLAP_SEC,
                Some(&cancel_flag),
                Some(&|pct: f64, _text: &str| {
                    let mut s = status.lock().unwrap();
                    if let Status::Transcribing { ref mut progress, .. } = *s {
                        *progress = pct;
                    }
                }),
            );

            match result {
                Ok(segments) => {
                    let text = whisper::segments_to_text(&segments);

                    // Save to file if requested
                    if let Some(ref path) = save_path {
                        if !text.is_empty() {
                            let _ = std::fs::write(path, &text);
                        }
                    }

                    let mut s = status.lock().unwrap();
                    if cancel_flag.load(Ordering::Relaxed) {
                        *s = Status::Done(
                            if text.is_empty() {
                                "\u{23f9} Transcription cancelled.".to_string()
                            } else {
                                format!("\u{23F9} Cancelled \u{2014} partial transcript:\n\n{text}")
                            },
                        );
                    } else {
                        *s = Status::Done(text);
                    }
                }
                Err(e) => {
                    if cancel_flag.load(Ordering::Relaxed) {
                        let mut s = status.lock().unwrap();
                        *s = Status::Done("\u{23f9} Transcription cancelled.".to_string());
                    } else {
                        let mut s = status.lock().unwrap();
                        *s = Status::Error(format!("Transcription failed: {e}"));
                    }
                }
            }
        });

        self.transcription_handle = Some(handle);
    }

    fn cancel_transcription(&mut self) {
        self.cancel_flag.store(true, Ordering::Relaxed);
    }

    fn start_recording(&mut self) {
        let mut recorder = AudioRecorder::new();
        if let Err(e) = recorder.start(None) {
            let mut s = self.status.lock().unwrap();
            *s = Status::Error(format!("Failed to start recording: {e}"));
            return;
        }

        let use_vad = self.use_vad;

        let status = self.status.clone();
        let stop_flag = Arc::new(AtomicBool::new(false));
        self.recorder_stop_flag = Some(stop_flag.clone());

        let handle = std::thread::spawn(move || {
            {
                let mut s = status.lock().unwrap();
                *s = Status::Recording {
                    elapsed: 0.0,
                    text: String::new(),
                };
            }

            let start = Instant::now();
            let full_text = String::new();

            while !stop_flag.load(Ordering::Relaxed) {
                std::thread::sleep(std::time::Duration::from_secs_f64(config::TICK_SEC));

                let audio = recorder.get_buffer(config::WINDOW_SEC);
                if audio.is_none() {
                    continue;
                }
                let audio = audio.unwrap();

                if use_vad && !recorder.has_speech(&audio, 0.2) {
                    continue;
                }

                let elapsed = start.elapsed().as_secs_f64();
                let mut s = status.lock().unwrap();
                *s = Status::Recording {
                    elapsed,
                    text: full_text.clone(),
                };
            }

            recorder.stop();

            let mut s = status.lock().unwrap();
            *s = Status::Done(full_text);
        });

        self.recording_handle = Some(handle);
    }

    fn stop_recording(&mut self) {
        if let Some(flag) = self.recorder_stop_flag.take() {
            flag.store(true, Ordering::Relaxed);
        }
        if let Some(handle) = self.recording_handle.take() {
            handle.join().ok();
        }
    }

    fn show_download_dialog(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(120.0);
                ui.heading("Setting up");
                ui.add_space(4.0);
                ui.label("Downloading speech models (~600 MB). Internet required.");
                ui.add_space(20.0);

                let status_cloned = self.status.lock().unwrap().clone();
                match status_cloned {
                    Status::Idle => {
                        if !self.needs_download.is_empty() {
                            let labels = self.needs_download.clone();
                            let status = self.status.clone();
                            let cancel = self.download_cancel.clone();
                            cancel.store(false, Ordering::Relaxed);

                            std::thread::spawn(move || {
                                for lbl in &labels {
                                    {
                                        let mut s = status.lock().unwrap();
                                        *s = Status::Downloading {
                                            label: lbl.clone(),
                                            downloaded: 0,
                                            total: 0,
                                        };
                                    }

                                    let rt = tokio::runtime::Runtime::new().unwrap();
                                    let result = rt.block_on(async {
                                        download_models::download_model(
                                            lbl,
                                            Some(&cancel),
                                            Some(&|downloaded: u64, total: u64| {
                                                let mut s = status.lock().unwrap();
                                                *s = Status::Downloading {
                                                    label: lbl.clone(),
                                                    downloaded,
                                                    total,
                                                };
                                            }),
                                        )
                                        .await
                                    });

                                    if let Err(e) = result {
                                        let mut s = status.lock().unwrap();
                                        *s = Status::Error(format!("Download failed: {e}"));
                                        return;
                                    }
                                }

                                *status.lock().unwrap() = Status::Done("Models downloaded".to_string());
                            });
                        }
                    }
                    Status::Downloading { ref label, downloaded, total } => {
                        let pct = if total > 0 {
                            (downloaded as f64 / total as f64 * 100.0) as u32
                        } else {
                            0
                        };
                        ui.label(format!("Downloading {label}..."));
                        ui.add(
                            egui::ProgressBar::new(pct as f32 / 100.0)
                                .show_percentage()
                                .desired_width(400.0),
                        );
                        ui.add_space(12.0);
                        ui.label(format!("{pct}% — {:.1} MB / {:.1} MB",
                            downloaded as f64 / 1_000_000.0,
                            total as f64 / 1_000_000.0));
                        ctx.request_repaint();
                    }
                    Status::Done(_) => {
                        ui.label("All models downloaded!");
                        ui.add_space(12.0);
                        if ui.button("Continue").clicked() {
                            self.needs_download.clear();
                        }
                        ctx.request_repaint();
                    }
                    Status::Error(ref e) => {
                        ui.label(RichText::new(format!("Error: {e}")).color(egui::Color32::RED));
                        ui.add_space(12.0);
                        if ui.button("Retry").clicked() {
                            let missing = download_models::check_missing_models();
                            self.needs_download = missing.iter().map(|&s| s.to_string()).collect();
                            *self.status.lock().unwrap() = Status::Idle;
                        }
                        if ui.button("Cancel").clicked() {
                            self.download_cancel.store(true, Ordering::Relaxed);
                            std::process::exit(1);
                        }
                        ctx.request_repaint();
                    }
                    Status::Transcribing { .. } | Status::Recording { .. } => {}
                }
            });
        });
    }

    fn check_models(&mut self) {
        let missing = download_models::check_missing_models();
        if !missing.is_empty() {
            self.needs_download = missing.iter().map(|&s| s.to_string()).collect();
        }
        self.model_check_done = true;
    }
}

enum ViewMode {
    Landing,
    FileTranscribe,
    LiveRecord,
}

impl eframe::App for TranscriberApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        if self.needs_model_check {
            self.needs_model_check = false;
            self.check_models();
            ctx.request_repaint();
        }

        if !self.needs_download.is_empty() {
            self.show_download_dialog(&ctx);
            return;
        }

        match self.view {
            AppView::Landing => self.show_landing(&ctx),
            AppView::FileTranscribe => self.show_file_view(&ctx),
            AppView::LiveRecord => self.show_record_view(&ctx),
        }

        let needs_repaint = matches!(
            *self.status.lock().unwrap(),
            Status::Transcribing { .. } | Status::Recording { .. } | Status::Downloading { .. }
        );
        if needs_repaint {
            ctx.request_repaint();
        }
    }
}
