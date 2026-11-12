// IPC wrappers -- system utilities, permissions, and OS settings
import { invoke } from "@tauri-apps/api/core";
import type { PermissionsStatus } from "@/types/commands";

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

/** macOS: show the microphone prompt once. No-op elsewhere. */
export async function requestMicrophonePermission(): Promise<void> {
  return invoke<void>("request_microphone_permission");
}

/** macOS: show the Accessibility prompt when off. Returns whether granted. */
export async function requestAccessibilityPermission(): Promise<boolean> {
  return invoke<boolean>("request_accessibility_permission");
}

/** Input device names, the system default first. */
export async function listInputDevices(): Promise<string[]> {
  return invoke<string[]>("list_input_devices");
}

export interface AboutInfo {
  version: string;
  data_dir: string;
  log_dir: string;
}

export async function aboutInfo(): Promise<AboutInfo> {
  return invoke<AboutInfo>("about_info");
}

/** OS, CPU, RAM, model and last timings as text. Never dictated text. */
export async function diagnostics(): Promise<string> {
  return invoke<string>("diagnostics");
}

/** Open the GitHub releases page in the browser (no auto-updater). */
export async function openReleases(): Promise<void> {
  return invoke<void>("open_releases");
}
