// IPC wrappers -- AI mode CRUD and active mode selection
import { invoke } from "@tauri-apps/api/core";
import type { Mode, NewMode } from "@/types/commands";

export async function getModes(): Promise<Mode[]> {
  return invoke<Mode[]>("get_modes");
}

export async function createMode(mode: NewMode): Promise<Mode> {
  return invoke<Mode>("create_mode", { mode });
}

export async function updateMode(mode: Mode): Promise<void> {
  return invoke<void>("update_mode", { mode });
}

export async function deleteMode(id: string): Promise<void> {
  return invoke<void>("delete_mode", { id });
}

export async function setActiveMode(id: string): Promise<void> {
  return invoke<void>("set_active_mode", { id });
}

export async function getActiveMode(): Promise<Mode> {
  return invoke<Mode>("get_active_mode");
}
