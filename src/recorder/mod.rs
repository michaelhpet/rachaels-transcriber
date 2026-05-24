pub mod vad;

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use parking_lot::Mutex;

use crate::config::{SAMPLE_RATE, MAX_BUFFER_SEC};
use crate::engine::chunk::resample_to_16khz;

pub struct AudioRecorder {
    is_recording: Arc<AtomicBool>,
    buffer: Arc<Mutex<VecDeque<f32>>>,
    stream: Option<cpal::Stream>,
    start_time: Option<Instant>,
    raw_audio: Arc<Mutex<Vec<f32>>>,
    actual_sample_rate: u32,
    _actual_channels: u16,
    vad_detector: vad::VadDetector,
}

impl AudioRecorder {
    pub fn new() -> Self {
        Self {
            is_recording: Arc::new(AtomicBool::new(false)),
            buffer: Arc::new(Mutex::new(VecDeque::with_capacity(
                (SAMPLE_RATE as f64 * MAX_BUFFER_SEC) as usize,
            ))),
            stream: None,
            start_time: None,
            raw_audio: Arc::new(Mutex::new(Vec::new())),
            actual_sample_rate: SAMPLE_RATE,
            _actual_channels: 1,
            vad_detector: vad::VadDetector::new(),
        }
    }

    pub fn start(&mut self, _wav_path: Option<&std::path::Path>) -> Result<()> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| anyhow::anyhow!("no input device found"))?;

        let config = device
            .default_input_config()
            .map_err(|e| anyhow::anyhow!("failed to get input config: {e}"))?;

        let sample_rate = config.sample_rate().0;
        let channels = config.channels() as usize;

        self.actual_sample_rate = sample_rate;
        self._actual_channels = config.channels();

        let is_recording = self.is_recording.clone();
        let buffer = self.buffer.clone();
        let raw_audio = self.raw_audio.clone();
        let max_samples = (SAMPLE_RATE as f64 * MAX_BUFFER_SEC) as usize;

        let stream = device.build_input_stream(
            &config.into(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                if !is_recording.load(Ordering::Relaxed) {
                    return;
                }
                let mut buf = buffer.lock();
                for &sample in data {
                    if buf.len() >= max_samples {
                        buf.pop_front();
                    }
                    buf.push_back(sample);
                }
                let mut raw = raw_audio.lock();
                raw.extend_from_slice(data);
            },
            move |err| {
                log::error!("audio stream error: {err}");
            },
            None,
        )?;

        stream.play()?;
        self.stream = Some(stream);
        self.is_recording.store(true, Ordering::Relaxed);
        self.start_time = Some(Instant::now());

        Ok(())
    }

    pub fn stop(&mut self) -> Option<Vec<f32>> {
        self.is_recording.store(false, Ordering::Relaxed);
        // Drop the stream to stop audio capture
        self.stream = None;
        self.start_time = None;

        let raw = self.raw_audio.lock().clone();
        if raw.is_empty() {
            return None;
        }

        let mono = if self._actual_channels > 1 {
            let frames = raw.len() / self._actual_channels as usize;
            let mut m = Vec::with_capacity(frames);
            for i in 0..frames {
                let mut sum = 0.0f32;
                for ch in 0..self._actual_channels as usize {
                    sum += raw[i * self._actual_channels as usize + ch];
                }
                m.push(sum / self._actual_channels as f32);
            }
            m
        } else {
            raw
        };

        if self.actual_sample_rate != SAMPLE_RATE {
            Some(resample_to_16khz(&mono, self.actual_sample_rate).unwrap_or(mono))
        } else {
            Some(mono)
        }
    }

    pub fn elapsed(&self) -> f64 {
        self.start_time
            .map(|t| t.elapsed().as_secs_f64())
            .unwrap_or(0.0)
    }

    pub fn get_buffer(&self, seconds: f64) -> Option<Vec<f32>> {
        let n = (SAMPLE_RATE as f64 * seconds) as usize;
        let buf = self.buffer.lock();
        if buf.len() < n {
            return None;
        }
        let data: Vec<f32> = buf.iter().rev().take(n).cloned().collect::<Vec<_>>();
        drop(buf);
        let mut result = data;
        result.reverse();
        Some(result)
    }

    pub fn has_speech(&self, audio: &[f32], threshold: f32) -> bool {
        self.vad_detector.is_speech(audio, threshold)
    }

    pub fn is_recording(&self) -> bool {
        self.is_recording.load(Ordering::Relaxed)
    }
}

impl Default for AudioRecorder {
    fn default() -> Self {
        Self::new()
    }
}

// Safe: AudioRecorder is only used from one thread at a time.
// The cpal stream on macOS is backed by thread-safe CoreAudio.
// VAD is protected by a Mutex.
unsafe impl Send for AudioRecorder {}
