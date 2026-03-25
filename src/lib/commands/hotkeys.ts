// IPC wrappers -- hotkey update and reset operations
import { invoke } from "@tauri-apps/api/core";
import type { AllSettings } from "@/types/commands";

export async function updateHotkey(key: string, value: string): Promise<void> {
  return invoke<void>("update_hotkey", { key, value });
}

export async function resetHotkeys(): Promise<AllSettings> {
  return invoke<AllSettings>("reset_hotkeys");
}
