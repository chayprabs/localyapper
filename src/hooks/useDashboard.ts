// Dashboard hook -- stats, last dictation, and speech model status with auto-refresh
import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import type { HistoryEntry, ModelsStatus, Stats } from "@/types/commands";
import type { PipelineEvent } from "@/types/overlay";
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
  error: string | null;
  refresh: () => void;
  deleteLastDictation: (id: string) => Promise<void>;
}

function toErrorMessage(error: unknown, fallback: string): string {
  return error instanceof Error
    ? error.message
    : typeof error === "string"
      ? error
      : fallback;
}

function eventShouldRefreshDashboard(event: PipelineEvent): boolean {
  return event.state === "injected" || (event.state === "error" && Boolean(event.text));
}

export function useDashboard(): DashboardData {
  const [stats, setStats] = useState<Stats | null>(null);
  const [lastDictation, setLastDictation] = useState<HistoryEntry | null>(null);
  const [modelStatus, setModelStatus] = useState<ModelsStatus | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

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
    const failed = [statsResult, historyResult, modelsResult].find(
      (result) => result.status === "rejected",
    );
    setError(
      failed?.status === "rejected"
        ? toErrorMessage(failed.reason, "Failed to load dashboard")
        : null,
    );

    setIsLoading(false);
  }, []);

  useEffect(() => {
    void fetchAll();
  }, [fetchAll]);

  useEffect(() => {
    const unlisten = listen<PipelineEvent>("pipeline-state", (event) => {
      if (eventShouldRefreshDashboard(event.payload)) {
        void fetchAll();
      }
    });

    return () => {
      unlisten.then((dispose) => dispose());
    };
  }, [fetchAll]);

  const deleteLastDictation = useCallback(
    async (id: string) => {
      setError(null);
      try {
        await deleteHistoryEntry(id);
        void fetchAll();
      } catch (deleteError) {
        setError(
          toErrorMessage(deleteError, "Failed to delete last dictation"),
        );
      }
    },
    [fetchAll],
  );

  return {
    stats,
    lastDictation,
    modelStatus,
    isLoading,
    error,
    refresh: fetchAll,
    deleteLastDictation,
  };
}
