// Last dictation preview -- shows most recent transcription with actions
import type { HistoryEntry } from "@/types/commands";
import { CopyButton } from "@/components/ui/CopyButton";
import { formatRelativeTime } from "@/lib/formatters";

interface LastDictationCardProps {
  entry: HistoryEntry | null;
  onDelete: (id: string) => void;
}

function EmptyState() {
  return (
    <div className="flex flex-col items-center text-center">
      <span className="material-symbols-outlined text-[40px] text-[rgba(0,0,0,0.15)] mb-4">
        mic
      </span>
      <p className="text-[14px] font-medium text-black/85 mb-1">No dictations yet</p>
      <p className="text-[12px] text-black/[0.26]">
        Use your record hotkey to start your first dictation.
      </p>
    </div>
  );
}

export function LastDictationCard({ entry, onDelete }: LastDictationCardProps) {
  if (!entry) {
    return (
      <div className="bg-white p-6 rounded-xl border border-black/[0.07] shadow-sm min-h-[200px] flex items-center justify-center">
        <EmptyState />
      </div>
    );
  }

  return (
    <div className="bg-white p-6 rounded-xl border border-black/[0.07] shadow-sm">
      <div className="flex justify-between items-start mb-4">
        <div className="flex items-baseline gap-4">
          <h3 className="text-[10px] font-bold text-black/[0.26] tracking-[0.06em] uppercase">
            LAST DICTATION
          </h3>
          <span className="text-[12px] text-black/[0.26]">
            {formatRelativeTime(entry.created_at)}
          </span>
          {entry.app_name && (
            <span className="px-2 py-0.5 bg-[rgba(0,88,188,0.12)] text-[#0058bc] text-[11px] font-semibold rounded">
              {entry.app_name}
            </span>
          )}
        </div>
        <CopyButton
          text={entry.final_text}
          variant="icon"
          iconSize={20}
          className="text-[#0058bc] hover:bg-[rgba(0,88,188,0.12)] p-1.5 rounded-md transition-colors flex items-center justify-center"
        />
      </div>

      <div className="mb-4">
        <p className="text-[15px] font-medium italic leading-relaxed text-black/85">
          &ldquo;{entry.final_text}&rdquo;
        </p>
      </div>

      <div className="flex items-center justify-between pt-4 border-t border-black/[0.07]">
        <div className="flex items-center gap-2">
          <span className="material-symbols-outlined text-black/50 text-[18px]">
            description
          </span>
          <span className="text-[13px] font-medium text-black/50">
            {entry.word_count ?? 0} words
          </span>
        </div>
        <button
          onClick={() => onDelete(entry.id)}
          className="text-black/50 hover:bg-[#f9f9f9] p-1.5 rounded-md transition-colors"
        >
          <span className="material-symbols-outlined text-[18px]">delete</span>
        </button>
      </div>
    </div>
  );
}
