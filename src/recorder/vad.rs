use std::sync::Mutex;

use webrtc_vad::{Vad, VadMode};

use crate::config::SAMPLE_RATE;

const FRAME_DURATION_MS: u32 = 30;
const FRAME_SIZE: usize = (SAMPLE_RATE as usize * FRAME_DURATION_MS as usize) / 1000;

pub struct VadDetector {
    vad: Mutex<Vad>,
}

impl VadDetector {
    pub fn new() -> Self {
        let vad = Vad::new(SAMPLE_RATE as i32).expect("failed to create VAD");
        Self { vad: Mutex::new(vad) }
    }

    pub fn set_mode(&self, mode: VadMode) {
        if let Ok(mut vad) = self.vad.lock() {
            let _ = vad.fvad_set_mode(mode);
        }
    }

    pub fn is_speech(&self, audio_f32: &[f32], threshold: f32) -> bool {
        let mut int16 = Vec::with_capacity(audio_f32.len());
        for &sample in audio_f32 {
            let clamped = sample.clamp(-1.0, 1.0);
            int16.push((clamped * 32768.0) as i16);
        }

        let total_frames = int16.len() / FRAME_SIZE;
        if total_frames == 0 {
            return false;
        }

        let mut vad = match self.vad.lock() {
            Ok(v) => v,
            Err(_) => return false,
        };

        let mut speech_frames = 0;
        for i in 0..total_frames {
            let start = i * FRAME_SIZE;
            let frame = &int16[start..start + FRAME_SIZE];
            if vad.is_voice_segment(frame).unwrap_or(false) {
                speech_frames += 1;
            }
        }

        (speech_frames as f32 / total_frames as f32) >= threshold
    }
}

impl Default for VadDetector {
    fn default() -> Self {
        Self::new()
    }
}

// Safe: VAD is always accessed through a Mutex, and the underlying webrtc_vad C library is reentrant.
unsafe impl Send for VadDetector {}
unsafe impl Sync for VadDetector {}
