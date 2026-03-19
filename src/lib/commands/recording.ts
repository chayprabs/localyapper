import { invoke } from "@tauri-apps/api/core";
import type { PipelineResult } from "@/types/commands";

export async function startRecording(): Promise<void> {
  return invoke<void>("start_recording");
}

export async function stopRecording(): Promise<PipelineResult> {
  return invoke<PipelineResult>("stop_recording");
}

export async function runPipeline(audio: number[]): Promise<PipelineResult> {
  return invoke<PipelineResult>("run_pipeline", { audio });
}

export async function injectText(
  text: string,
  holdShift: boolean,
): Promise<void> {
  return invoke<void>("inject_text", { text, holdShift });
}

export async function pasteLast(): Promise<void> {
  return invoke<void>("paste_last");
}

export async function cancelRecording(): Promise<void> {
  return invoke<void>("cancel_recording");
}
