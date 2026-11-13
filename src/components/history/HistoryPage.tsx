// History page -- paginated list of all past dictations
import { useEffect, useMemo, useState } from "react";
import { useSetAtom } from "jotai";
import { useHistory } from "@/hooks/useHistory";
import { activePageAtom } from "@/stores/appStore";
import { HistoryCard } from "./HistoryCard";
import { Icon } from "@/components/ui/Icon";
import { exportHistory } from "@/lib/commands/history";
import { formatLocalDay } from "@/lib/formatters";
import type { HistoryEntry } from "@/types/commands";

/** Consecutive entries grouped by their local calendar day. */
function groupByDay(entries: HistoryEntry[]): { day: string; items: HistoryEntry[] }[] {
  const groups: { day: string; items: HistoryEntry[] }[] = [];
  for (const entry of entries) {
    const day = formatLocalDay(entry.created_at);
    const last = groups[groups.length - 1];
    if (last && last.day === day) {
      last.items.push(entry);
    } else {
      groups.push({ day, items: [entry] });
    }
  }
  return groups;
}

function errorText(err: unknown): string {
  return err instanceof Error ? err.message : typeof err === "string" ? err : "Export failed";
}

function EmptyState({ onOpenHotkeys }: { onOpenHotkeys: () => void }) {
  return (
    <div className="flex-1 flex flex-col items-center justify-center -mt-16">
      <div className="w-[56px] h-[56px] rounded-full bg-black/[0.05] flex items-center justify-center mb-4">
        <Icon name="history" size={24} className="text-black/[0.20]" />
      </div>
      <p className="text-[14px] font-medium text-black/90 mb-1">
        No dictations yet
      </p>
      <p className="text-[12px] text-black/[0.26] text-center mb-6">
        Use your record hotkey to start your first dictation.
      </p>
      <button
        onClick={onOpenHotkeys}
        className="w-[140px] h-[36px] bg-[#0058bc] text-white text-[13px] font-medium rounded-[8px] hover:bg-[#004ea8] transition-colors shadow-sm"
      >
        Set up dictation
      </button>
    </div>
  );
}

export function HistoryPage() {
  const [query, setQuery] = useState("");
  const [search, setSearch] = useState("");
  const { entries, isLoading, hasMore, error, loadMore, deleteEntry, clearAll } =
    useHistory(search);
  const setActivePage = useSetAtom(activePageAtom);
  const [showClearConfirm, setShowClearConfirm] = useState(false);
  const [exportNote, setExportNote] = useState<string | null>(null);
  const groups = useMemo(() => groupByDay(entries), [entries]);

  // Search as the user types, without a query per keystroke.
  useEffect(() => {
    const t = window.setTimeout(() => setSearch(query.trim()), 250);
    return () => window.clearTimeout(t);
  }, [query]);

  const isEmpty = !isLoading && entries.length === 0;
  const noHistory = isEmpty && search === "";

  const handleExport = () => {
    setExportNote(null);
    exportHistory()
      .then((path) => setExportNote(path ? `Saved to ${path}` : null))
      .catch((err: unknown) => setExportNote(errorText(err)));
  };

  useEffect(() => {
    if (isEmpty && showClearConfirm) {
      setShowClearConfirm(false);
    }
  }, [isEmpty, showClearConfirm]);

  const handleDelete = (id: string) => {
    void deleteEntry(id);
  };

  const handleConfirmClearAll = () => {
    setShowClearConfirm(false);
    void clearAll();
  };

  return (
    <div className="flex flex-col h-full px-6 py-8 sm:px-12 sm:py-10">
      <header className="mb-10 flex justify-between items-start gap-4 shrink-0">
        <div>
          <h1 className="text-[26px] font-semibold text-black/90">History</h1>
          {exportNote && (
            <p className="mt-2 max-w-[520px] break-all text-[12px] text-black/50">{exportNote}</p>
          )}
          {error && (
            <p className="mt-3 max-w-[520px] rounded-lg border border-[#ba1a1a]/15 bg-[#ba1a1a]/[0.06] px-3 py-2 text-[12px] font-medium text-[#ba1a1a]">
              {error}
            </p>
          )}
        </div>
        {showClearConfirm ? (
          <div className="flex flex-wrap items-center justify-end gap-2">
            <span className="text-[13px] text-black/50">Clear all history?</span>
            <button
              onClick={() => setShowClearConfirm(false)}
              className="h-8 rounded-lg border border-black/[0.15] bg-surface px-3 text-[13px] font-medium shadow-sm transition-colors hover:bg-black/[0.02]"
            >
              Cancel
            </button>
            <button
              onClick={handleConfirmClearAll}
              className="h-8 rounded-lg bg-[#ba1a1a] px-3 text-[13px] font-medium text-white shadow-sm transition-colors hover:bg-[#a01616]"
            >
              Clear All
            </button>
          </div>
        ) : (
          <div className="flex items-center gap-4">
            <button
              onClick={handleExport}
              disabled={noHistory}
              className={
                noHistory
                  ? "text-[13px] font-medium text-black/[0.20] cursor-default"
                  : "text-[13px] font-medium text-[#0058bc] hover:underline"
              }
            >
              Export
            </button>
            <button
              onClick={() => setShowClearConfirm(true)}
              disabled={noHistory}
              className={
                noHistory
                  ? "text-[13px] font-medium text-black/[0.20] cursor-default"
                  : "text-[13px] font-medium text-[#ba1a1a] hover:underline transition-all"
              }
            >
              Clear All
            </button>
          </div>
        )}
      </header>

      <div className="mb-4 shrink-0">
        <input
          type="search"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Search dictations"
          aria-label="Search dictations"
          className="h-9 w-full max-w-[360px] rounded-lg border border-black/[0.12] bg-surface px-3 text-[13px] text-black/85 outline-none focus:border-[#0058bc]"
        />
      </div>

      {isLoading ? (
        <div className="flex flex-1 items-center justify-center">
          <Icon name="progress_activity" size={28} className="text-black/[0.30] animate-spin" />
        </div>
      ) : noHistory ? (
        <EmptyState onOpenHotkeys={() => setActivePage("dictation")} />
      ) : isEmpty ? (
        <p className="text-[13px] text-black/50">No dictations match “{search}”.</p>
      ) : (
        <div className="flex-1 overflow-y-auto pr-2 -mr-2 flex flex-col gap-2">
          {groups.map((group) => (
            <section key={group.day} className="flex flex-col gap-2">
              <h2 className="mt-3 text-[12px] font-semibold uppercase tracking-wide text-black/40 first:mt-0">
                {group.day}
              </h2>
              {group.items.map((entry) => (
                <HistoryCard key={entry.id} entry={entry} onDelete={handleDelete} />
              ))}
            </section>
          ))}

          {hasMore && (
            <div className="flex justify-center mt-6 mb-10 shrink-0">
              <button
                onClick={loadMore}
                className="h-8 px-4 bg-surface border border-black/[0.15] rounded-lg text-[13px] text-black/85 font-medium hover:bg-gray-50 transition-colors"
              >
                Load More
              </button>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
