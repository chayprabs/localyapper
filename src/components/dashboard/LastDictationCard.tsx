import type { HistoryEntry } from "@/types/commands";
import { formatRelativeTime } from "@/lib/formatters";

interface LastDictationCardProps {
  entry: HistoryEntry | null;
  onCopy: (text: string) => void;
  onDelete: (id: string) => void;
}

function KeyBadge({ children }: { children: React.ReactNode }) {
  return (
    <span className="bg-[rgba(0,0,0,0.05)] border border-[rgba(0,0,0,0.1)] rounded px-1 py-px text-[11px] font-medium">
      {children}
    </span>
  );
}

function EmptyState() {
  const isMac = navigator.userAgent.includes("Mac");
  return (
    <div className="flex flex-col items-center text-center">
      <span className="material-symbols-outlined text-[40px] text-[rgba(0,0,0,0.15)] mb-4">
        mic
      </span>
      <p className="text-[14px] font-medium text-black/85 mb-1">No dictations yet</p>
      <p className="text-[12px] text-black/[0.26] flex items-center gap-1.5">
        Hold <KeyBadge>{isMac ? "⌥" : "Ctrl"}</KeyBadge>{" "}
        {!isMac && <><KeyBadge>Shift</KeyBadge>{" "}</>}
        <KeyBadge>Space</KeyBadge> to start your first dictation.
      </p>
    </div>
  );
}

export function LastDictationCard({ entry, onCopy, onDelete }: LastDictationCardProps) {
  if (!entry) {
    return (
      <div className="bg-white p-8 rounded-xl border border-black/[0.07] shadow-sm min-h-[300px] flex items-center justify-center">
        <EmptyState />
      </div>
    );
  }

  return (
    <div className="bg-white p-8 rounded-xl border border-black/[0.07] shadow-sm">
      {/* Header */}
      <div className="flex justify-between items-start mb-6">
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
        <button
          onClick={() => onCopy(entry.final_text)}
          className="text-[#0058bc] hover:bg-[rgba(0,88,188,0.12)] p-1.5 rounded-md transition-colors flex items-center justify-center"
        >
          <span className="material-symbols-outlined text-[20px]">content_copy</span>
        </button>
      </div>

      {/* Body */}
      <div className="mb-8">
        <p className="text-[17px] font-medium italic leading-relaxed text-black/85">
          &ldquo;{entry.final_text}&rdquo;
        </p>
      </div>

      {/* Footer */}
      <div className="flex items-center justify-between pt-6 border-t border-black/[0.07]">
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
