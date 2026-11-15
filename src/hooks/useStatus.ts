// Status hook -- word counts, whether a speech model is installed and the
// record hotkey, refreshed when history or models change
import { DEFAULT_RECORD_HOTKEY } from "@/lib/hotkeyDefaults";
import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import type { SpeechModelFileStatus, Stats } from "@/types/commands";
import { getStats } from "@/lib/commands/history";
import { checkSpeechModelFileExists } from "@/lib/commands/models";
import { getSetting } from "@/lib/commands/settings";

interface StatusData {
  stats: Stats | null;
  modelFileStatus: SpeechModelFileStatus | null;
  recordHotkey: string;
  isLoading: boolean;
  error: string | null;
  refresh: () => void;
}

function toErrorMessage(error: unknown, fallback: string): string {
  return error instanceof Error ? error.message : typeof error === "string" ? error : fallback;
}

export function useStatus(): StatusData {
  const [stats, setStats] = useState<Stats | null>(null);
  const [modelFileStatus, setModelFileStatus] = useState<SpeechModelFileStatus | null>(null);
  const [recordHotkey, setRecordHotkey] = useState(DEFAULT_RECORD_HOTKEY);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Only the first load shows a spinner; refreshes update in place.
  const fetchAll = useCallback(async () => {
    const [statsResult, filesResult, hotkeyResult] = await Promise.allSettled([
      getStats(),
      checkSpeechModelFileExists(),
      getSetting("hotkey_record"),
    ]);

    if (statsResult.status === "fulfilled") setStats(statsResult.value);
    if (filesResult.status === "fulfilled") setModelFileStatus(filesResult.value);
    if (
      hotkeyResult.status === "fulfilled" &&
      typeof hotkeyResult.value === "string" &&
      hotkeyResult.value.length > 0
    ) {
      setRecordHotkey(hotkeyResult.value);
    }

    const failed = [statsResult, filesResult].find((result) => result.status === "rejected");
    setError(failed?.status === "rejected" ? toErrorMessage(failed.reason, "Failed to load status") : null);
    setIsLoading(false);
  }, []);

  useEffect(() => {
    void fetchAll();
    const refresh = () => void fetchAll();
    const unlisten = [listen("history-updated", refresh), listen("models-changed", refresh)];
    return () => {
      for (const u of unlisten) void u.then((dispose) => dispose());
    };
  }, [fetchAll]);

  return { stats, modelFileStatus, recordHotkey, isLoading, error, refresh: () => void fetchAll() };
}
