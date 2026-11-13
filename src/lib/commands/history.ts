// IPC wrappers -- history entries and aggregated statistics
import { invoke } from "@tauri-apps/api/core";
import type { HistoryEntry, Stats } from "@/types/commands";

export async function getHistory(
  limit: number,
  offset: number,
  search: string | null = null,
): Promise<HistoryEntry[]> {
  return invoke<HistoryEntry[]>("get_history", { limit, offset, search });
}

/** Opens a save dialog and writes all history as text. Resolves to the
 *  saved path, or null when the user cancelled. */
export async function exportHistory(): Promise<string | null> {
  return invoke<string | null>("export_history");
}

export async function deleteHistoryEntry(id: string): Promise<void> {
  return invoke<void>("delete_history_entry", { id });
}

export async function clearHistory(): Promise<void> {
  return invoke<void>("clear_history");
}

export async function getStats(): Promise<Stats> {
  return invoke<Stats>("get_stats");
}
