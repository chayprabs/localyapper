// History entry card -- timestamp, word count, app badge, and text preview
import type { HistoryEntry } from "@/types/commands";
import { CopyButton } from "@/components/ui/CopyButton";
import { Icon } from "@/components/ui/Icon";
import { useEffect, useState } from "react";
import { formatLocalTime, formatWordCount } from "@/lib/formatters";

interface HistoryCardProps {
  entry: HistoryEntry;
  onDelete: (id: string) => void;
  confirmDelete?: boolean;
}

export function HistoryCard({ entry, onDelete, confirmDelete = true }: HistoryCardProps) {
  // The first click asks, the second deletes. (A browser confirm box is
  // not shown by every system WebView, which made Delete do nothing.)
  const [asking, setAsking] = useState(false);
  // Long dictations show two lines until clicked.
  const [expanded, setExpanded] = useState(false);
  useEffect(() => {
    if (!asking) return;
    const timer = window.setTimeout(() => setAsking(false), 4000);
    return () => window.clearTimeout(timer);
  }, [asking]);

  const handleDelete = () => {
    if (confirmDelete && !asking) {
      setAsking(true);
      return;
    }
    setAsking(false);
    onDelete(entry.id);
  };

  return (
    <div className="bg-surface p-4 rounded-xl border border-black/[0.07] shadow-sm">
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-[minmax(0,1fr)_auto] sm:items-center mb-2.5">
        <div className="min-w-0 flex flex-wrap items-center gap-2">
          <span className="text-[12px] text-black/[0.26]">
            {formatLocalTime(entry.created_at)}
          </span>
          <span className="text-[12px] text-black/[0.26]">&middot;</span>
          <span className="text-[12px] text-black/[0.26]">
            {formatWordCount(entry.word_count)}
          </span>
        </div>

        <div className="min-w-0 flex flex-wrap items-center justify-end gap-1 sm:justify-end">
          {entry.app_name ? (
            <span className="max-w-full truncate px-2.5 py-1 bg-[rgba(0,88,188,0.12)] text-[#0058bc] text-[11px] font-semibold rounded-md">
              {entry.app_name}
            </span>
          ) : null}
          <CopyButton
            text={entry.final_text}
            variant="icon"
            iconSize={18}
            className="w-8 h-8 flex items-center justify-center rounded-md text-black/[0.26] hover:bg-black/[0.04] hover:text-black/85 transition-colors"
          />
          <button
            type="button"
            onClick={handleDelete}
            aria-label={asking ? "Confirm delete" : "Delete dictation"}
            className={
              asking
                ? "h-8 px-2.5 flex items-center justify-center rounded-md bg-[#ba1a1a] text-[12px] font-medium text-white transition-colors"
                : "w-8 h-8 flex items-center justify-center rounded-md text-black/[0.26] hover:bg-[#ba1a1a]/[0.08] hover:text-[#ba1a1a] transition-colors"
            }
          >
            {asking ? "Delete?" : <Icon name="delete" size={18} />}
          </button>
        </div>
      </div>

      <p
        onClick={() => setExpanded((v) => !v)}
        title={expanded ? undefined : "Click to show all of it"}
        className={`cursor-pointer whitespace-pre-wrap break-words text-[13px] leading-[1.5] text-black/85 ${
          expanded ? "" : "line-clamp-2"
        }`}
      >
        {entry.final_text}
      </p>
    </div>
  );
}
