import { useState, useEffect, useCallback } from "react";
import type { Stats, HistoryEntry, ModelsStatus } from "@/types/commands";
import { getStats, getHistory, deleteHistoryEntry } from "@/lib/commands/history";
import { checkModelsStatus } from "@/lib/commands/models";

interface DashboardData {
  stats: Stats | null;
  lastDictation: HistoryEntry | null;
  modelStatus: ModelsStatus | null;
  isLoading: boolean;
  refresh: () => void;
  deleteLastDictation: (id: string) => Promise<void>;
}

export function useDashboard(): DashboardData {
  const [stats, setStats] = useState<Stats | null>(null);
  const [lastDictation, setLastDictation] = useState<HistoryEntry | null>(null);
  const [modelStatus, setModelStatus] = useState<ModelsStatus | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  const fetchAll = useCallback(async () => {
    setIsLoading(true);
    const [statsResult, historyResult, modelsResult] = await Promise.allSettled([
      getStats(),
      getHistory(1, 0),
      checkModelsStatus(),
    ]);

    if (statsResult.status === "fulfilled") setStats(statsResult.value);
    if (historyResult.status === "fulfilled") {
      setLastDictation(historyResult.value[0] ?? null);
    }
    if (modelsResult.status === "fulfilled") setModelStatus(modelsResult.value);

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
