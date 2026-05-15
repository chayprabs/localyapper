// Shared data models and pipeline event types
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A transcription history entry.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: String,
    /// Raw speech-to-text transcription from the local engine.
    pub raw_text: String,
    /// Final text saved to history and injected into the focused app.
    pub final_text: String,
    /// Focused app at recording time (None if detection failed).
    pub app_name: Option<String>,
    /// Speech duration from VAD in milliseconds.
    pub duration_ms: Option<i64>,
    /// Whitespace-separated word count of raw text.
    pub word_count: Option<i64>,
    pub created_at: String,
}

/// Result from the voice dictation pipeline.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PipelineResult {
    pub raw_text: String,
    pub final_text: String,
    pub duration_ms: i64,
    pub word_count: i64,
}

/// Model download progress event payload.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DownloadProgress {
    /// Completion percentage (0.0 to 100.0).
    pub percent: f64,
    /// Bytes downloaded so far, converted to megabytes.
    pub downloaded_mb: u64,
    /// Total file size in megabytes.
    pub total_mb: u64,
    /// Current download speed in megabytes per second.
    pub speed_mbps: f64,
}

/// Dashboard statistics aggregated from transcription_history.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Stats {
    /// Words dictated in the current calendar day (UTC).
    pub words_today: i64,
    /// Words dictated in the current 7-day window.
    pub words_week: i64,
    pub words_all_time: i64,
    /// Average words per minute across all sessions with duration > 0.
    pub avg_wpm: f64,
    /// Total number of completed dictation sessions.
    pub total_sessions: i64,
}

/// System permissions status.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PermissionsStatus {
    pub microphone: bool,
    pub accessibility: bool,
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
    pub speech_model_loaded: bool,
}

/// Speech model file status (exists on disk + size).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpeechModelFileStatus {
    pub exists: bool,
    pub size_mb: u64,
    pub model_name: String,
}

/// All settings as a key-value map (settings table rows flattened).
/// Keys are setting identifiers (e.g. "hotkey_record", "speech_model").
pub type AllSettings = HashMap<String, String>;
