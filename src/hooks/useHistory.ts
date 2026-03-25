// History hook -- paginated entries with optimistic delete and auto-refresh
import { useState, useEffect, useCallback, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import type { HistoryEntry } from "@/types/commands";
import {
  getHistory,
  deleteHistoryEntry,
  clearHistory,
} from "@/lib/commands/history";

const PAGE_SIZE = 20;

interface HistoryData {
  entries: HistoryEntry[];
  isLoading: boolean;
  hasMore: boolean;
  loadMore: () => void;
  deleteEntry: (id: string) => Promise<void>;
  clearAll: () => Promise<void>;
  refresh: () => void;
}

export function useHistory(): HistoryData {
  const [entries, setEntries] = useState<HistoryEntry[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [hasMore, setHasMore] = useState(false);
  const offsetRef = useRef(0);

  const fetchInitial = useCallback(async () => {
    setIsLoading(true);
    try {
      const result = await getHistory(PAGE_SIZE, 0);
      setEntries(result);
      setHasMore(result.length === PAGE_SIZE);
      offsetRef.current = result.length;
    } catch {
      setEntries([]);
      setHasMore(false);
    }
    setIsLoading(false);
  }, []);

  useEffect(() => {
    void fetchInitial();
  }, [fetchInitial]);

  // Auto-refresh when a new dictation completes
  useEffect(() => {
    const unlisten = listen<{ state: string }>("pipeline-state", (event) => {
      if (event.payload.state === "injected") {
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
    } catch {
      setHasMore(false);
    }
  }, []);

  const deleteEntry = useCallback(
    async (id: string) => {
      setEntries((prev) => prev.filter((e) => e.id !== id));
      try {
        await deleteHistoryEntry(id);
      } catch {
        void fetchInitial();
      }
    },
    [fetchInitial],
  );

  const clearAll = useCallback(async () => {
    setEntries([]);
    setHasMore(false);
    offsetRef.current = 0;
    await clearHistory();
  }, []);

  return {
    entries,
    isLoading,
    hasMore,
    loadMore: () => void loadMore(),
    deleteEntry,
    clearAll,
    refresh: () => void fetchInitial(),
  };
}
