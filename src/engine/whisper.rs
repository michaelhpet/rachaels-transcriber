use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use anyhow::{Context, Result};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

pub struct WhisperEngine {
    ctx: WhisperContext,
    language: String,
}

#[derive(Clone, Debug)]
pub struct Segment {
    pub start: f64,
    pub end: f64,
    pub text: String,
}

impl WhisperEngine {
    pub fn new(model_path: &Path, language: &str) -> Result<Self> {
        let params = WhisperContextParameters::default();
        let ctx = WhisperContext::new_with_params(
            model_path.to_str().context("invalid model path")?,
            params,
        )
        .context("failed to load whisper model")?;
        Ok(Self {
            ctx,
            language: language.to_string(),
        })
    }

    pub fn transcribe(
        &self,
        audio: &[f32],
        cancel: Option<&AtomicBool>,
        progress: Option<&dyn Fn(f64)>,
    ) -> Result<Vec<Segment>> {
        let mut state = self
            .ctx
            .create_state()
            .context("failed to create whisper state")?;
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_n_threads(4);
        params.set_language(Some(&self.language));
        params.set_no_timestamps(false);
        params.set_suppress_non_speech_tokens(true);

        state
            .full(params, audio)
            .context("whisper inference failed")?;

        let n_segments = state
            .full_n_segments()
            .context("failed to get segment count")?;
        let total_dur = audio.len() as f64 / crate::config::SAMPLE_RATE as f64;

        let mut segments = Vec::with_capacity(n_segments as usize);
        for i in 0..n_segments {
            if let Some(ref c) = cancel {
                if c.load(Ordering::Relaxed) {
                    break;
                }
            }

            let text = state
                .full_get_segment_text(i)
                .context("failed to get segment text")?;
            let t0_ms = state
                .full_get_segment_t0(i)
                .context("failed to get segment start")?;
            let t1_ms = state
                .full_get_segment_t1(i)
                .context("failed to get segment end")?;

            segments.push(Segment {
                start: t0_ms as f64 / 100.0,
                end: t1_ms as f64 / 100.0,
                text: text.trim().to_string(),
            });

            if let Some(ref p) = progress {
                let pct = (t1_ms as f64 / 100.0) / total_dur;
                p(pct.min(0.99));
            }
        }

        Ok(segments)
    }

    pub fn transcribe_chunked(
        &self,
        audio: Vec<f32>,
        chunk_sec: f64,
        overlap_sec: f64,
        cancel: Option<&AtomicBool>,
        progress: Option<&dyn Fn(f64, &str)>,
    ) -> Result<Vec<Segment>> {
        let sample_rate = crate::config::SAMPLE_RATE as f64;
        let chunk_samples = (chunk_sec * sample_rate) as usize;
        let overlap_samples = (overlap_sec * sample_rate) as usize;
        let total_samples = audio.len();
        let mut all_segments: Vec<Segment> = Vec::new();
        let mut pos = 0usize;
        let mut chunk_idx = 0;

        while pos < total_samples {
            if let Some(ref c) = cancel {
                if c.load(Ordering::Relaxed) {
                    break;
                }
            }

            let end = (pos + chunk_samples).min(total_samples);
            let chunk = &audio[pos..end];
            if chunk.len() < sample_rate as usize / 2 {
                break;
            }

            let segments = self.transcribe(chunk, cancel, None)?;

            if chunk_idx > 0 {
                let overlap_start = chunk_sec - overlap_sec;
                let kept: Vec<Segment> = segments
                    .into_iter()
                    .filter(|s| s.end > overlap_start)
                    .collect();
                all_segments.extend(kept);
            } else {
                all_segments.extend(segments);
            }

            pos = pos.saturating_sub(overlap_samples) + chunk_samples;

            if let Some(ref p) = progress {
                let overall = pos.min(total_samples) as f64 / total_samples as f64;
                p(overall.min(0.99), "");
            }

            chunk_idx += 1;
        }

        Ok(all_segments)
    }
}

pub fn segments_to_text(segments: &[Segment]) -> String {
    segments
        .iter()
        .map(|s| s.text.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}
