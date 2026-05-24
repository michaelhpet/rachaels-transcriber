use std::path::PathBuf;
use directories::ProjectDirs;

pub const APP_NAME: &str = "RachaelsTranscriber";
pub const MODEL_ACCURATE: &str = "ggml-small.en.bin";
pub const MODEL_FAST: &str = "ggml-base.en.bin";
pub const SAMPLE_RATE: u32 = 16000;
pub const CHUNK_SEC: f64 = 30.0;
pub const CHUNK_OVERLAP_SEC: f64 = 2.0;
pub const WINDOW_SEC: f64 = 4.0;
pub const TICK_SEC: f64 = 2.0;
pub const MAX_BUFFER_SEC: f64 = 30.0;

pub fn get_persistent_base() -> PathBuf {
    if let Some(proj_dirs) = ProjectDirs::from("com", "rachaels", APP_NAME) {
        proj_dirs.data_local_dir().to_path_buf()
    } else {
        PathBuf::from(".")
    }
}

pub fn models_dir() -> PathBuf {
    get_persistent_base().join("models")
}

pub fn supported_extensions() -> &'static [&'static str] {
    &["mp3", "wav", "m4a", "flac", "ogg", "aac", "wma"]
}
