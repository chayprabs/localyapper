// TypeScript type definitions matching Rust IPC structs
export interface HistoryEntry {
  id: string;
  raw_text: string;
  final_text: string;
  app_name: string | null;
  duration_ms: number | null;
  word_count: number | null;
  created_at: string;
}

// Pipeline & model types

export interface PipelineResult {
  raw_text: string;
  final_text: string;
  duration_ms: number;
  word_count: number;
}

export interface DownloadProgress {
  percent: number;
  downloaded_mb: number;
  total_mb: number;
  speed_mbps: number;
}

export interface Stats {
  words_today: number;
  words_week: number;
  words_all_time: number;
  avg_wpm: number;
  total_sessions: number;
}

export interface PermissionsStatus {
  microphone: boolean;
  accessibility: boolean;
}

export interface ModelsStatus {
  speech_model_loaded: boolean;
}

export interface SpeechModelFileStatus {
  exists: boolean;
  size_mb: number;
  model_name: string;
}

export type AllSettings = Record<string, string>;
