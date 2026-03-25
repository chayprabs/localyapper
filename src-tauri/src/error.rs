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

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Audio error: {0}")]
    AudioError(String),

    #[error("Transcription error: {0}")]
    TranscriptionError(String),

    #[error("LLM error: {0}")]
    LlmError(String),

    #[error("Injection error: {0}")]
    InjectionError(String),
}

impl From<LocalYapperError> for String {
    fn from(err: LocalYapperError) -> String {
        err.to_string()
    }
}
