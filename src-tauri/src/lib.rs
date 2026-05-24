pub mod commands;
pub mod config;
pub mod download_models;
pub mod engine;
pub mod recorder;

use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::Arc;

use parking_lot::Mutex;
use tauri::Manager;

use engine::whisper::WhisperEngine;
use recorder::AudioRecorder;

pub struct AppState {
    pub cancel_flag: Arc<AtomicBool>,
    pub engine: Mutex<Option<WhisperEngine>>,
    pub recorder: Mutex<Option<AudioRecorder>>,
    pub save_audio_path: Mutex<Option<std::path::PathBuf>>,
    pub save_transcript_path: Mutex<Option<std::path::PathBuf>>,
    pub vad_enabled: Arc<AtomicBool>,
    pub samples_processed: Arc<AtomicUsize>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            cancel_flag: Arc::new(AtomicBool::new(false)),
            engine: Mutex::new(None),
            recorder: Mutex::new(None),
            save_audio_path: Mutex::new(None),
            save_transcript_path: Mutex::new(None),
            vad_enabled: Arc::new(AtomicBool::new(true)),
            samples_processed: Arc::new(AtomicUsize::new(0)),
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let models_dir = data_dir.join("models");
            std::fs::create_dir_all(&models_dir).ok();
            config::set_models_dir(models_dir);
            Ok(())
        })
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::check_models,
            commands::download_models,
            commands::get_models_dir,
            commands::pick_audio_file,
            commands::pick_save_file,
            commands::pick_audio_save_file,
            commands::transcribe_file,
            commands::start_recording,
            commands::stop_recording,
            commands::cancel,
            commands::save_text_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
