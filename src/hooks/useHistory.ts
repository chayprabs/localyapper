// History hook -- paginated entries with optimistic delete and auto-refresh
import { useState, useEffect, useCallback, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import type { HistoryEntry } from "@/types/commands";
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

export function useHistory(search = ""): HistoryData {
  const [entries, setEntries] = useState<HistoryEntry[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [hasMore, setHasMore] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const offsetRef = useRef(0);
  // Each fetch gets a number; a reply to an older one (a slower search
  // for text typed earlier) is dropped instead of replacing newer results.
  const requestRef = useRef(0);

  const fetchInitial = useCallback(async () => {
    const request = ++requestRef.current;
    try {
      const result = await getHistory(PAGE_SIZE, 0, search || null);
      if (request !== requestRef.current) return;
      setEntries(result);
      setHasMore(result.length === PAGE_SIZE);
      offsetRef.current = result.length;
      setError(null);
    } catch (fetchError) {
      if (request !== requestRef.current) return;
      setEntries([]);
      setHasMore(false);
      setError(toErrorMessage(fetchError, "Failed to load history"));
    }
    // The spinner shows only until the first result: later refreshes (a
    // new dictation, a search) swap the list in place.
    setIsLoading(false);
  }, [search]);

  useEffect(() => {
    void fetchInitial();
  }, [fetchInitial]);

  // Auto-refresh when a new dictation completes
  useEffect(() => {
    const unlisten = listen("history-updated", () => {
      void fetchInitial();
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [fetchInitial]);

  const loadMore = useCallback(async () => {
    const request = requestRef.current;
    try {
      const result = await getHistory(PAGE_SIZE, offsetRef.current, search || null);
      // The list was reloaded meanwhile; this page no longer follows it.
      if (request !== requestRef.current) return;
      setEntries((prev) => {
        const seen = new Set(prev.map((e) => e.id));
        return [...prev, ...result.filter((e) => !seen.has(e.id))];
      });
      setHasMore(result.length === PAGE_SIZE);
      offsetRef.current += result.length;
      setError(null);
    } catch (loadError) {
      setHasMore(false);
      setError(toErrorMessage(loadError, "Failed to load more history"));
    }
  }, [search]);

  const deleteEntry = useCallback(
    async (id: string) => {
      setError(null);
      setEntries((prev) => prev.filter((e) => e.id !== id));
      offsetRef.current = Math.max(0, offsetRef.current - 1);
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
