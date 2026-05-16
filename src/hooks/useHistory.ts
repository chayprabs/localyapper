// History hook -- paginated entries with optimistic delete and auto-refresh
import { useState, useEffect, useCallback, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import type { HistoryEntry } from "@/types/commands";
import type { PipelineEvent } from "@/types/overlay";
import {
  getHistory,
  deleteHistoryEntry,
  clearHistory,
} from "@/lib/commands/history";

/** Number of history entries fetched per page — matches backend LIMIT. */
const PAGE_SIZE = 20;

interface HistoryData {
  entries: HistoryEntry[];
  isLoading: boolean;
  hasMore: boolean;
  error: string | null;
  loadMore: () => void;
  deleteEntry: (id: string) => Promise<void>;
  clearAll: () => Promise<void>;
  refresh: () => void;
}

function toErrorMessage(error: unknown, fallback: string): string {
  return error instanceof Error
    ? error.message
    : typeof error === "string"
      ? error
      : fallback;
}

function eventShouldRefreshHistory(event: PipelineEvent): boolean {
  return event.state === "injected" || (event.state === "error" && Boolean(event.text));
}

export function useHistory(): HistoryData {
  const [entries, setEntries] = useState<HistoryEntry[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [hasMore, setHasMore] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const offsetRef = useRef(0);

  const fetchInitial = useCallback(async () => {
    setIsLoading(true);
    try {
      const result = await getHistory(PAGE_SIZE, 0);
      setEntries(result);
      setHasMore(result.length === PAGE_SIZE);
      offsetRef.current = result.length;
      setError(null);
    } catch (fetchError) {
      setEntries([]);
      setHasMore(false);
      setError(toErrorMessage(fetchError, "Failed to load history"));
    }
    setIsLoading(false);
  }, []);

  useEffect(() => {
    void fetchInitial();
  }, [fetchInitial]);

  // Auto-refresh when a new dictation completes
  useEffect(() => {
    const unlisten = listen<PipelineEvent>("pipeline-state", (event) => {
      if (eventShouldRefreshHistory(event.payload)) {
        void fetchInitial();
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [fetchInitial]);

  const loadMore = useCallback(async () => {
    try {
      const result = await getHistory(PAGE_SIZE, offsetRef.current);
      setEntries((prev) => [...prev, ...result]);
      setHasMore(result.length === PAGE_SIZE);
      offsetRef.current += result.length;
      setError(null);
    } catch (loadError) {
      setHasMore(false);
      setError(toErrorMessage(loadError, "Failed to load more history"));
    }
  }, []);

  const deleteEntry = useCallback(
    async (id: string) => {
      setError(null);
      setEntries((prev) => prev.filter((e) => e.id !== id));
      try {
        await deleteHistoryEntry(id);
      } catch (deleteError) {
        setError(toErrorMessage(deleteError, "Failed to delete history entry"));
        void fetchInitial();
      }
    },
    [fetchInitial],
  );

  const clearAll = useCallback(async () => {
    setError(null);
    setEntries([]);
    setHasMore(false);
    offsetRef.current = 0;
    try {
      await clearHistory();
    } catch (clearError) {
      setError(toErrorMessage(clearError, "Failed to clear history"));
      void fetchInitial();
    }
  }, [fetchInitial]);

  return {
    entries,
    isLoading,
    hasMore,
    error,
    loadMore: () => void loadMore(),
    deleteEntry,
    clearAll,
    refresh: () => void fetchInitial(),
  };
}
