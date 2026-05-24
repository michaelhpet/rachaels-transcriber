use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::{Context, Result};
use futures_util::StreamExt;

use crate::config;
use crate::config::{MODEL_ACCURATE, MODEL_FAST};

pub struct ModelInfo {
    #[allow(dead_code)]
    pub name: &'static str,
    pub url: &'static str,
    pub size_mb: u64,
}

pub const ACCURATE_MODEL: ModelInfo = ModelInfo {
    name: "Accurate",
    url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.en.bin",
    size_mb: 465,
};

pub const FAST_MODEL: ModelInfo = ModelInfo {
    name: "Fast",
    url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin",
    size_mb: 141,
};

pub fn model_filename(label: &str) -> &'static str {
    match label {
        "Accurate" => MODEL_ACCURATE,
        "Fast" => MODEL_FAST,
        _ => MODEL_FAST,
    }
}

pub fn model_path(label: &str) -> PathBuf {
    config::models_dir().join(model_filename(label))
}

pub fn is_model_downloaded(label: &str) -> bool {
    model_path(label).exists()
}

pub fn check_missing_models() -> Vec<&'static str> {
    let mut missing = Vec::new();
    if !is_model_downloaded("Accurate") {
        missing.push("Accurate");
    }
    if !is_model_downloaded("Fast") {
        missing.push("Fast");
    }
    missing
}

pub async fn download_model(
    label: &str,
    cancel: Option<&AtomicBool>,
    progress: Option<Box<dyn Fn(u64, u64) + Send>>,
) -> Result<PathBuf> {
    let info = match label {
        "Accurate" => &ACCURATE_MODEL,
        "Fast" => &FAST_MODEL,
        _ => anyhow::bail!("unknown model: {label}"),
    };

    let dest_dir = config::models_dir();
    tokio::fs::create_dir_all(&dest_dir)
        .await
        .context("failed to create models directory")?;

    let dest_path = dest_dir.join(model_filename(label));
    let temp_path = dest_path.with_extension("tmp");

    // Check if already downloaded
    if dest_path.exists() {
        return Ok(dest_path);
    }

    let client = reqwest::Client::builder()
        .user_agent("RachaelsTranscriber/0.1")
        .build()?;

    let response = client
        .get(info.url)
        .send()
        .await
        .context("failed to send download request")?;

    let total_size = response
        .content_length()
        .unwrap_or(info.size_mb * 1024 * 1024);

    let mut file = tokio::fs::File::create(&temp_path)
        .await
        .context("failed to create temp file")?;

    let mut downloaded: u64 = 0;
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        if let Some(ref c) = cancel {
            if c.load(Ordering::Relaxed) {
                tokio::fs::remove_file(&temp_path).await.ok();
                anyhow::bail!("download cancelled");
            }
        }

        let chunk = chunk.context("failed to read download chunk")?;
        tokio::io::AsyncWriteExt::write_all(&mut file, &chunk)
            .await
            .context("failed to write to temp file")?;

        downloaded += chunk.len() as u64;
        if let Some(ref p) = progress {
            p(downloaded, total_size);
        }
    }

    tokio::fs::rename(&temp_path, &dest_path)
        .await
        .context("failed to rename temp file")?;

    Ok(dest_path)
}
