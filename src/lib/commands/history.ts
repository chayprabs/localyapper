import { invoke } from "@tauri-apps/api/core";
import type { HistoryEntry, Stats } from "@/types/commands";

export async function getHistory(
  limit: number,
  offset: number,
): Promise<HistoryEntry[]> {
  return invoke<HistoryEntry[]>("get_history", { limit, offset });
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
