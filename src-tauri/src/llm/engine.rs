// LLM engine stub — local llama-cpp inference is disabled to avoid
// ggml symbol conflicts with whisper-rs on Windows. Use Ollama or BYOK instead.

use std::path::Path;

use crate::error::LocalYapperError;

/// Expected filename for the bundled LLM model.
pub const LLM_MODEL_FILENAME: &str = "qwen2.5-0.5b-q4.gguf";

/// LLM engine stub. Local inference is disabled; use Ollama or BYOK.
pub struct LlmEngine {
    _private: (),
}

// SAFETY: Stub has no interior state.
unsafe impl Send for LlmEngine {}
unsafe impl Sync for LlmEngine {}

impl LlmEngine {
    /// Always returns an error — local LLM is disabled.
    pub fn new(_model_path: &Path) -> Result<Self, LocalYapperError> {
        Err(LocalYapperError::LlmError(
            "Local LLM inference is disabled. Use Ollama or BYOK for text cleanup.".to_string(),
        ))
    }

    /// Always returns an error — local LLM is disabled.
    pub fn generate(&self, _prompt: &str, _max_tokens: u32) -> Result<String, LocalYapperError> {
        Err(LocalYapperError::LlmError(
            "Local LLM inference is disabled.".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_filename_is_correct() {
        assert_eq!(LLM_MODEL_FILENAME, "qwen2.5-0.5b-q4.gguf");
    }
}
