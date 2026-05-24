pub mod commands;
pub mod config;
pub mod download_models;
pub mod engine;
pub mod recorder;

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use parking_lot::Mutex;

use engine::whisper::WhisperEngine;
use recorder::AudioRecorder;

pub struct AppState {
    pub cancel_flag: Arc<AtomicBool>,
    pub engine: Mutex<Option<WhisperEngine>>,
    pub recorder: Mutex<Option<AudioRecorder>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            cancel_flag: Arc::new(AtomicBool::new(false)),
            engine: Mutex::new(None),
            recorder: Mutex::new(None),
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::check_models,
            commands::download_model,
            commands::get_models_dir,
            commands::pick_audio_file,
            commands::pick_save_file,
            commands::transcribe_file,
            commands::start_recording,
            commands::stop_recording,
            commands::cancel,
            commands::save_text_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
