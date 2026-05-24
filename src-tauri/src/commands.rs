use std::sync::atomic::Ordering;

use serde::Serialize;
use tauri::Emitter;
use tauri::Manager;

use crate::config;
use crate::download_models;
use crate::engine::chunk;
use crate::engine::whisper::{self, WhisperEngine};
use crate::recorder::AudioRecorder;
use crate::AppState;

#[derive(Serialize)]
pub struct MissingModels {
    pub accurate: bool,
    pub fast: bool,
}

#[derive(Serialize, Clone)]
pub struct DownloadProgress {
    pub label: String,
    pub downloaded: u64,
    pub total: u64,
}

#[derive(Serialize, Clone)]
pub struct TranscribeProgress {
    pub progress: f64,
    pub text: String,
}

#[derive(Serialize, Clone)]
pub struct RecordProgress {
    pub elapsed: f64,
    pub text: String,
}

#[derive(Serialize, Clone)]
pub struct RecordSegment {
    pub text: String,
}

#[tauri::command]
pub fn check_models() -> MissingModels {
    let accurate = download_models::is_model_downloaded("Accurate");
    let fast = download_models::is_model_downloaded("Fast");
    MissingModels {
        accurate,
        fast,
    }
}

#[tauri::command]
pub async fn download_models(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let cancel = state.cancel_flag.clone();
    cancel.store(false, Ordering::Relaxed);

    let fast_mb = 141u64;
    let accurate_mb = 465u64;
    let total_mb = fast_mb + accurate_mb;

    let models = [("Fast", fast_mb), ("Accurate", accurate_mb)];
    let mut accumulated: u64 = 0;

    for &(label, size_mb) in &models {
        if download_models::is_model_downloaded(label) {
            accumulated += size_mb;
            continue;
        }

        let app_handle_clone = app_handle.clone();
        let label_owned = label.to_string();
        let acc = accumulated;

        download_models::download_model(label, Some(&cancel), Some(Box::new(move |downloaded, total| {
            let pct = if total > 0 { downloaded as f64 / total as f64 } else { 0.0 };
            let combined = (acc as f64 + pct * size_mb as f64) as u64;
            let _ = app_handle_clone.emit("download-progress", DownloadProgress {
                label: label_owned.clone(),
                downloaded: combined,
                total: total_mb,
            });
        })))
        .await
        .map_err(|e| e.to_string())?;

        accumulated += size_mb;
    }

    let _ = app_handle.emit("download-done", ());
    Ok(())
}

#[tauri::command]
pub fn get_models_dir() -> String {
    config::models_dir().to_string_lossy().to_string()
}

#[tauri::command]
pub fn pick_audio_file() -> Option<String> {
    rfd::FileDialog::new()
        .add_filter("Audio", &["wav", "mp3", "m4a", "flac", "ogg", "aac", "wma"])
        .pick_file()
        .map(|p| p.to_string_lossy().to_string())
}

#[tauri::command]
pub fn pick_save_file() -> Option<String> {
    rfd::FileDialog::new()
        .add_filter("Text", &["txt"])
        .set_file_name("transcript.txt")
        .save_file()
        .map(|p| p.to_string_lossy().to_string())
}

#[tauri::command]
pub fn pick_audio_save_file() -> Option<String> {
    rfd::FileDialog::new()
        .add_filter("Audio", &["wav"])
        .set_file_name("recording.wav")
        .save_file()
        .map(|p| p.to_string_lossy().to_string())
}

#[tauri::command]
pub fn transcribe_file(
    path: String,
    model: String,
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let cancel = state.cancel_flag.clone();
    cancel.store(false, Ordering::Relaxed);

    let path_buf = std::path::PathBuf::from(&path);
    if !path_buf.exists() {
        return Err("file not found".to_string());
    }

    std::thread::spawn(move || {
        let model_path = download_models::model_path(&model);
        if !model_path.exists() {
            let _ = app_handle.emit("transcribe-error", "Model not found. Please download it first.".to_string());
            return;
        }

        let engine = match WhisperEngine::new(&model_path, "en") {
            Ok(e) => e,
            Err(e) => {
                let _ = app_handle.emit("transcribe-error", format!("Failed to load model: {e}"));
                return;
            }
        };

        let audio = match chunk::prepare_audio(&path_buf) {
            Ok(a) => a,
            Err(e) => {
                let _ = app_handle.emit("transcribe-error", format!("Failed to decode audio: {e}"));
                return;
            }
        };

        let cancel_ref = &cancel;
        let segments = match engine.transcribe(
            &audio,
            Some(cancel_ref),
            Some(&|progress: f64| {
                let _ = app_handle.emit("transcribe-progress", TranscribeProgress {
                    progress,
                    text: String::new(),
                });
            }),
        ) {
            Ok(s) => s,
            Err(e) => {
                let _ = app_handle.emit("transcribe-error", format!("Transcription failed: {e}"));
                return;
            }
        };

        let text = whisper::segments_to_text(&segments);
        let _ = app_handle.emit("transcribe-done", text);
    });

    Ok(())
}

