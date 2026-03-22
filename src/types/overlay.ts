export type OverlayVisualState =
  | "hidden"
  | "listening"
  | "stopping-soon"
  | "processing"
  | "long-recording"
  | "transcribed"
  | "no-speech";

export interface PipelineEvent {
  state: string;
  text: string | null;
  duration_ms: number | null;
  word_count: number | null;
  error: string | null;
}

export interface OverlayData {
  visualState: OverlayVisualState;
  text: string | null;
  durationMs: number | null;
  wordCount: number | null;
  error: string | null;
  recordingStartedAt: number | null;
}
