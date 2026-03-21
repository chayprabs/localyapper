export interface HistoryEntry {
  id: string;
  raw_text: string;
  final_text: string;
  app_name: string | null;
  mode_id: string | null;
  duration_ms: number | null;
  word_count: number | null;
  created_at: string;
}

export interface Correction {
  id: string;
  raw_word: string;
  corrected: string;
  count: number;
  confidence: number;
  last_used_at: string | null;
  created_at: string;
}

export interface DictionaryWord {
  id: string;
  word: string;
  count: number;
  added_at: string;
}

export interface Mode {
  id: string;
  name: string;
  system_prompt: string;
  skip_llm: boolean;
  is_builtin: boolean;
  color: string;
  created_at: string;
}

export interface NewMode {
  name: string;
  system_prompt: string;
  skip_llm: boolean;
  color: string;
}

export interface AppProfile {
  id: string;
  app_name: string;
  mode_id: string;
}

export interface PipelineResult {
  raw_text: string;
  final_text: string;
  duration_ms: number;
  word_count: number;
}

export interface OllamaStatus {
  running: boolean;
  models: string[];
}

export interface DownloadProgress {
  percent: number;
  downloaded_mb: number;
  total_mb: number;
  speed_mbps: number;
}

export interface ConnectionResult {
  success: boolean;
  latency_ms: number;
  error: string | null;
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

export interface ImportResult {
  imported: number;
  skipped: number;
  errors: string[];
}

export interface ModelsStatus {
  whisper_loaded: boolean;
  llm_loaded: boolean;
}

export interface LlmFileStatus {
  exists: boolean;
  size_mb: number;
}

export type AllSettings = Record<string, string>;
