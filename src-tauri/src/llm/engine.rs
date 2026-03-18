// LLM engine wrapping llama-cpp-2 for local text cleanup.

use std::num::NonZeroU32;
use std::path::Path;

use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaModel};
use llama_cpp_2::sampling::LlamaSampler;

use crate::error::LocalYapperError;

/// Encoding for decoding token bytes to UTF-8 strings.
fn new_decoder() -> encoding_rs::Decoder {
    encoding_rs::UTF_8.new_decoder()
}

/// Expected filename for the bundled LLM model.
pub const LLM_MODEL_FILENAME: &str = "qwen2.5-0.5b-q4.gguf";

/// LLM engine for text cleanup using a local GGUF model.
///
/// The backend and model are loaded once. A fresh context is created per
/// `generate()` call to avoid state leakage between requests.
pub struct LlmEngine {
    backend: LlamaBackend,
    model: LlamaModel,
    n_threads: i32,
}

// SAFETY: LlamaBackend and LlamaModel are thread-safe in practice —
// we only create contexts from them (which is safe) and never mutate them.
// The llama-cpp-2 crate doesn't mark them Send+Sync due to the C FFI,
// but the underlying llama.cpp operations we use are thread-safe.
unsafe impl Send for LlmEngine {}
unsafe impl Sync for LlmEngine {}

impl LlmEngine {
    /// Load a GGUF model from disk and initialize the engine.
    pub fn new(model_path: &Path) -> Result<Self, LocalYapperError> {
        if !model_path.exists() {
            return Err(LocalYapperError::LlmError(format!(
                "LLM model not found at {}",
                model_path.display()
            )));
        }

        let backend = LlamaBackend::init().map_err(|e| {
            LocalYapperError::LlmError(format!("Failed to init llama backend: {e}"))
        })?;

        let model_params = LlamaModelParams::default();
        let model = LlamaModel::load_from_file(&backend, model_path, &model_params).map_err(
            |e| LocalYapperError::LlmError(format!("Failed to load LLM model: {e}")),
        )?;

        let n_threads = std::thread::available_parallelism()
            .map(|p| (p.get() / 2).max(1) as i32)
            .unwrap_or(2);

        log::info!(
            "LLM engine loaded from {} using {} threads",
            model_path.display(),
            n_threads
        );

        Ok(Self {
            backend,
            model,
            n_threads,
        })
    }

    /// Generate text from a prompt.
    ///
    /// Creates a fresh context per call. Runs synchronously — call from a
    /// blocking thread via `spawn_blocking`.
    pub fn generate(&self, prompt: &str, max_tokens: u32) -> Result<String, LocalYapperError> {
        let ctx_params = LlamaContextParams::default()
            .with_n_ctx(Some(NonZeroU32::new(2048).expect("non-zero")))
            .with_n_batch(512)
            .with_n_threads(self.n_threads)
            .with_n_threads_batch(self.n_threads);

        let mut ctx = self
            .model
            .new_context(&self.backend, ctx_params)
            .map_err(|e| LocalYapperError::LlmError(format!("Failed to create LLM context: {e}")))?;

        // Tokenize the prompt
        let tokens = self
            .model
            .str_to_token(prompt, AddBos::Always)
            .map_err(|e| LocalYapperError::LlmError(format!("Tokenization failed: {e}")))?;

        if tokens.is_empty() {
            return Ok(String::new());
        }

        // Create batch and feed prompt tokens
        let mut batch = LlamaBatch::new(512, 1);
        for (i, token) in tokens.iter().enumerate() {
            let is_last = i == tokens.len() - 1;
            batch
                .add(*token, i as i32, &[0], is_last)
                .map_err(|e| LocalYapperError::LlmError(format!("Batch add failed: {e}")))?;
        }

        // Process the prompt
        ctx.decode(&mut batch)
            .map_err(|e| LocalYapperError::LlmError(format!("Prompt decode failed: {e}")))?;

        // Sample output tokens with low temperature for deterministic cleanup
        let mut sampler = LlamaSampler::chain_simple([
            LlamaSampler::temp(0.1),
            LlamaSampler::greedy(),
        ]);

        let mut output = String::new();
        let mut n_decoded = tokens.len() as i32;
        let mut decoder = new_decoder();

        for _ in 0..max_tokens {
            let new_token = sampler.sample(&ctx, batch.n_tokens() - 1);
            sampler.accept(new_token);

            if self.model.is_eog_token(new_token) {
                break;
            }

            let token_str = self
                .model
                .token_to_piece(new_token, &mut decoder, false, None)
                .map_err(|e| {
                    LocalYapperError::LlmError(format!("Token to string failed: {e}"))
                })?;
            output.push_str(&token_str);

            // Prepare next decode step
            batch.clear();
            batch
                .add(new_token, n_decoded, &[0], true)
                .map_err(|e| LocalYapperError::LlmError(format!("Batch add failed: {e}")))?;

            ctx.decode(&mut batch)
                .map_err(|e| LocalYapperError::LlmError(format!("Decode failed: {e}")))?;

            n_decoded += 1;
        }

        let result = output.trim().to_string();
        log::info!(
            "LLM generated {} chars from {} token prompt",
            result.len(),
            tokens.len()
        );

        Ok(result)
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
