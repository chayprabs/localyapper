// LLM inference engine -- mistral.rs Candle backend
// LLM engine powered by mistral.rs (Candle backend, no ggml dependency).
// Loads a Qwen3 GGUF model for local text cleanup/formatting.

use std::path::Path;

use mistralrs::{GgufModelBuilder, Model, RequestBuilder, TextMessageRole, TextMessages};

use crate::error::LocalYapperError;

/// Expected filename for the bundled LLM GGUF model.
pub const LLM_MODEL_FILENAME: &str = "qwen2.5-1.5b-instruct-q4_k_m.gguf";

/// Expected filename for the tokenizer (downloaded alongside the GGUF).
pub const LLM_TOKENIZER_FILENAME: &str = "tokenizer.json";

/// LLM engine wrapping a mistral.rs Model for local inference.
pub struct LlmEngine {
    model: Model,
}

impl LlmEngine {
    /// Load a GGUF model from the given directory.
    ///
    /// Expects `LLM_MODEL_FILENAME` and `LLM_TOKENIZER_FILENAME` to exist in `model_dir`.
    /// This is async because mistral.rs model loading is async.
    pub async fn new(model_dir: &Path) -> Result<Self, LocalYapperError> {
        let tokenizer_path = model_dir.join(LLM_TOKENIZER_FILENAME);

        if !model_dir.join(LLM_MODEL_FILENAME).exists() {
            return Err(LocalYapperError::LlmError(format!(
                "GGUF file not found: {}",
                model_dir.join(LLM_MODEL_FILENAME).display()
            )));
        }

        if !tokenizer_path.exists() {
            return Err(LocalYapperError::LlmError(format!(
                "Tokenizer not found: {}",
                tokenizer_path.display()
            )));
        }

        println!("LLM: Loading model from {}", model_dir.display());

        let model = GgufModelBuilder::new(
            model_dir.to_string_lossy().to_string(),
            vec![LLM_MODEL_FILENAME.to_string()],
        )
        .with_tokenizer_json(tokenizer_path.to_string_lossy().to_string())
        .with_force_cpu()
        .with_logging()
        .build()
        .await
        .map_err(|e| LocalYapperError::LlmError(format!("Failed to load model: {e}")))?;

        println!("LLM: Model loaded successfully");
        Ok(Self { model })
    }

    /// Run LLM text cleanup: given a system prompt and user text, return cleaned text.
    ///
    /// Uses deterministic sampling (temperature=0) with max 1024 output tokens.
    /// Thinking mode is disabled for Qwen3 to get direct output.
    pub async fn generate(
        &self,
        system_prompt: &str,
        user_text: &str,
    ) -> Result<String, LocalYapperError> {
        let messages = TextMessages::new()
            .add_message(TextMessageRole::System, system_prompt)
            .add_message(TextMessageRole::User, user_text)
            .enable_thinking(false);

        let request = RequestBuilder::from(messages)
            .set_sampler_temperature(0.0)
            .set_sampler_max_len(1024);

        let response = self
            .model
            .send_chat_request(request)
            .await
            .map_err(|e| LocalYapperError::LlmError(format!("Inference failed: {e}")))?;

        response
            .choices
            .first()
            .and_then(|c| c.message.content.as_ref())
            .map(|s| s.to_string())
            .ok_or_else(|| LocalYapperError::LlmError("Empty response from model".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_filename_is_correct() {
        assert_eq!(LLM_MODEL_FILENAME, "qwen2.5-1.5b-instruct-q4_k_m.gguf");
    }

    #[test]
    fn tokenizer_filename_is_correct() {
        assert_eq!(LLM_TOKENIZER_FILENAME, "tokenizer.json");
    }
}
