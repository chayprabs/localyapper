// IPC wrappers -- dictation actions handled by the backend controller
import { invoke } from "@tauri-apps/api/core";

/** Paste the last dictated text again. */
export async function pasteLast(): Promise<void> {
  return invoke<void>("paste_last");
}

/** Cancel the recording in progress. */
export async function cancelRecording(): Promise<void> {
  return invoke<void>("cancel_recording");
}
