// Voice activity detection — Silero VAD (primary) with energy-based fallback
use std::path::Path;

use sherpa_onnx::{SileroVadModelConfig, VadModelConfig, VoiceActivityDetector};

use crate::error::LocalYapperError;

/// Result of applying VAD to an audio buffer.
#[derive(Clone, Debug)]
pub struct VadResult {
    /// Audio trimmed of leading/trailing silence.
    pub trimmed_audio: Vec<f32>,
    /// Number of frames classified as speech.
    pub speech_frame_count: usize,
    /// Duration of speech in milliseconds.
    pub speech_duration_ms: u64,
    /// Whether meaningful speech was detected.
    pub has_speech: bool,
}

/// Silero VAD engine wrapping sherpa-onnx VoiceActivityDetector.
pub struct SileroVad {
    config: VadModelConfig,
}

// SAFETY: VadModelConfig is just data (no pointers). VoiceActivityDetector is created
// fresh per call in process(). The config is safe to share across threads.
unsafe impl Send for SileroVad {}
unsafe impl Sync for SileroVad {}

impl SileroVad {
    /// Load the Silero VAD model from disk.
    pub fn new(model_path: &Path) -> Result<Self, LocalYapperError> {
        if !model_path.exists() {
            return Err(LocalYapperError::AudioError(format!(
                "Silero VAD model not found at {}",
                model_path.display()
            )));
        }

        let config = VadModelConfig {
            silero_vad: SileroVadModelConfig {
                model: Some(model_path.to_string_lossy().to_string()),
                threshold: 0.5,
                min_silence_duration: 0.5,
                min_speech_duration: 0.1,
                max_speech_duration: 120.0,
                ..Default::default()
            },
            sample_rate: 16000,
            num_threads: 1,
            provider: Some("cpu".to_string()),
            debug: false,
            ..Default::default()
        };

        // Validate config by creating a test detector
        let _test = VoiceActivityDetector::create(&config, 0.5).ok_or_else(|| {
            LocalYapperError::AudioError("Failed to initialize Silero VAD".to_string())
        })?;

        log::info!("Silero VAD loaded from {}", model_path.display());
        Ok(Self { config })
    }

    /// Process audio and return VAD result with trimmed speech segments.
    pub fn process(&self, audio: &[f32]) -> VadResult {
        if audio.is_empty() {
            return VadResult {
                trimmed_audio: Vec::new(),
                speech_frame_count: 0,
                speech_duration_ms: 0,
                has_speech: false,
            };
        }

        // Create a fresh detector for each call (stateful, not reusable across calls)
        let detector = match VoiceActivityDetector::create(&self.config, 0.5) {
            Some(d) => d,
            None => {
                log::warn!("Failed to create VAD detector, falling back to energy");
                return apply_energy_vad(audio);
            }
        };

        // Feed audio in 512-sample chunks (required by Silero VAD)
        // Pad the last chunk with zeros to avoid dropping trailing audio
        let chunk_size = 512;
        for chunk in audio.chunks(chunk_size) {
            if chunk.len() == chunk_size {
                detector.accept_waveform(chunk);
            } else {
                // Pad the final partial chunk with silence
                let mut padded = vec![0.0f32; chunk_size];
                padded[..chunk.len()].copy_from_slice(chunk);
                detector.accept_waveform(&padded);
            }
        }

        // Flush remaining audio
        detector.flush();

        // Collect speech segments
        let mut speech_samples: Vec<f32> = Vec::new();
        let mut segment_count = 0;

        while !detector.is_empty() {
            if let Some(segment) = detector.front() {
                speech_samples.extend_from_slice(segment.samples());
                segment_count += 1;
            }
            detector.pop();
        }

        if speech_samples.is_empty() {
            return VadResult {
                trimmed_audio: Vec::new(),
                speech_frame_count: 0,
                speech_duration_ms: 0,
                has_speech: false,
            };
        }

        let speech_duration_ms = (speech_samples.len() as u64 * 1000) / 16_000;

        VadResult {
            trimmed_audio: speech_samples,
            speech_frame_count: segment_count,
            speech_duration_ms,
            has_speech: true,
        }
    }
}

// ============================================================================
// Energy-based VAD fallback (used when Silero model not available)
// ============================================================================

/// Energy-based voice activity detection configuration.
#[derive(Clone, Debug)]
pub struct VadConfig {
    /// RMS energy threshold to classify a frame as speech.
    pub energy_threshold: f32,
    /// Number of samples per analysis frame (480 = 30ms at 16kHz).
    pub frame_size: usize,
    /// Minimum consecutive speech frames to confirm speech presence.
    pub min_speech_frames: usize,
    /// Frames to keep after last speech frame (hangover smoothing).
    pub hangover_frames: usize,
}

/// Returns the default energy-based VAD configuration.
pub fn default_config() -> VadConfig {
    VadConfig {
        energy_threshold: 0.01,
        frame_size: 480,
        min_speech_frames: 3,
        hangover_frames: 10,
    }
}

/// Compute RMS energy for a frame of audio samples.
pub fn compute_rms(frame: &[f32]) -> f32 {
    if frame.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = frame.iter().map(|&s| s * s).sum();
    (sum_sq / frame.len() as f32).sqrt()
}

