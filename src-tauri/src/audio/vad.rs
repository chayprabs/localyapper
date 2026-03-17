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

/// Returns the default VAD configuration.
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

/// Apply VAD with hangover smoothing and trim leading/trailing silence.
pub fn apply_vad(audio: &[f32], config: &VadConfig) -> VadResult {
    if audio.is_empty() {
        return VadResult {
            trimmed_audio: Vec::new(),
            speech_frame_count: 0,
            speech_duration_ms: 0,
            has_speech: false,
        };
    }

    let raw_flags = classify_frames(audio, config);
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

/// Quick check for whether audio contains speech.
pub fn has_speech(audio: &[f32], config: &VadConfig) -> bool {
    let flags = classify_frames(audio, config);
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
    fn vad_on_silence_returns_no_speech() {
        let silence = vec![0.0f32; 16_000]; // 1 second of silence
        let config = default_config();
        let result = apply_vad(&silence, &config);
        assert!(!result.has_speech);
        assert!(result.trimmed_audio.is_empty());
    }

    #[test]
    fn vad_on_speech_returns_speech() {
        // Simulate: 0.5s silence + 1s speech + 0.5s silence
        let mut audio = vec![0.0f32; 8_000]; // 0.5s silence
        let speech: Vec<f32> = (0..16_000)
            .map(|i| (i as f32 * 0.05).sin() * 0.5)
            .collect();
        audio.extend_from_slice(&speech);
        audio.extend_from_slice(&vec![0.0f32; 8_000]); // 0.5s silence

        let config = default_config();
        let result = apply_vad(&audio, &config);
        assert!(result.has_speech);
        assert!(result.speech_frame_count > 0);
        assert!(result.speech_duration_ms > 0);
        // Trimmed audio should be shorter than original
        assert!(result.trimmed_audio.len() < audio.len());
        // But should contain the speech portion
        assert!(result.trimmed_audio.len() >= 16_000);
    }

    #[test]
    fn vad_on_empty_returns_no_speech() {
        let config = default_config();
        let result = apply_vad(&[], &config);
        assert!(!result.has_speech);
        assert!(result.trimmed_audio.is_empty());
    }

    #[test]
    fn has_speech_quick_check() {
        let silence = vec![0.0f32; 16_000];
        let config = default_config();
        assert!(!has_speech(&silence, &config));

        let signal: Vec<f32> = (0..16_000)
            .map(|i| (i as f32 * 0.05).sin() * 0.5)
            .collect();
        assert!(has_speech(&signal, &config));
    }

    #[test]
    fn classify_frames_correct_count() {
        let audio = vec![0.0f32; 960]; // 2 frames of 480
        let config = default_config();
        let flags = classify_frames(&audio, &config);
        assert_eq!(flags.len(), 2);
        assert!(flags.iter().all(|&f| !f));
    }

    #[test]
    fn min_speech_frames_threshold() {
        // Create audio with only 1-2 speech frames (below min_speech_frames=3)
        let mut audio = vec![0.0f32; 480 * 10]; // 10 frames of silence
        // Add a tiny blip in 2 frames
        for sample in audio[0..960].iter_mut() {
            *sample = 0.5;
        }
        let config = default_config();
        let result = apply_vad(&audio, &config);
        assert!(!result.has_speech);
    }
}
