import { useState, useEffect, useCallback } from "react";
import type { Stats, HistoryEntry, OllamaStatus } from "@/types/commands";
import { getStats, getHistory, deleteHistoryEntry } from "@/lib/commands/history";
import { checkOllama } from "@/lib/commands/models";

interface DashboardData {
  stats: Stats | null;
  lastDictation: HistoryEntry | null;
  modelStatus: OllamaStatus | null;
  isLoading: boolean;
  refresh: () => void;
  deleteLastDictation: (id: string) => Promise<void>;
}

export function useDashboard(): DashboardData {
  const [stats, setStats] = useState<Stats | null>(null);
  const [lastDictation, setLastDictation] = useState<HistoryEntry | null>(null);
  const [modelStatus, setModelStatus] = useState<OllamaStatus | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  const fetchAll = useCallback(async () => {
    setIsLoading(true);
    const [statsResult, historyResult, ollamaResult] = await Promise.allSettled([
      getStats(),
      getHistory(1, 0),
      checkOllama(),
    ]);

    if (statsResult.status === "fulfilled") setStats(statsResult.value);
    if (historyResult.status === "fulfilled") {
      setLastDictation(historyResult.value[0] ?? null);
    }
    if (ollamaResult.status === "fulfilled") setModelStatus(ollamaResult.value);

    setIsLoading(false);
  }, []);

  useEffect(() => {
    void fetchAll();
  }, [fetchAll]);

  const deleteLastDictation = useCallback(
    async (id: string) => {
      await deleteHistoryEntry(id);
      void fetchAll();
    },
    [fetchAll],
  );

  return { stats, lastDictation, modelStatus, isLoading, refresh: fetchAll, deleteLastDictation };
}
