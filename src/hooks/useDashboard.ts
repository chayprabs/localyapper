// Dashboard hook -- stats, last dictation, and model status with auto-refresh
import { useState, useEffect, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import type { Stats, HistoryEntry, ModelsStatus } from "@/types/commands";
import { getStats, getHistory, deleteHistoryEntry } from "@/lib/commands/history";
import { checkModelsStatus, checkOllama } from "@/lib/commands/models";
import { getAllSettings } from "@/lib/commands/settings";

interface DashboardData {
  stats: Stats | null;
  lastDictation: HistoryEntry | null;
  modelStatus: ModelsStatus | null;
  llmMode: string;
  llmLabel: string;
  isLoading: boolean;
  refresh: () => void;
  deleteLastDictation: (id: string) => Promise<void>;
}

export function useDashboard(): DashboardData {
  const [stats, setStats] = useState<Stats | null>(null);
  const [lastDictation, setLastDictation] = useState<HistoryEntry | null>(null);
  const [modelStatus, setModelStatus] = useState<ModelsStatus | null>(null);
  const [llmMode, setLlmMode] = useState("local");
  const [llmLabel, setLlmLabel] = useState("");
  const [isLoading, setIsLoading] = useState(true);

  const fetchAll = useCallback(async () => {
    setIsLoading(true);
    const [statsResult, historyResult, modelsResult, settingsResult] = await Promise.allSettled([
      getStats(),
      getHistory(1, 0),
      checkModelsStatus(),
      getAllSettings(),
    ]);

    if (statsResult.status === "fulfilled") setStats(statsResult.value);
    if (historyResult.status === "fulfilled") {
      setLastDictation(historyResult.value[0] ?? null);
    }
    if (modelsResult.status === "fulfilled") setModelStatus(modelsResult.value);

    if (settingsResult.status === "fulfilled") {
      const s = settingsResult.value;
      const mode = s["llm_mode"] ?? "local";
      setLlmMode(mode);

      if (mode === "ollama") {
        const ollamaModel = s["ollama_model"] ?? "";
        try {
          const ollama = await checkOllama();
          if (ollama.running) {
            setLlmLabel(ollamaModel || (ollama.models[0] ?? ""));
          }
        } catch { /* ignore */ }
      } else if (mode === "byok") {
        setLlmLabel(s["byok_provider"] ?? "");
      }
    }

    setIsLoading(false);
  }, []);

  useEffect(() => {
    void fetchAll();
  }, [fetchAll]);

  // Auto-refresh when a new dictation completes
  useEffect(() => {
    const unlisten = listen<{ state: string }>("pipeline-state", (event) => {
      if (event.payload.state === "injected") {
        void fetchAll();
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [fetchAll]);

  const deleteLastDictation = useCallback(
    async (id: string) => {
      await deleteHistoryEntry(id);
      void fetchAll();
    },
    [fetchAll],
  );

  return { stats, lastDictation, modelStatus, llmMode, llmLabel, isLoading, refresh: fetchAll, deleteLastDictation };
}
