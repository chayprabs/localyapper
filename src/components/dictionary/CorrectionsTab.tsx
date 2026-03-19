import { useState } from "react";
import { useCorrections } from "@/hooks/useCorrections";

interface CorrectionsTabProps {
  showAddForm: boolean;
  onCloseAddForm: () => void;
  onSwitchToTraining: () => void;
}

function InlineAddForm({
  onSave,
  onClose,
}: {
  onSave: (rawWord: string, corrected: string) => void;
  onClose: () => void;
}) {
  const [rawWord, setRawWord] = useState("");
  const [corrected, setCorrected] = useState("");

  const handleSave = () => {
    const trimmedRaw = rawWord.trim();
    const trimmedCorrected = corrected.trim();
    if (trimmedRaw && trimmedCorrected) {
      onSave(trimmedRaw, trimmedCorrected);
      setRawWord("");
      setCorrected("");
    }
  };

  return (
    <div className="px-6 py-4 flex items-center justify-between bg-[rgba(0,88,188,0.03)]">
      <div className="flex items-center gap-4 flex-1 max-w-[800px]">
        <input
          type="text"
          value={rawWord}
          onChange={(e) => setRawWord(e.target.value)}
          placeholder="Whisper heard..."
          className="w-full h-[28px] px-3 bg-white border border-[#0058bc] rounded-[6px] text-[13px] placeholder:text-[rgba(0,0,0,0.35)] focus:ring-0 focus:outline-none"
          onKeyDown={(e) => {
            if (e.key === "Enter") handleSave();
          }}
        />
        <input
          type="text"
          value={corrected}
          onChange={(e) => setCorrected(e.target.value)}
          placeholder="Corrected to..."
          className="w-full h-[28px] px-3 bg-white border border-[#0058bc] rounded-[6px] text-[13px] placeholder:text-[rgba(0,0,0,0.35)] focus:ring-0 focus:outline-none"
          onKeyDown={(e) => {
            if (e.key === "Enter") handleSave();
          }}
        />
      </div>
      <div className="flex items-center gap-4 ml-6">
        <button
          onClick={handleSave}
          className="bg-[#0058bc] text-white text-[11px] font-medium w-[52px] h-[28px] rounded-[6px] shadow-sm hover:brightness-110 transition-all"
        >
          Save
        </button>
        <button
          onClick={onClose}
          className="flex items-center justify-center text-[rgba(0,0,0,0.35)] hover:text-black/85 transition-colors"
        >
          <span className="material-symbols-outlined text-[20px]">close</span>
        </button>
      </div>
    </div>
  );
}

function EmptyState({ onStartTraining }: { onStartTraining: () => void }) {
  return (
    <div className="flex-1 flex flex-col items-center justify-center text-center p-8">
      <div className="w-[52px] h-[52px] rounded-full bg-[rgba(0,0,0,0.05)] flex items-center justify-center mb-4">
        <span
          className="material-symbols-outlined text-[22px]"
          style={{ color: "rgba(0,0,0,0.20)" }}
        >
          description
        </span>
      </div>
      <h2 className="text-[15px] font-semibold text-[#1C1C1E] mb-2">
        No corrections learned yet
      </h2>
      <p className="text-[13px] text-[rgba(0,0,0,0.45)] mb-6">
        LocalYapper will learn from your edits automatically.
      </p>
      <button
        onClick={onStartTraining}
        className="bg-[#0058bc] text-white text-[13px] font-medium w-[140px] h-[36px] rounded-lg shadow-sm hover:brightness-110 transition-all"
      >
        Start Training
      </button>
    </div>
  );
}

