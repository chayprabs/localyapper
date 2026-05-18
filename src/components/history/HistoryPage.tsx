// History page -- paginated list of all past dictations
import { useEffect, useState } from "react";
import { useSetAtom } from "jotai";
import { useHistory } from "@/hooks/useHistory";
import { activePageAtom } from "@/stores/appStore";
import { HistoryCard } from "./HistoryCard";
import { Icon } from "@/components/ui/Icon";

function EmptyState({ onOpenHotkeys }: { onOpenHotkeys: () => void }) {
  return (
    <div className="flex-1 flex flex-col items-center justify-center -mt-16">
      <div className="w-[56px] h-[56px] rounded-full bg-[rgba(0,0,0,0.05)] flex items-center justify-center mb-4">
        <Icon name="history" size={24} className="text-[rgba(0,0,0,0.20)]" />
      </div>
      <p className="text-[14px] font-medium text-[#1C1C1E] mb-1">
        No dictations yet
      </p>
      <p className="text-[12px] text-black/[0.26] text-center mb-6">
        Use your record hotkey to start your first dictation.
      </p>
      <button
        onClick={onOpenHotkeys}
        className="w-[140px] h-[36px] bg-[#0058bc] text-white text-[13px] font-medium rounded-[8px] hover:bg-[#004ea8] transition-colors shadow-sm"
      >
        Open Hotkeys
      </button>
    </div>
  );
}

export function HistoryPage() {
  const { entries, isLoading, hasMore, error, loadMore, deleteEntry, clearAll } =
    useHistory();
  const setActivePage = useSetAtom(activePageAtom);
  const [showClearConfirm, setShowClearConfirm] = useState(false);

  const isEmpty = !isLoading && entries.length === 0;

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
    <div className="flex flex-col h-full px-12 py-10">
      <header className="mb-10 flex justify-between items-start gap-4 shrink-0">
        <div>
          <h1 className="text-[26px] font-semibold text-[#1C1C1E]">History</h1>
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
              className="h-8 rounded-lg border border-black/[0.15] bg-white px-3 text-[13px] font-medium shadow-sm transition-colors hover:bg-black/[0.02]"
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
          <button
            onClick={() => setShowClearConfirm(true)}
            disabled={isEmpty}
            className={
              isEmpty
                ? "text-[13px] font-medium text-black/[0.20] cursor-default"
                : "text-[13px] font-medium text-[#ba1a1a] hover:underline transition-all"
            }
          >
            Clear All
          </button>
        )}
      </header>

      {isEmpty ? (
        <EmptyState onOpenHotkeys={() => setActivePage("hotkeys")} />
      ) : (
        <div className="flex-1 overflow-y-auto pr-2 -mr-2 flex flex-col gap-2">
          {entries.map((entry) => (
            <HistoryCard
              key={entry.id}
              entry={entry}
              onDelete={handleDelete}
            />
          ))}

          {hasMore && (
            <div className="flex justify-center mt-6 mb-10 shrink-0">
              <button
                onClick={loadMore}
                className="h-8 px-4 bg-white border border-[rgba(0,0,0,0.15)] rounded-lg text-[13px] text-black/85 font-medium hover:bg-gray-50 transition-colors"
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
