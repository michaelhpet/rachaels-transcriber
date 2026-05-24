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
pub async fn download_model(
    label: String,
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let cancel = state.cancel_flag.clone();
    cancel.store(false, Ordering::Relaxed);

    let app_handle_clone = app_handle.clone();
    let progress_clone = label.clone();
    download_models::download_model(&label, Some(&cancel), Some(Box::new(move |downloaded, total| {
        let _ = app_handle_clone.emit("download-progress", DownloadProgress {
            label: progress_clone.clone(),
            downloaded,
            total,
        });
    })))
    .await
    .map_err(|e| e.to_string())?;

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
    _model: String,
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let cancel = state.cancel_flag.clone();
    cancel.store(false, Ordering::Relaxed);

    let mut recorder = AudioRecorder::new();
    recorder.start(None).map_err(|e| e.to_string())?;

    state.recorder.lock().replace(recorder);

    let cancel_clone = cancel.clone();
    let app_handle_clone = app_handle.clone();

    std::thread::spawn(move || {
        let window_sec = 5.0;
        loop {
            if cancel_clone.load(Ordering::Relaxed) {
                break;
            }

            let state = app_handle_clone.state::<AppState>();
            if let Some(ref recorder) = *state.recorder.lock() {
                if let Some(buf) = recorder.get_buffer(window_sec) {
                    let text = format!("[captured {} samples at {window_sec}s]", buf.len());
                    let elapsed = recorder.elapsed();
                    let _ = app_handle_clone.emit("record-progress", RecordProgress {
                        elapsed,
                        text,
                    });
                }
            }

            std::thread::sleep(std::time::Duration::from_millis(2000));
        }
    });

    Ok(())
}

#[tauri::command]
pub fn stop_recording(
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    state.cancel_flag.store(true, Ordering::Relaxed);

    let mut recorder_opt = state.recorder.lock();
    if let Some(mut recorder) = recorder_opt.take() {
        let audio = recorder.stop().unwrap_or_default();
        let text = format!("[recorded {} samples]", audio.len());
        Ok(text)
    } else {
        Err("not recording".to_string())
    }
}

#[tauri::command]
pub fn cancel(state: tauri::State<'_, AppState>) {
    state.cancel_flag.store(true, Ordering::Relaxed);
}

#[tauri::command]
pub fn save_text_file(path: String, content: String) -> Result<(), String> {
    std::fs::write(&path, &content).map_err(|e| e.to_string())
}
