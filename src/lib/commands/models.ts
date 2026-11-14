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

/** Whether a model download is running in the backend right now. */
export async function modelDownloadActive(): Promise<boolean> {
  return invoke<boolean>("model_download_active");
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

/** One entry of the speech model catalog. */
export interface ModelInfo {
  id: string;
  name: string;
  languages: string[];
  license: string;
  size_bytes: number;
  /** "installed" | "partial" | "missing" */
  state: string;
  active: boolean;
  recommended: boolean;
  loaded: boolean;
}

export async function listModels(): Promise<ModelInfo[]> {
  return invoke<ModelInfo[]>("list_models");
}

/** Download and verify a model; progress arrives as `model-download-progress`. */
export async function downloadModel(id: string): Promise<void> {
  return invoke<void>("download_model", { id });
}

/** Make `id` (or "auto") the dictation model. */
export async function selectModel(id: string): Promise<void> {
  return invoke<void>("use_model", { id });
}

export async function deleteModel(id: string): Promise<void> {
  return invoke<void>("delete_model", { id });
}
