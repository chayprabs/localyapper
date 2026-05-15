// History entry card -- timestamp, word count, app badge, and text preview
import type { HistoryEntry } from "@/types/commands";
import { CopyButton } from "@/components/ui/CopyButton";
import { formatHistoryTimestamp } from "@/lib/formatters";

interface HistoryCardProps {
  entry: HistoryEntry;
  onDelete: (id: string) => void;
}

export function HistoryCard({ entry, onDelete }: HistoryCardProps) {
  return (
    <div className="bg-white p-4 rounded-xl border border-black/[0.07] shadow-sm">
      <div className="grid grid-cols-[minmax(0,1fr)_minmax(0,156px)_72px] items-center gap-3 mb-2.5">
        <div className="min-w-0 flex flex-wrap items-center gap-2">
          <span className="text-[12px] text-black/[0.26]">
            {formatHistoryTimestamp(entry.created_at)}
          </span>
          <span className="text-[12px] text-black/[0.26]">&middot;</span>
          <span className="text-[12px] text-black/[0.26]">
            {entry.word_count ?? 0} words
          </span>
        </div>

        <div className="min-w-0 flex justify-end">
          {entry.app_name ? (
            <span className="max-w-full truncate px-2.5 py-1 bg-[rgba(0,88,188,0.12)] text-[#0058bc] text-[11px] font-semibold rounded-md">
              {entry.app_name}
            </span>
          ) : (
            <span className="h-6" aria-hidden="true" />
          )}
        </div>

        <div className="flex items-center justify-end gap-1">
          <CopyButton
            text={entry.final_text}
            variant="icon"
            iconSize={18}
            className="w-8 h-8 flex items-center justify-center rounded-md text-black/[0.26] hover:bg-black/[0.04] hover:text-black/85 transition-colors"
          />
          <button
            onClick={() => onDelete(entry.id)}
            className="w-8 h-8 flex items-center justify-center rounded-md text-black/[0.26] hover:bg-[#fff1f1] hover:text-[#ba1a1a] transition-colors"
          >
            <span className="material-symbols-outlined text-[18px]">delete</span>
          </button>
        </div>
      </div>

      <p className="text-[13px] leading-[1.5] text-black/85 line-clamp-2">
        {entry.final_text}
      </p>
    </div>
  );
}
