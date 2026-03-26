// Shared data models and pipeline event types
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A transcription history entry.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: String,
    /// Raw Whisper transcription before correction/LLM.
    pub raw_text: String,
    /// Final text after correction engine and LLM cleanup.
    pub final_text: String,
    /// Focused app at recording time (None if detection failed).
    pub app_name: Option<String>,
    /// Active AI mode used for processing (None if unset).
    pub mode_id: Option<String>,
    /// Speech duration from VAD in milliseconds.
    pub duration_ms: Option<i64>,
    /// Whitespace-separated word count of raw text.
    pub word_count: Option<i64>,
    pub created_at: String,
}

/// A learned correction mapping.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Correction {
    pub id: String,
    pub raw_word: String,
    pub corrected: String,
    pub count: i64,
    pub confidence: f64,
    pub last_used_at: Option<String>,
    pub created_at: String,
}

/// A personal dictionary word.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DictionaryWord {
    pub id: String,
    pub word: String,
    pub count: i64,
    pub added_at: String,
}

/// An AI mode preset.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Mode {
    pub id: String,
    pub name: String,
    /// System prompt fed to the LLM for text cleanup (empty = no prompt).
    pub system_prompt: String,
    /// When true, bypass LLM entirely and use raw/corrected text as-is.
    pub skip_llm: bool,
    /// Built-in modes cannot be deleted or renamed by the user.
    pub is_builtin: bool,
    /// UI color identifier (e.g. "blue", "purple", "green").
    pub color: String,
    pub created_at: String,
}

/// Payload for creating a new mode.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NewMode {
    pub name: String,
    pub system_prompt: String,
    pub skip_llm: bool,
    pub color: String,
}

/// App profile linking an app to a mode.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppProfile {
    pub id: String,
    pub app_name: String,
    pub mode_id: String,
}

/// Result from the voice dictation pipeline.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PipelineResult {
    pub raw_text: String,
    pub final_text: String,
    pub duration_ms: i64,
    pub word_count: i64,
}

/// Ollama service status.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OllamaStatus {
    pub running: bool,
    pub models: Vec<String>,
}

/// Model download progress event payload.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DownloadProgress {
    pub percent: f64,
    pub downloaded_mb: u64,
    pub total_mb: u64,
    pub speed_mbps: f64,
}

/// BYOK API connection test result.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConnectionResult {
    pub success: bool,
    pub latency_ms: u64,
    pub error: Option<String>,
}

/// Dashboard statistics.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Stats {
    pub words_today: i64,
    pub words_week: i64,
    pub words_all_time: i64,
    pub avg_wpm: f64,
    pub total_sessions: i64,
}

/// System permissions status.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PermissionsStatus {
    pub microphone: bool,
    pub accessibility: bool,
}

/// Result of importing a dictionary.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImportResult {
    pub imported: i64,
    pub skipped: i64,
    pub errors: Vec<String>,
}

/// Pipeline state event emitted to frontend for overlay state transitions.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PipelineEvent {
    /// One of: "listening", "processing", "transcribed", "injected", "cancelled", "error"
    pub state: String,
    /// The transcribed/final text (populated in "transcribed" and "injected" states).
    pub text: Option<String>,
    /// Speech duration in milliseconds.
    pub duration_ms: Option<i64>,
    /// Word count.
    pub word_count: Option<i64>,
    /// Error message (populated in "error" state).
    pub error: Option<String>,
}

/// Status of loaded models.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelsStatus {
    pub whisper_loaded: bool,
    pub llm_loaded: bool,
}

/// LLM model file status (exists on disk + size).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LlmFileStatus {
    pub exists: bool,
    pub size_mb: u64,
}

/// Whisper model file status (exists on disk + size).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WhisperFileStatus {
    pub exists: bool,
    pub size_mb: u64,
    pub model_name: String,
}

/// All settings as a key-value map.
pub type AllSettings = HashMap<String, String>;
