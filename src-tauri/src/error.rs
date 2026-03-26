// Error types for the application
/// Custom error types for LocalYapper.
#[derive(Debug, thiserror::Error)]
pub enum LocalYapperError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] rusqlite::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Resource lookup failure (mode, history entry, setting key).
    #[error("Not found: {0}")]
    NotFound(String),

    /// Validation failure from user-supplied data.
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// cpal device or stream initialization failure.
    #[error("Audio error: {0}")]
    AudioError(String),

    /// Whisper model load or inference failure.
    #[error("Transcription error: {0}")]
    TranscriptionError(String),

    /// mistral.rs model load, tokenizer, or generation failure.
    #[error("LLM error: {0}")]
    LlmError(String),

    /// Clipboard or keyboard simulation failure during text injection.
    #[error("Injection error: {0}")]
    InjectionError(String),
}

impl From<LocalYapperError> for String {
    fn from(err: LocalYapperError) -> String {
        err.to_string()
    }
}
