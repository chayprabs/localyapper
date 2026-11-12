// Shared data models and pipeline event types
use serde::{Deserialize, Serialize};

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
