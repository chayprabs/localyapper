// IPC wrappers -- system utilities, permissions, and OS settings
import { invoke } from "@tauri-apps/api/core";
import type { PermissionsStatus } from "@/types/commands";

export async function getFocusedApp(): Promise<string> {
  return invoke<string>("get_focused_app");
}

export async function checkPermissions(): Promise<PermissionsStatus> {
  return invoke<PermissionsStatus>("check_permissions");
}

export async function getPausedState(): Promise<boolean> {
  return invoke<boolean>("get_paused_state");
}

// OS settings deep links

export async function openAccessibilitySettings(): Promise<void> {
  return invoke<void>("open_accessibility_settings");
}

export async function openMicSettings(): Promise<void> {
  return invoke<void>("open_mic_settings");
}