#[tauri::command]
pub fn start_recording(
    model: String,
    save_audio_path: Option<String>,
    save_transcript_path: Option<String>,
    vad_enabled: Option<bool>,
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let cancel = state.cancel_flag.clone();
    cancel.store(false, Ordering::Relaxed);

    let vad = vad_enabled.unwrap_or(true);
    state.vad_enabled.store(vad, Ordering::Relaxed);

    *state.save_audio_path.lock() = save_audio_path.map(std::path::PathBuf::from);
    *state.save_transcript_path.lock() = save_transcript_path.map(std::path::PathBuf::from);

    // Load the Whisper engine
    let model_path = download_models::model_path(&model);
    if !model_path.exists() {
        return Err("Model not found. Please download it first.".to_string());
    }
    let engine = WhisperEngine::new(&model_path, "en")
        .map_err(|e| format!("Failed to load model: {e}"))?;
    state.engine.lock().replace(engine);

    let mut recorder = AudioRecorder::new();
    recorder.start(None).map_err(|e| e.to_string())?;
    state.recorder.lock().replace(recorder);

    let cancel_clone = cancel.clone();
    let app_handle_clone = app_handle.clone();
    let vad_enabled_flag = state.vad_enabled.clone();
    let raw_consumed = state.samples_processed.clone();
    raw_consumed.store(0, Ordering::Relaxed);

    let chunk_sec = crate::config::WINDOW_SEC; // 4.0 seconds
    let tick_sec = crate::config::TICK_SEC; // 2.0 seconds

    std::thread::spawn(move || {
        loop {
            if cancel_clone.load(Ordering::Relaxed) {
                break;
            }

            let state = app_handle_clone.state::<AppState>();
            let recorder_guard = state.recorder.lock();
            if let Some(ref recorder) = *recorder_guard {
                let consumed = raw_consumed.load(Ordering::Relaxed);

                if let Some((chunk, taken)) = recorder.read_chunk(consumed, chunk_sec) {
                    let has_speech = if vad_enabled_flag.load(Ordering::Relaxed) {
                        recorder.has_speech(&chunk, 0.5)
                    } else {
                        true
                    };

                    let text = if has_speech {
                        if let Some(ref engine) = *state.engine.lock() {
                            let cancel_ref = &cancel_clone;
                            let segments = match engine.transcribe(
                                &chunk,
                                Some(cancel_ref),
                                Some(&|_| {}),
                            ) {
                                Ok(s) => s,
                                Err(_) => vec![],
                            };
                            whisper::segments_to_text(&segments)
                        } else {
                            String::new()
                        }
                    } else {
                        String::new()
                    };

                    raw_consumed.store(consumed + taken, Ordering::Relaxed);

                    let elapsed = recorder.elapsed();
                    let _ = app_handle_clone.emit("record-segment", RecordSegment {
                        text: text.clone(),
                    });
                    let _ = app_handle_clone.emit("record-progress", RecordProgress {
                        elapsed,
                        text,
                    });
                }
            }
            drop(recorder_guard);

            std::thread::sleep(std::time::Duration::from_secs_f64(tick_sec));
        }
    });

    Ok(())
}

#[tauri::command]
pub fn stop_recording(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    state.cancel_flag.store(true, Ordering::Relaxed);

    let mut recorder_opt = state.recorder.lock();
    if let Some(mut recorder) = recorder_opt.take() {
        let full_audio = recorder.stop().unwrap_or_default();

        // Transcribe the entire recording (frontend replaces live segments with this)
        let text = if full_audio.is_empty() {
            String::new()
        } else {
            let engine = state.engine.lock().take();
            if let Some(engine) = engine {
                let cancel_ref = state.cancel_flag.clone();
                let segments = engine
                    .transcribe(&full_audio, Some(&cancel_ref), Some(&|_| {}))
                    .unwrap_or_default();
                whisper::segments_to_text(&segments)
            } else {
                String::new()
            }
        };

        let audio_path = state.save_audio_path.lock().take();
        let transcript_path = state.save_transcript_path.lock().take();

        if let Some(ref path) = audio_path {
            if !full_audio.is_empty() {
                write_wav(path, &full_audio).map_err(|e| format!("failed to save audio: {e}"))?;
            }
        }

        if let Some(ref path) = transcript_path {
            if !text.is_empty() {
                std::fs::write(path, &text)
                    .map_err(|e| format!("failed to save transcript: {e}"))?;
            }
        }

        let _ = app_handle.emit("transcribe-done", text.clone());
        Ok(text)
    } else {
        Err("not recording".to_string())
    }
}

fn write_wav(path: &std::path::Path, samples: &[f32]) -> Result<(), String> {
    use hound::WavSpec;
    let spec = WavSpec {
        channels: 1,
        sample_rate: crate::config::SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).map_err(|e| e.to_string())?;
    for &sample in samples {
        let clamped = sample.clamp(-1.0, 1.0);
        let amplitude = i16::MAX as f32;
        let int_sample = (clamped * amplitude) as i16;
        writer.write_sample(int_sample).map_err(|e| e.to_string())?;
    }
    writer.finalize().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn cancel(state: tauri::State<'_, AppState>) {
    state.cancel_flag.store(true, Ordering::Relaxed);
}

#[tauri::command]
pub fn save_text_file(path: String, content: String) -> Result<(), String> {
    std::fs::write(&path, &content).map_err(|e| e.to_string())
}
