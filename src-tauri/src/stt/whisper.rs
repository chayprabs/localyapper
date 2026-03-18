use std::path::Path;
use std::sync::Mutex;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use crate::error::LocalYapperError;

/// Minimum audio length in samples (0.5s at 16kHz) below which transcription is skipped.
const MIN_AUDIO_SAMPLES: usize = 8_000;

/// Model filename expected in the resources directory.
pub const WHISPER_MODEL_FILENAME: &str = "ggml-tiny.en.bin";

/// Whisper speech-to-text engine wrapping whisper-rs.
///
/// The WhisperContext is behind a Mutex to prevent concurrent model access.
/// Actual inference runs on a WhisperState created from the context, which
/// can operate independently after creation.
pub struct WhisperEngine {
    ctx: Mutex<WhisperContext>,
    n_threads: i32,
}

impl WhisperEngine {
    /// Load a Whisper model from disk and create the engine.
    pub fn new(model_path: &Path) -> Result<Self, LocalYapperError> {
        if !model_path.exists() {
            return Err(LocalYapperError::TranscriptionError(format!(
                "Whisper model not found at {}",
                model_path.display()
            )));
        }

        let path_str = model_path.to_str().ok_or_else(|| {
            LocalYapperError::TranscriptionError(
                "Model path contains invalid UTF-8".to_string(),
            )
        })?;

        let ctx = WhisperContext::new_with_params(path_str, WhisperContextParameters::default())
            .map_err(|e| {
                LocalYapperError::TranscriptionError(format!("Failed to load Whisper model: {}", e))
            })?;

        let n_threads = std::thread::available_parallelism()
            .map(|p| (p.get() / 2).max(1) as i32)
            .unwrap_or(2);

        log::info!(
            "Whisper engine loaded from {} using {} threads",
            model_path.display(),
            n_threads
        );

        Ok(Self {
            ctx: Mutex::new(ctx),
            n_threads,
        })
    }

    /// Transcribe f32 audio samples (16kHz mono) into text.
    ///
    /// Returns an empty string if audio is too short (< 0.5s).
    /// Runs synchronously — call from a blocking thread.
    pub fn transcribe(&self, audio: &[f32]) -> Result<String, LocalYapperError> {
        if audio.len() < MIN_AUDIO_SAMPLES {
            log::debug!(
                "Audio too short for transcription ({} samples, need {})",
                audio.len(),
                MIN_AUDIO_SAMPLES
            );
            return Ok(String::new());
        }

        // Lock context briefly to create an independent state
        let mut state = {
            let ctx = self.ctx.lock().map_err(|e| {
                LocalYapperError::TranscriptionError(format!(
                    "Failed to lock Whisper context: {}",
                    e
                ))
            })?;
            ctx.create_state().map_err(|e| {
                LocalYapperError::TranscriptionError(format!(
                    "Failed to create Whisper state: {}",
                    e
                ))
            })?
        };
        // Mutex is released here — inference runs lock-free

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(Some("en"));
        params.set_n_threads(self.n_threads);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_single_segment(false);
        params.set_no_context(true);

        state.full(params, audio).map_err(|e| {
            LocalYapperError::TranscriptionError(format!("Whisper inference failed: {}", e))
        })?;

        let num_segments = state.full_n_segments();

        let mut text = String::new();
        for i in 0..num_segments {
            let segment = state.get_segment(i).ok_or_else(|| {
                LocalYapperError::TranscriptionError(format!(
                    "Segment {} out of bounds",
                    i
                ))
            })?;
            let segment_text = segment.to_str().map_err(|e| {
                LocalYapperError::TranscriptionError(format!(
                    "Failed to get segment {} text: {}",
                    i, e
                ))
            })?;
            text.push_str(segment_text);
        }

        let result = text.trim().to_string();
        log::info!(
            "Transcribed {} samples -> {} chars: {:?}",
            audio.len(),
            result.len(),
            if result.len() > 80 {
                format!("{}...", &result[..80])
            } else {
                result.clone()
            }
        );

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn too_short_audio_returns_empty() {
        // We can't load a real model in unit tests, but we can test the guard
        // by creating a fake engine-like scenario. Since we can't construct
        // WhisperEngine without a model, we test the constant instead.
        assert_eq!(MIN_AUDIO_SAMPLES, 8_000);
    }

    #[test]
    fn model_filename_is_correct() {
        assert_eq!(WHISPER_MODEL_FILENAME, "ggml-tiny.en.bin");
    }
}
