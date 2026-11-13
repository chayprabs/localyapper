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

/** The Audio page's test dictation: start or stop. The result arrives as
 *  a `test-dictation-result` event; nothing is pasted or saved. */
export async function testDictation(start: boolean): Promise<void> {
  return invoke<void>("test_dictation", { start });
}
