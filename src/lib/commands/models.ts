// IPC wrappers -- model download, status check, and lifecycle management
import { invoke } from "@tauri-apps/api/core";
import type { OllamaStatus, ConnectionResult, ModelsStatus, LlmFileStatus, WhisperFileStatus } from "@/types/commands";

export async function checkOllama(): Promise<OllamaStatus> {
  return invoke<OllamaStatus>("check_ollama");
}

export async function downloadModel(): Promise<void> {
  return invoke<void>("download_model");
}

export async function downloadWhisperModel(model?: string): Promise<void> {
  return invoke<void>("download_whisper_model", { model: model ?? null });
}

export async function cancelModelDownload(): Promise<void> {
  return invoke<void>("cancel_model_download");
}

export async function getOllamaModels(): Promise<string[]> {
  return invoke<string[]>("get_ollama_models");
}

export async function testByokConnection(
  provider: string,
  apiKey: string,
): Promise<ConnectionResult> {
  return invoke<ConnectionResult>("test_byok_connection", {
    provider,
    api_key: apiKey,
  });
}

// Status & lifecycle — hot-reload models without restarting the app

/** Reload Whisper and LLM models from disk into AppState. */
export async function reloadModels(): Promise<void> {
  return invoke<void>("reload_models");
}

export async function checkModelsStatus(): Promise<ModelsStatus> {
  return invoke<ModelsStatus>("check_models_status");
}

export async function checkLlmFileExists(): Promise<LlmFileStatus> {
  return invoke<LlmFileStatus>("check_llm_file_exists");
}

export async function deleteLlmModel(): Promise<void> {
  return invoke<void>("delete_llm_model");
}

export async function checkWhisperFileExists(model?: string): Promise<WhisperFileStatus> {
  return invoke<WhisperFileStatus>("check_whisper_file_exists", { model: model ?? null });
}

export async function deleteWhisperModel(model: string): Promise<void> {
  return invoke<void>("delete_whisper_model", { model });
}
