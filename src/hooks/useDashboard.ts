// Dashboard hook -- stats, last dictation, and speech model status with auto-refresh
import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import type { HistoryEntry, ModelsStatus, Stats } from "@/types/commands";
import {
  deleteHistoryEntry,
  getHistory,
  getStats,
} from "@/lib/commands/history";
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

    if (statsResult.status === "fulfilled") {
      setStats(statsResult.value);
    }
    if (historyResult.status === "fulfilled") {
      setLastDictation(historyResult.value[0] ?? null);
    }
    if (modelsResult.status === "fulfilled") {
      setModelStatus(modelsResult.value);
    }

    setIsLoading(false);
  }, []);

  useEffect(() => {
    void fetchAll();
  }, [fetchAll]);

  useEffect(() => {
    const unlisten = listen<{ state: string }>("pipeline-state", (event) => {
      if (event.payload.state === "injected") {
        void fetchAll();
      }
    });

    return () => {
      unlisten.then((dispose) => dispose());
    };
  }, [fetchAll]);

  const deleteLastDictation = useCallback(
    async (id: string) => {
      await deleteHistoryEntry(id);
      void fetchAll();
    },
    [fetchAll],
  );

  return {
    stats,
    lastDictation,
    modelStatus,
    isLoading,
    refresh: fetchAll,
    deleteLastDictation,
  };
}
