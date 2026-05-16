// Speech-to-text engine wrapper (sherpa-onnx + Parakeet)
use std::path::Path;

use sherpa_onnx::{
    OfflineModelConfig, OfflineNemoEncDecCtcModelConfig, OfflineRecognizer, OfflineRecognizerConfig,
};

use crate::error::LocalYapperError;

/// Minimum audio length in samples (0.2s at 16kHz) below which transcription is skipped.
/// Lowered from 0.5s to support single-word utterances like "hi", "yes", "no".
const MIN_AUDIO_SAMPLES: usize = 3_200;
/// Clamp ONNX worker threads to keep the engine from over-allocating on high-core CPUs.
const MAX_STT_THREADS: usize = 4;

/// Default STT model variant for new installs.
pub const DEFAULT_STT_MODEL: &str = "parakeet-110m";

/// Return a supported current speech model setting, falling back from removed
/// legacy model settings to the current default.
pub fn normalize_stt_model_name(model: &str) -> &'static str {
    match model {
        "parakeet-110m" => "parakeet-110m",
        "parakeet-0.6b" => "parakeet-0.6b",
        _ => DEFAULT_STT_MODEL,
    }
}

/// Map a model setting string to the directory name where ONNX files are stored.
pub fn stt_model_dir_name(model: &str) -> String {
    match model {
        "parakeet-110m" => "parakeet-tdt-ctc-110m".to_string(),
        "parakeet-0.6b" => "parakeet-tdt-0.6b-v2".to_string(),
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

/// Minimum size required before a downloaded STT file is treated as usable.
pub fn minimum_valid_stt_model_file_size(filename: &str) -> u64 {
    if filename == "tokens.txt" {
        100
    } else {
        1_000_000
    }
}

/// Return the valid size of a model file if it exists and is large enough.
pub fn valid_stt_model_file_size(model_dir: &Path, filename: &str) -> Option<u64> {
    let path = model_dir.join(filename);
    let size = std::fs::metadata(path).ok()?.len();
    if size > minimum_valid_stt_model_file_size(filename) {
        Some(size)
    } else {
        None
    }
}

/// True when a model directory has all files needed by the Parakeet loader.
pub fn is_valid_stt_model_dir(model_dir: &Path) -> bool {
    model_dir.is_dir()
        && (valid_stt_model_file_size(model_dir, "model.int8.onnx").is_some()
            || valid_stt_model_file_size(model_dir, "model.onnx").is_some())
        && valid_stt_model_file_size(model_dir, "tokens.txt").is_some()
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
            .map(|p| p.get().clamp(1, MAX_STT_THREADS) as i32)
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
            "Transcribed {} samples -> {} chars",
            audio.len(),
            text.len()
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
        assert_eq!(DEFAULT_STT_MODEL, "parakeet-110m");
    }

    #[test]
    fn removed_model_settings_normalize_to_default() {
        assert_eq!(normalize_stt_model_name("base.en"), DEFAULT_STT_MODEL);
        assert_eq!(normalize_stt_model_name("tiny.en"), DEFAULT_STT_MODEL);
        assert_eq!(normalize_stt_model_name("unknown"), DEFAULT_STT_MODEL);
    }

    #[test]
    fn model_dir_names_are_correct() {
        assert_eq!(stt_model_dir_name("parakeet-110m"), "parakeet-tdt-ctc-110m");
        assert_eq!(stt_model_dir_name("parakeet-0.6b"), "parakeet-tdt-0.6b-v2");
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

    #[test]
    fn minimum_valid_model_file_size_accepts_small_token_files() {
        assert_eq!(minimum_valid_stt_model_file_size("tokens.txt"), 100);
        assert_eq!(minimum_valid_stt_model_file_size("model.onnx"), 1_000_000);
    }

    #[test]
    fn speech_model_dir_requires_valid_onnx_and_tokens() {
        let dir = std::env::temp_dir().join(format!(
            "localyapper-model-status-test-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&dir).expect("test model dir");

        std::fs::write(dir.join("model.onnx"), vec![0_u8; 512]).expect("tiny model");
        std::fs::write(dir.join("tokens.txt"), vec![b'a'; 256]).expect("tokens");
        assert!(!is_valid_stt_model_dir(&dir));

        std::fs::write(
            dir.join("model.onnx"),
            vec![0_u8; minimum_valid_stt_model_file_size("model.onnx") as usize + 1],
        )
        .expect("valid model");
        assert!(is_valid_stt_model_dir(&dir));

        std::fs::remove_file(dir.join("tokens.txt")).expect("remove tokens");
        assert!(!is_valid_stt_model_dir(&dir));

        std::fs::remove_dir_all(&dir).expect("cleanup");
    }
}

#[cfg(all(test, target_os = "windows"))]
mod manual_tests {
    use super::*;
    use crate::audio::vad::{apply_vad, compute_rms};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    fn default_app_data_dir() -> Result<PathBuf, String> {
        if let Ok(path) = std::env::var("LOCALYAPPER_APP_DATA_DIR") {
            return Ok(PathBuf::from(path));
        }

        let app_data = std::env::var("APPDATA")
            .map_err(|_| "APPDATA is not set; set LOCALYAPPER_APP_DATA_DIR".to_string())?;
        Ok(PathBuf::from(app_data).join("com.localyapper.desktop"))
    }

    fn read_u16_le(bytes: &[u8], offset: usize) -> Result<u16, String> {
        let slice = bytes
            .get(offset..offset + 2)
            .ok_or_else(|| format!("WAV ended before u16 at offset {offset}"))?;
        Ok(u16::from_le_bytes([slice[0], slice[1]]))
    }

    fn read_u32_le(bytes: &[u8], offset: usize) -> Result<u32, String> {
        let slice = bytes
            .get(offset..offset + 4)
            .ok_or_else(|| format!("WAV ended before u32 at offset {offset}"))?;
        Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
    }

    fn read_pcm16_mono_16khz_wav(path: &Path) -> Result<Vec<f32>, String> {
        let bytes = fs::read(path).map_err(|e| format!("Failed to read WAV: {e}"))?;
        if bytes.get(0..4) != Some(b"RIFF") || bytes.get(8..12) != Some(b"WAVE") {
            return Err("WAV is not RIFF/WAVE".to_string());
        }

        let mut audio_format = None;
        let mut channels = None;
        let mut sample_rate = None;
        let mut bits_per_sample = None;
        let mut data_range = None;
        let mut offset = 12usize;

        while offset + 8 <= bytes.len() {
            let chunk_id = bytes
                .get(offset..offset + 4)
                .ok_or_else(|| "Missing WAV chunk id".to_string())?;
            let chunk_size = read_u32_le(&bytes, offset + 4)? as usize;
            let chunk_start = offset + 8;
            let chunk_end = chunk_start
                .checked_add(chunk_size)
                .ok_or_else(|| "WAV chunk size overflow".to_string())?;
            if chunk_end > bytes.len() {
                return Err("WAV chunk extends past end of file".to_string());
            }

            match chunk_id {
                b"fmt " => {
                    audio_format = Some(read_u16_le(&bytes, chunk_start)?);
                    channels = Some(read_u16_le(&bytes, chunk_start + 2)?);
                    sample_rate = Some(read_u32_le(&bytes, chunk_start + 4)?);
                    bits_per_sample = Some(read_u16_le(&bytes, chunk_start + 14)?);
                }
                b"data" => data_range = Some(chunk_start..chunk_end),
                _ => {}
            }

            offset = chunk_end + (chunk_size % 2);
        }

        if audio_format != Some(1) {
            return Err(format!("Expected PCM WAV format 1, got {audio_format:?}"));
        }
        if channels != Some(1) {
            return Err(format!("Expected mono WAV, got {channels:?} channels"));
        }
        if sample_rate != Some(16_000) {
            return Err(format!("Expected 16 kHz WAV, got {sample_rate:?}"));
        }
        if bits_per_sample != Some(16) {
            return Err(format!(
                "Expected 16-bit WAV samples, got {bits_per_sample:?}"
            ));
        }

        let data_range = data_range.ok_or_else(|| "WAV has no data chunk".to_string())?;
        let data = &bytes[data_range];
        if data.len() % 2 != 0 {
            return Err("PCM16 data chunk has an odd byte length".to_string());
        }

        Ok(data
            .chunks_exact(2)
            .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]) as f32 / 32768.0)
            .collect())
    }

    fn generate_windows_tts_wav(path: &Path, text: &str) -> Result<(), String> {
        let script = "Add-Type -AssemblyName System.Speech; \
            $format = New-Object System.Speech.AudioFormat.SpeechAudioFormatInfo(16000, [System.Speech.AudioFormat.AudioBitsPerSample]::Sixteen, [System.Speech.AudioFormat.AudioChannel]::Mono); \
            $speaker = New-Object System.Speech.Synthesis.SpeechSynthesizer; \
            $speaker.SetOutputToWaveFile($env:LOCALYAPPER_TTS_WAV_PATH, $format); \
            $speaker.Speak($env:LOCALYAPPER_TTS_TEXT); \
            $speaker.Dispose()";
        let status = Command::new("powershell")
            .args([
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                script,
            ])
            .env("LOCALYAPPER_TTS_WAV_PATH", path)
            .env("LOCALYAPPER_TTS_TEXT", text)
            .status()
            .map_err(|e| format!("Failed to generate Windows TTS WAV: {e}"))?;

        if status.success() {
            Ok(())
        } else {
            Err(format!("Windows TTS WAV generation exited with {status}"))
        }
    }

    #[test]
    #[ignore = "requires Windows SAPI and installed speech model files"]
    fn manual_windows_tts_file_transcription_smoke() -> Result<(), String> {
        let text = std::env::var("LOCALYAPPER_TTS_FILE_SMOKE_TEXT").unwrap_or_else(|_| {
            "LocalYapper synthetic speech transcription smoke test.".to_string()
        });
        let wav_path = std::env::temp_dir().join(format!(
            "localyapper-tts-file-smoke-{}.wav",
            std::process::id()
        ));
        generate_windows_tts_wav(&wav_path, &text)?;

        let audio = match read_pcm16_mono_16khz_wav(&wav_path) {
            Ok(audio) => audio,
            Err(e) => {
                let _ = fs::remove_file(&wav_path);
                return Err(e);
            }
        };
        let _ = fs::remove_file(&wav_path);

        let rms = compute_rms(&audio);
        let peak = audio
            .iter()
            .fold(0.0_f32, |max, sample| max.max(sample.abs()));
        println!(
            "Generated TTS WAV: {} samples, RMS {rms:.6}, peak {peak:.6}",
            audio.len()
        );

        let vad_result = apply_vad(&audio, None);
        if !vad_result.has_speech {
            return Err(format!(
                "Generated TTS audio did not pass VAD. RMS {rms:.6}, peak {peak:.6}"
            ));
        }

        let speech_model_dir = default_app_data_dir()?
            .join("models")
            .join(stt_model_dir_name(DEFAULT_STT_MODEL));
        let engine = WhisperEngine::new(&speech_model_dir).map_err(|e| e.to_string())?;
        let transcript = engine
            .transcribe(&vad_result.trimmed_audio)
            .map_err(|e| e.to_string())?;

        println!("Transcript: {transcript}");
        if transcript.trim().is_empty() {
            return Err("STT returned an empty transcript for generated speech".to_string());
        }

        Ok(())
    }
}
