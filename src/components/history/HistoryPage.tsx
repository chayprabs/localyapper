import { useHistory } from "@/hooks/useHistory";
import { HistoryCard } from "./HistoryCard";

function KeyBadge({ children }: { children: React.ReactNode }) {
  return (
    <span className="bg-[rgba(0,0,0,0.07)] border border-[rgba(0,0,0,0.1)] rounded-[6px] px-1.5 py-px text-[11px] font-medium inline-flex items-center justify-center min-w-[1.5em]">
      {children}
    </span>
  );
}

function EmptyState() {
  const isMac = navigator.userAgent.includes("Mac");
  return (
    <div className="flex-1 flex flex-col items-center justify-center -mt-16">
      <div className="w-[56px] h-[56px] rounded-full bg-[rgba(0,0,0,0.05)] flex items-center justify-center mb-4">
        <span className="material-symbols-outlined text-[24px] text-[rgba(0,0,0,0.20)]">
          history
        </span>
      </div>
      <p className="text-[14px] font-medium text-[#1C1C1E] mb-1">
        No dictations yet
      </p>
      <p className="text-[12px] text-black/[0.26] flex items-center gap-1.5 mb-6">
        Hold <KeyBadge>{isMac ? "⌥" : "Ctrl"}</KeyBadge>{" "}
        <KeyBadge>Space</KeyBadge> to start your first dictation.
      </p>
      <button className="w-[140px] h-[36px] bg-[#0058bc] text-white text-[13px] font-medium rounded-[8px] hover:bg-[#004ea8] transition-colors shadow-sm">
        Start Dictating
      </button>
    </div>
  );
}

export function HistoryPage() {
  const { entries, isLoading, hasMore, loadMore, deleteEntry, clearAll } =
    useHistory();

  const isEmpty = !isLoading && entries.length === 0;

  const handleCopy = (text: string) => {
    void navigator.clipboard.writeText(text);
  };

  const handleDelete = (id: string) => {
    void deleteEntry(id);
  };

  const handleClearAll = () => {
    if (window.confirm("Delete all history? This cannot be undone.")) {
      void clearAll();
    }
  };

  return (
    <div className="flex flex-col h-full px-12 py-10">
      {/* Header */}
      <header className="mb-10 flex justify-between items-baseline shrink-0">
        <h1 className="text-[26px] font-semibold text-[#1C1C1E]">History</h1>
        <button
          onClick={handleClearAll}
          disabled={isEmpty}
          className={
            isEmpty
              ? "text-[13px] font-medium text-black/[0.20] cursor-default"
              : "text-[13px] font-medium text-[#ba1a1a] hover:underline transition-all"
          }
        >
          Clear All
        </button>
      </header>

      {/* Content */}
      {isEmpty ? (
        <EmptyState />
      ) : (
        <div className="flex-1 overflow-y-auto pr-2 -mr-2 flex flex-col gap-2">
          {entries.map((entry) => (
            <HistoryCard
              key={entry.id}
              entry={entry}
              onCopy={handleCopy}
              onDelete={handleDelete}
            />
          ))}

          {/* Load More */}
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
