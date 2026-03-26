// Overlay type definitions -- visual states and pipeline event payloads
// Maps 1:1 to pipeline-state event values from Rust backend
/** Visual state of the floating overlay pill. Transitions driven by PipelineEvent. */
export type OverlayVisualState =
  | "hidden"        // Pill is not visible
  | "listening"     // Actively recording, blue waveform
  | "stopping-soon" // Last 15s of recording, red countdown
  | "processing"    // Pipeline running after recording stops
  | "long-recording"// Processing a recording > 30s, shows minute counter
  | "transcribed"   // Final text displayed, auto-inject countdown running
  | "no-speech";    // VAD found no speech, auto-hides after 2s

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
