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
      {/* Header */}
      <div className="flex justify-between items-center mb-2.5">
        <div className="flex items-center gap-2">
          <span className="text-[12px] text-black/[0.26]">
            {formatHistoryTimestamp(entry.created_at)}
          </span>
          <span className="text-[12px] text-black/[0.26]">&middot;</span>
          <span className="text-[12px] text-black/[0.26]">
            {entry.word_count ?? 0} words
          </span>
        </div>
        <div className="flex items-center gap-3">
          {entry.app_name && (
            <span className="px-2 py-0.5 bg-[rgba(0,88,188,0.12)] text-[#0058bc] text-[11px] font-semibold rounded">
              {entry.app_name}
            </span>
          )}
          <CopyButton
            text={entry.final_text}
            variant="icon"
            iconSize={18}
            className="text-black/[0.26] hover:text-black/85 transition-colors"
          />
          <button
            onClick={() => onDelete(entry.id)}
            className="text-black/[0.26] hover:text-[#ba1a1a] transition-colors"
          >
            <span className="material-symbols-outlined text-[18px]">delete</span>
          </button>
        </div>
      </div>

      {/* Body */}
      <p className="text-[13px] leading-[1.5] text-black/85 line-clamp-2">
        {entry.final_text}
      </p>
    </div>
  );
}
