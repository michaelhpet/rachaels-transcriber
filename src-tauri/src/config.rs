use std::path::PathBuf;
use std::sync::OnceLock;

pub const APP_NAME: &str = "RachaelsTranscriber";
pub const MODEL_ACCURATE: &str = "ggml-small.en.bin";
pub const MODEL_FAST: &str = "ggml-base.en.bin";
pub const SAMPLE_RATE: u32 = 16000;
pub const CHUNK_SEC: f64 = 30.0;
pub const CHUNK_OVERLAP_SEC: f64 = 2.0;
pub const WINDOW_SEC: f64 = 4.0;
pub const TICK_SEC: f64 = 2.0;
pub const MAX_BUFFER_SEC: f64 = 30.0;

static MODELS_DIR: OnceLock<PathBuf> = OnceLock::new();

pub fn set_models_dir(path: PathBuf) {
    MODELS_DIR.set(path).ok();
}

pub fn models_dir() -> PathBuf {
    MODELS_DIR
        .get()
        .cloned()
        .unwrap_or_else(|| PathBuf::from("models"))
}

#[allow(dead_code)]
pub fn supported_extensions() -> &'static [&'static str] {
    &["mp3", "wav", "m4a", "flac", "ogg", "aac", "wma"]
}
