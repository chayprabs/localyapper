// IPC wrappers -- hotkey update and reset operations
import { invoke } from "@tauri-apps/api/core";
import type { AllSettings } from "@/types/commands";

export async function updateHotkey(key: string, value: string): Promise<void> {
  return invoke<void>("update_hotkey", { key, value });
}

export async function resetHotkeys(): Promise<AllSettings> {
  return invoke<AllSettings>("reset_hotkeys");
}

/** While enabled, record-key presses emit `hotkey-detected` instead of dictating. */
export async function setHotkeyProbe(enabled: boolean): Promise<void> {
  return invoke<void>("set_hotkey_probe", { enabled });
}
