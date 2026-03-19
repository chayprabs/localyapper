import { invoke } from "@tauri-apps/api/core";
import type { Correction, ImportResult } from "@/types/commands";

export async function getCorrections(
  limit: number,
  offset: number,
): Promise<Correction[]> {
  return invoke<Correction[]>("get_corrections", { limit, offset });
}

export async function addCorrection(
  rawWord: string,
  corrected: string,
): Promise<Correction> {
  return invoke<Correction>("add_correction", {
    raw_word: rawWord,
    corrected,
  });
}

export async function deleteCorrection(id: string): Promise<void> {
  return invoke<void>("delete_correction", { id });
}

export async function exportDictionary(): Promise<string> {
  return invoke<string>("export_dictionary");
}

export async function importDictionary(json: string): Promise<ImportResult> {
  return invoke<ImportResult>("import_dictionary", { json });
}
