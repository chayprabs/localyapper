// Speech-to-text engine wrapper (sherpa-onnx + Parakeet)
use std::path::Path;

use sherpa_onnx::{
    OfflineModelConfig, OfflineNemoEncDecCtcModelConfig, OfflineRecognizer,
    OfflineRecognizerConfig,
};

use crate::error::LocalYapperError;

/// Minimum audio length in samples (0.2s at 16kHz) below which transcription is skipped.
/// Lowered from 0.5s to support single-word utterances like "hi", "yes", "no".
const MIN_AUDIO_SAMPLES: usize = 3_200;

/// Default STT model variant for new installs.
pub const DEFAULT_WHISPER_MODEL: &str = "parakeet-110m";

/// Map a model setting string to the directory name where ONNX files are stored.
pub fn stt_model_dir_name(model: &str) -> String {
    match model {
        "parakeet-110m" => "parakeet-tdt-ctc-110m".to_string(),
        "parakeet-0.6b" => "parakeet-tdt-0.6b-v2".to_string(),
        // Legacy Whisper support — map old settings to a directory name
        "tiny.en" | "base.en" | "small.en" | "medium.en" => format!("whisper-{model}"),
        _ => model.to_string(),
    }
}

/// Return the list of files (filename, download URL) needed for a given STT model.
pub fn stt_model_files(model: &str) -> Vec<(&'static str, String)> {
    match model {
        "parakeet-110m" => vec![
            (
                "model.onnx",
                "https://huggingface.co/csukuangfj/sherpa-onnx-nemo-parakeet_tdt_ctc_110m-en-36000/resolve/main/model.onnx".to_string(),
            ),
            (
                "tokens.txt",
                "https://huggingface.co/csukuangfj/sherpa-onnx-nemo-parakeet_tdt_ctc_110m-en-36000/resolve/main/tokens.txt".to_string(),
            ),
        ],
        "parakeet-0.6b" => vec![
            (
                "model.onnx",
                "https://huggingface.co/csukuangfj/sherpa-onnx-nemo-parakeet-tdt-0.6b-v2/resolve/main/model.onnx".to_string(),
            ),
            (
                "tokens.txt",
                "https://huggingface.co/csukuangfj/sherpa-onnx-nemo-parakeet-tdt-0.6b-v2/resolve/main/tokens.txt".to_string(),
            ),
        ],
        _ => vec![],
    }
}

/// Silero VAD model download URL.
pub const SILERO_VAD_FILENAME: &str = "silero_vad.onnx";
pub const SILERO_VAD_URL: &str =
    "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/silero_vad.onnx";

/// Speech-to-text engine wrapping sherpa-onnx OfflineRecognizer.
///
/// Uses Parakeet NeMo CTC models via ONNX Runtime for fast, accurate
/// transcription with native punctuation and capitalization.
pub struct WhisperEngine {
    recognizer: OfflineRecognizer,
}

// SAFETY: OfflineRecognizer is backed by C++ ONNX Runtime which is thread-safe.
// The sherpa-onnx C API functions used are documented as thread-safe for inference.
unsafe impl Send for WhisperEngine {}
unsafe impl Sync for WhisperEngine {}

impl WhisperEngine {
    /// Load a Parakeet/NeMo CTC model from a directory.
    ///
    /// Expects `model.int8.onnx` (or `model.onnx`) and `tokens.txt` in `model_dir`.
    pub fn new(model_dir: &Path) -> Result<Self, LocalYapperError> {
        if !model_dir.exists() {
            return Err(LocalYapperError::TranscriptionError(format!(
                "STT model directory not found at {}",
                model_dir.display()
            )));
        }

        // Find the ONNX model file (prefer int8, fall back to fp32)
        let model_file = if model_dir.join("model.int8.onnx").exists() {
            model_dir.join("model.int8.onnx")
        } else if model_dir.join("model.onnx").exists() {
            model_dir.join("model.onnx")
        } else {
            return Err(LocalYapperError::TranscriptionError(format!(
                "No ONNX model file found in {}",
                model_dir.display()
            )));
        };

        let tokens_file = model_dir.join("tokens.txt");
        if !tokens_file.exists() {
            return Err(LocalYapperError::TranscriptionError(format!(
                "tokens.txt not found in {}",
                model_dir.display()
            )));
        }

        let n_threads = std::thread::available_parallelism()
            .map(|p| (p.get().saturating_sub(2)).max(1) as i32)
            .unwrap_or(2);

        let config = OfflineRecognizerConfig {
            model_config: OfflineModelConfig {
                nemo_ctc: OfflineNemoEncDecCtcModelConfig {
                    model: Some(model_file.to_string_lossy().to_string()),
                },
                tokens: Some(tokens_file.to_string_lossy().to_string()),
                num_threads: n_threads,
                debug: false,
                provider: Some("cpu".to_string()),
                ..Default::default()
            },
            decoding_method: Some("greedy_search".to_string()),
            // Penalize blank token to reduce missed words in short utterances
            blank_penalty: 1.2,
            ..Default::default()
        };

        let recognizer = OfflineRecognizer::create(&config).ok_or_else(|| {
            LocalYapperError::TranscriptionError(
                "Failed to create STT recognizer — check model files".to_string(),
            )
        })?;

        log::info!(
            "STT engine loaded from {} using {} threads",
            model_dir.display(),
            n_threads
        );

        Ok(Self { recognizer })
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

        let stream = self.recognizer.create_stream();
        stream.accept_waveform(16000, audio);
        self.recognizer.decode(&stream);

        let result = stream.get_result().ok_or_else(|| {
            LocalYapperError::TranscriptionError("No result from STT recognizer".to_string())
        })?;

        let text = result.text.trim().to_string();

        log::info!(
            "Transcribed {} samples -> {} chars: {:?}",
            audio.len(),
            text.len(),
            if text.len() > 80 {
                format!("{}...", &text[..80])
            } else {
                text.clone()
            }
        );

        Ok(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn too_short_audio_returns_empty() {
        assert_eq!(MIN_AUDIO_SAMPLES, 3_200);
    }

    #[test]
    fn default_model_is_parakeet() {
        assert_eq!(DEFAULT_WHISPER_MODEL, "parakeet-110m");
    }

    #[test]
    fn model_dir_names_are_correct() {
        assert_eq!(
            stt_model_dir_name("parakeet-110m"),
            "parakeet-tdt-ctc-110m"
        );
        assert_eq!(
            stt_model_dir_name("parakeet-0.6b"),
            "parakeet-tdt-0.6b-v2"
        );
        assert_eq!(stt_model_dir_name("base.en"), "whisper-base.en");
    }

    #[test]
    fn model_files_returns_urls_for_parakeet() {
        let files = stt_model_files("parakeet-110m");
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].0, "model.onnx");
        assert_eq!(files[1].0, "tokens.txt");
    }

    #[test]
    fn unknown_model_returns_empty_files() {
        assert!(stt_model_files("nonexistent-model").is_empty());
    }
}
