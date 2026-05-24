use std::fs::File;
use std::path::Path;

use anyhow::{Context, Result};
use rubato::{Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use crate::config::SAMPLE_RATE;

pub fn decode_audio(path: &Path) -> Result<(Vec<f32>, u32, u32)> {
    let src = File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let mss = MediaSourceStream::new(Box::new(src), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .context("failed to probe audio format")?;

    let mut format = probed.format;

    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .context("no audio track found")?;

    let track_id = track.id;
    let codec_params = &track.codec_params;
    let src_sample_rate = codec_params.sample_rate.unwrap_or(SAMPLE_RATE);
    let src_channels = codec_params
        .channels
        .map(|c| c.count() as u32)
        .unwrap_or(1);

    let mut decoder = symphonia::default::get_codecs()
        .make(&codec_params.clone(), &DecoderOptions::default())
        .context("failed to create audio decoder")?;

    let mut all_samples: Vec<f32> = Vec::new();

    loop {
        let packet = match format.next_packet() {
            Ok(pkt) => pkt,
            Err(SymphoniaError::IoError(ref e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(SymphoniaError::ResetRequired) => continue,
            Err(e) => return Err(anyhow::anyhow!("symphonia error: {e}")),
        };

        if packet.track_id() != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(decoded) => {
                let spec = *decoded.spec();
                let mut buf = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);
                buf.copy_interleaved_ref(decoded);
                all_samples.extend_from_slice(buf.samples());
            }
            Err(SymphoniaError::DecodeError(_)) => continue,
            Err(e) => return Err(anyhow::anyhow!("decode error: {e}")),
        }
    }

    Ok((all_samples, src_sample_rate, src_channels))
}

pub fn convert_to_mono(samples: &[f32], channels: u32) -> Vec<f32> {
    if channels == 1 {
        return samples.to_vec();
    }
    let frames = samples.len() / channels as usize;
    let mut mono = Vec::with_capacity(frames);
    for i in 0..frames {
        let mut sum = 0.0f32;
        for ch in 0..channels {
            sum += samples[i * channels as usize + ch as usize];
        }
        mono.push(sum / channels as f32);
    }
    mono
}

pub fn resample_to_16khz(samples: &[f32], from_rate: u32) -> Result<Vec<f32>> {
    if from_rate == SAMPLE_RATE {
        return Ok(samples.to_vec());
    }

    let params = SincInterpolationParameters {
        sinc_len: 256,
        f_cutoff: 0.95,
        interpolation: SincInterpolationType::Linear,
        oversampling_factor: 256,
        window: WindowFunction::Blackman2,
    };

    let mut resampler = SincFixedIn::<f32>::new(
        SAMPLE_RATE as f64 / from_rate as f64,
        1.0,
        params,
        samples.len(),
        1,
    )
    .map_err(|e| anyhow::anyhow!("resampler init failed: {e}"))?;

    let waves_in = vec![samples.to_vec()];
    let waves_out = resampler
        .process(&waves_in, None)
        .map_err(|e| anyhow::anyhow!("resampling failed: {e}"))?;

    Ok(waves_out.into_iter().next().unwrap_or_default())
}

pub fn prepare_audio(path: &Path) -> Result<Vec<f32>> {
    let (samples, sample_rate, channels) = decode_audio(path)?;
    let mono = convert_to_mono(&samples, channels);
    let pcm = resample_to_16khz(&mono, sample_rate)?;
    Ok(pcm)
}

#[allow(dead_code)]
pub fn duration_from_path(path: &Path) -> Result<f64> {
    let (samples, sample_rate, _channels) = decode_audio(path)?;
    let duration = samples.len() as f64 / sample_rate as f64;
    Ok(duration)
}

#[allow(dead_code)]
pub fn extract_chunk(audio: &[f32], start_sec: f64, duration_sec: f64) -> Vec<f32> {
    let sample_rate = SAMPLE_RATE as f64;
    let start_sample = (start_sec * sample_rate) as usize;
    let num_samples = (duration_sec * sample_rate) as usize;
    let end_sample = (start_sample + num_samples).min(audio.len());

    if start_sample >= audio.len() {
        return Vec::new();
    }

    audio[start_sample..end_sample].to_vec()
}
