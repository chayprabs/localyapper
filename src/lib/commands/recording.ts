import { invoke } from "@tauri-apps/api/core";

export async function injectText(
  text: string,
  holdShift: boolean,
): Promise<void> {
  return invoke<void>("inject_text", { text, holdShift });
}

export async function cancelRecording(): Promise<void> {
  return invoke<void>("cancel_recording");
}