export function CorrectionsTab({
  showAddForm,
  onCloseAddForm,
  onSwitchToTraining,
}: CorrectionsTabProps) {
  const {
    corrections,
    totalCount,
    isLoading,
    currentPage,
    totalPages,
    nextPage,
    prevPage,
    addNewCorrection,
    removeCorrection,
  } = useCorrections();

  const isEmpty = !isLoading && corrections.length === 0 && totalCount === 0;

  const handleSave = (rawWord: string, corrected: string) => {
    void addNewCorrection(rawWord, corrected);
    onCloseAddForm();
  };

  const handleDelete = (id: string) => {
    void removeCorrection(id);
  };

  // Empty state (with optional add form above)
  if (isEmpty) {
    return (
      <section className="bg-white rounded-xl border border-black/[0.07] shadow-sm min-h-[460px] flex flex-col overflow-hidden">
        {showAddForm && (
          <>
            <InlineAddForm onSave={handleSave} onClose={onCloseAddForm} />
            <hr className="border-t border-black/[0.07]" />
          </>
        )}
        <EmptyState onStartTraining={onSwitchToTraining} />
      </section>
    );
  }

  // Table with data
  return (
    <section className="bg-white rounded-xl shadow-sm border border-black/[0.07] overflow-hidden">
      {showAddForm && (
        <>
          <InlineAddForm onSave={handleSave} onClose={onCloseAddForm} />
          <hr className="border-t border-black/[0.07]" />
        </>
      )}

      <table className="w-full text-left border-collapse">
        <thead>
          <tr className="bg-white">
            <th className="px-6 py-4 text-[10px] uppercase font-semibold text-black/[0.26] tracking-[0.06em]">
              Whisper Heard
            </th>
            <th className="px-6 py-4 text-[10px] uppercase font-semibold text-black/[0.26] tracking-[0.06em]">
              Corrected To
            </th>
            <th className="px-6 py-4 text-[10px] uppercase font-semibold text-black/[0.26] tracking-[0.06em]">
              Times Used
            </th>
            <th className="px-6 py-4 text-[10px] uppercase font-semibold text-black/[0.26] tracking-[0.06em] text-right">
              Actions
            </th>
          </tr>
        </thead>
        <tbody className="divide-y divide-[rgba(0,0,0,0.05)]">
          {corrections.map((c) => (
            <tr
              key={c.id}
              className="h-[52px] hover:bg-[rgba(0,0,0,0.01)] transition-colors"
            >
              <td className="px-6 text-[13px] font-medium text-black/85">
                {c.raw_word}
              </td>
              <td className="px-6 text-[13px] font-medium text-[#0058bc]">
                {c.corrected}
              </td>
              <td className="px-6 text-[13px] text-black/50 font-mono">
                {c.count}
              </td>
              <td className="px-6 text-right">
                <button
                  onClick={() => handleDelete(c.id)}
                  className="text-black/[0.26] hover:text-red-500 transition-colors p-1.5 rounded-md"
                >
                  <span className="material-symbols-outlined text-[18px]">
                    delete
                  </span>
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>

      {/* Pagination Footer */}
      <footer className="px-6 py-4 flex items-center justify-between bg-[rgba(0,0,0,0.01)] border-t border-black/[0.07]">
        <span className="text-[10px] font-semibold text-black/[0.26] uppercase tracking-[0.06em]">
          {totalCount} total corrections
        </span>
        <div className="flex items-center gap-4">
          <span className="text-[12px] text-black/[0.26]">
            Page {currentPage} of {totalPages}
          </span>
          <div className="flex items-center gap-1">
            <button
              onClick={prevPage}
              disabled={currentPage <= 1}
              className={
                currentPage <= 1
                  ? "p-1 rounded opacity-30 cursor-not-allowed"
                  : "p-1 rounded hover:bg-black/5 transition-colors"
              }
            >
              <span className="material-symbols-outlined text-[20px]">
                chevron_left
              </span>
            </button>
            <button
              onClick={nextPage}
              disabled={currentPage >= totalPages}
              className={
                currentPage >= totalPages
                  ? "p-1 rounded opacity-30 cursor-not-allowed"
                  : "p-1 rounded hover:bg-black/5 transition-colors"
              }
            >
              <span className="material-symbols-outlined text-[20px]">
                chevron_right
              </span>
            </button>
          </div>
        </div>
      </footer>
    </section>
  );
}