/// Classify each frame as speech (true) or silence (false).
pub fn classify_frames(audio: &[f32], config: &VadConfig) -> Vec<bool> {
    audio
        .chunks(config.frame_size)
        .map(|frame| compute_rms(frame) >= config.energy_threshold)
        .collect()
}

/// Apply energy-based VAD with hangover smoothing and trim leading/trailing silence.
pub fn apply_energy_vad(audio: &[f32]) -> VadResult {
    let config = default_config();

    if audio.is_empty() {
        return VadResult {
            trimmed_audio: Vec::new(),
            speech_frame_count: 0,
            speech_duration_ms: 0,
            has_speech: false,
        };
    }

    let raw_flags = classify_frames(audio, &config);
    let speech_frame_count = raw_flags.iter().filter(|&&f| f).count();

    if speech_frame_count < config.min_speech_frames {
        return VadResult {
            trimmed_audio: Vec::new(),
            speech_frame_count,
            speech_duration_ms: 0,
            has_speech: false,
        };
    }

    // Apply hangover smoothing: extend speech regions by hangover_frames
    let mut smoothed = raw_flags.clone();
    let mut hangover_remaining: usize = 0;
    for flag in smoothed.iter_mut() {
        if *flag {
            hangover_remaining = config.hangover_frames;
        } else if hangover_remaining > 0 {
            *flag = true;
            hangover_remaining -= 1;
        }
    }

    // Find first and last speech frame
    let first_speech = smoothed.iter().position(|&f| f).unwrap_or(0);
    let last_speech = smoothed.len()
        - 1
        - smoothed.iter().rev().position(|&f| f).unwrap_or(0);

    let start_sample = first_speech * config.frame_size;
    let end_sample = ((last_speech + 1) * config.frame_size).min(audio.len());

    let trimmed_audio = audio[start_sample..end_sample].to_vec();
    let speech_duration_ms =
        (trimmed_audio.len() as u64 * 1000) / 16_000;

    VadResult {
        trimmed_audio,
        speech_frame_count,
        speech_duration_ms,
        has_speech: true,
    }
}

/// Top-level VAD: uses Silero if available, falls back to energy-based.
pub fn apply_vad(audio: &[f32], silero: Option<&SileroVad>) -> VadResult {
    match silero {
        Some(vad) => {
            println!("VAD: Using Silero VAD");
            vad.process(audio)
        }
        None => {
            println!("VAD: Using energy-based fallback");
            apply_energy_vad(audio)
        }
    }
}

/// Quick check for whether audio contains speech (energy-based only).
pub fn has_speech(audio: &[f32]) -> bool {
    let config = default_config();
    let flags = classify_frames(audio, &config);
    flags.iter().filter(|&&f| f).count() >= config.min_speech_frames
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rms_of_silence_is_zero() {
        let silence = vec![0.0f32; 480];
        assert_eq!(compute_rms(&silence), 0.0);
    }

    #[test]
    fn rms_of_signal_is_nonzero() {
        let signal: Vec<f32> = (0..480).map(|i| (i as f32 * 0.1).sin() * 0.5).collect();
        let rms = compute_rms(&signal);
        assert!(rms > 0.0, "RMS should be positive for non-silent signal");
    }

    #[test]
    fn rms_of_empty_is_zero() {
        assert_eq!(compute_rms(&[]), 0.0);
    }

    #[test]
    fn energy_vad_on_silence_returns_no_speech() {
        let silence = vec![0.0f32; 16_000];
        let result = apply_energy_vad(&silence);
        assert!(!result.has_speech);
        assert!(result.trimmed_audio.is_empty());
    }

    #[test]
    fn energy_vad_on_speech_returns_speech() {
        let mut audio = vec![0.0f32; 8_000];
        let speech: Vec<f32> = (0..16_000)
            .map(|i| (i as f32 * 0.05).sin() * 0.5)
            .collect();
        audio.extend_from_slice(&speech);
        audio.extend_from_slice(&vec![0.0f32; 8_000]);

        let result = apply_energy_vad(&audio);
        assert!(result.has_speech);
        assert!(result.speech_frame_count > 0);
        assert!(result.speech_duration_ms > 0);
        assert!(result.trimmed_audio.len() < audio.len());
        assert!(result.trimmed_audio.len() >= 16_000);
    }

    #[test]
    fn energy_vad_on_empty_returns_no_speech() {
        let result = apply_energy_vad(&[]);
        assert!(!result.has_speech);
        assert!(result.trimmed_audio.is_empty());
    }

    #[test]
    fn has_speech_quick_check() {
        let silence = vec![0.0f32; 16_000];
        assert!(!has_speech(&silence));

        let signal: Vec<f32> = (0..16_000)
            .map(|i| (i as f32 * 0.05).sin() * 0.5)
            .collect();
        assert!(has_speech(&signal));
    }

    #[test]
    fn apply_vad_without_silero_uses_energy() {
        let silence = vec![0.0f32; 16_000];
        let result = apply_vad(&silence, None);
        assert!(!result.has_speech);
    }

    #[test]
    fn classify_frames_correct_count() {
        let config = default_config();
        let audio = vec![0.0f32; 960];
        let flags = classify_frames(&audio, &config);
        assert_eq!(flags.len(), 2);
        assert!(flags.iter().all(|&f| !f));
    }
}
