// IPC wrappers -- speech model download, status check, and lifecycle management
import { invoke } from "@tauri-apps/api/core";
import type {
  ModelsStatus,
  ModelState,
  SpeechModelFileStatus,
} from "@/types/commands";

export async function downloadSpeechModel(): Promise<void> {
  return invoke<void>("download_speech_model");
}

export async function cancelModelDownload(): Promise<void> {
  return invoke<void>("cancel_model_download");
}

/** Reload speech model assets from disk into AppState. */
export async function reloadModels(): Promise<void> {
  return invoke<void>("reload_models");
}

export async function checkModelsStatus(): Promise<ModelsStatus> {
  return invoke<ModelsStatus>("check_models_status");
}

/** Current model load state. Backend also emits `model-state` events on change. */
export async function getModelState(): Promise<ModelState> {
  return invoke<ModelState>("get_model_state");
}

export async function checkSpeechModelFileExists(): Promise<SpeechModelFileStatus> {
  return invoke<SpeechModelFileStatus>("check_speech_model_file_exists");
}

export async function deleteSpeechModel(): Promise<void> {
  return invoke<void>("delete_speech_model");
}
