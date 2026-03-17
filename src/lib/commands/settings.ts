import { invoke } from "@tauri-apps/api/core";
import type { AllSettings } from "@/types/commands";

export async function getSetting(key: string): Promise<string> {
  return invoke<string>("get_setting", { key });
}

export async function setSetting(key: string, value: string): Promise<void> {
  return invoke<void>("set_setting", { key, value });
}

export async function getAllSettings(): Promise<AllSettings> {
  return invoke<AllSettings>("get_all_settings");
}
