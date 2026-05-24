use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use anyhow::Result;
use clap::Parser;

use rachaels_transcriber::config;
use rachaels_transcriber::download_models;
use rachaels_transcriber::engine::chunk;
use rachaels_transcriber::engine::whisper::{self, WhisperEngine};

#[derive(Parser)]
#[command(name = "rachaels-transcriber", version, about = "Local offline audio transcription")]
struct Cli {
    #[arg(short, long)]
    file: Option<PathBuf>,

    #[arg(short, long, default_value = "Fast")]
    model: String,

    #[arg(short, long)]
    output: Option<PathBuf>,

    #[arg(short, long)]
    gui: bool,
}

fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    match (cli.file, cli.gui) {
        (Some(path), _) => run_cli(path, &cli.model, cli.output)?,
        (None, true) | (None, false) => rachaels_transcriber::run(),
    }

    Ok(())
}

fn run_cli(file: PathBuf, model_label: &str, output: Option<PathBuf>) -> Result<()> {
    if !file.exists() {
        anyhow::bail!("file not found: {}", file.display());
    }

    let model_path = download_models::model_path(model_label);
    if !model_path.exists() {
        eprintln!("Model '{model_label}' not found. Downloading...");
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            download_models::download_model(model_label, None, Some(Box::new(|d, t| {
                let pct = if t > 0 { d as f64 / t as f64 * 100.0 } else { 0.0 };
                eprint!("\rDownloading... {pct:.0}%");
            })))
            .await
        })?;
        eprintln!("\nDownload complete.");
    }

    eprintln!("Loading model '{model_label}'...");
    let engine = WhisperEngine::new(&model_path, "en")?;

    eprintln!("Decoding audio...");
    let audio = chunk::prepare_audio(&file)?;
    let duration = audio.len() as f64 / config::SAMPLE_RATE as f64;
    eprintln!("Audio duration: {duration:.1}s");

    eprintln!("Transcribing...");
    let cancel = Arc::new(AtomicBool::new(false));
    let segments = engine.transcribe(
        &audio,
        Some(&cancel),
        Some(&|progress: f64| {
            eprint!("\rProgress: {:.0}%", progress * 100.0);
        }),
    )?;
    eprintln!();

    let text = whisper::segments_to_text(&segments);
    println!("{text}");

    if let Some(ref out_path) = output {
        std::fs::write(out_path, &text)?;
        eprintln!("Saved to {}", out_path.display());
    }

    Ok(())
}
