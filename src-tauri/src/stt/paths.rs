//! Where speech models live on disk, and the legacy model ids. The
//! pinned manifest and catalog replace the download lists in Phase 4.

use crate::core::stt::engine::{Family, ModelFiles};
use crate::core::stt::manager::ModelSpec;
use std::path::{Path, PathBuf};

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

/// Resolve a `speech_model` setting to an installed model in
/// `models_dir`.
pub fn resolve_spec(models_dir: &Path, setting: &str) -> Result<ModelSpec, String> {
    let id = normalize_stt_model_name(setting);
    let dir: PathBuf = models_dir.join(stt_model_dir_name(id));
    if !is_valid_stt_model_dir(&dir) {
        return Err("Speech model not downloaded. Open LocalYapper to download it.".into());
    }
    let files = ModelFiles::discover(Family::NemoCtc, &dir)?;
    Ok(ModelSpec {
        id: id.to_string(),
        family: Family::NemoCtc,
        files,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_model_is_a_clear_error() {
        let err = resolve_spec(Path::new("/nonexistent-localyapper"), "parakeet-110m")
            .err()
            .unwrap_or_default();
        assert!(err.contains("not downloaded"));
        assert_eq!(normalize_stt_model_name("tiny.en"), DEFAULT_STT_MODEL);
    }
}
