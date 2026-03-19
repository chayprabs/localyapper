import { invoke } from "@tauri-apps/api/core";
import type { OllamaStatus, ConnectionResult } from "@/types/commands";

export async function checkOllama(): Promise<OllamaStatus> {
  return invoke<OllamaStatus>("check_ollama");
}

export async function downloadModel(): Promise<void> {
  return invoke<void>("download_model");
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
