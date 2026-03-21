import type { ModelsStatus } from "@/types/commands";

interface ModelStatusCardProps {
  status: ModelsStatus | null;
  llmMode: string;
  llmLabel: string;
  isLoading: boolean;
}

export function ModelStatusCard({ status, llmMode, llmLabel, isLoading }: ModelStatusCardProps) {
  const whisperReady = status?.whisper_loaded ?? false;
  const llmReady = status?.llm_loaded ?? false;

  // Determine LLM display
  let llmDot = "bg-[#FF9500]";
  let llmText = "text-black/50";
  let llmDisplay = "No LLM — Whisper only";

  if (llmMode === "local" && llmReady) {
    llmDot = "bg-[#006b19]";
    llmText = "text-[#006b19]";
    llmDisplay = "Qwen3 0.6B";
  } else if (llmMode === "ollama" && llmLabel) {
    llmDot = "bg-[#006b19]";
    llmText = "text-[#006b19]";
    llmDisplay = `Ollama: ${llmLabel}`;
  } else if (llmMode === "byok" && llmLabel) {
    llmDot = "bg-[#006b19]";
    llmText = "text-[#006b19]";
    llmDisplay = `BYOK: ${llmLabel}`;
  }

  return (
    <div className="bg-white p-5 rounded-xl border border-black/[0.07] shadow-sm">
      <p className="text-[10px] font-bold text-black/[0.26] tracking-[0.06em] uppercase mb-2">
        MODEL STATUS
      </p>
      {/* Whisper status */}
      <div className="flex items-center justify-between w-full mb-1.5">
        <div className="flex items-center gap-2">
          {isLoading ? (
            <span className="text-[13px] text-black/50">Checking...</span>
          ) : whisperReady ? (
            <>
              <span className="w-2 h-2 rounded-full bg-[#006b19]" />
              <span className="text-[13px] font-semibold text-[#006b19]">Ready</span>
            </>
          ) : (
            <>
              <span className="w-2 h-2 rounded-full bg-[#ba1a1a]" />
              <span className="text-[13px] font-semibold text-[#ba1a1a]">Not loaded</span>
            </>
          )}
        </div>
        {!isLoading && (
          <span className="text-[12px] font-medium text-black/50">Whisper tiny.en</span>
        )}
      </div>
      {/* LLM status */}
      {!isLoading && (
        <div className="flex items-center justify-between w-full">
          <div className="flex items-center gap-2">
            <span className={`w-2 h-2 rounded-full ${llmDot}`} />
            <span className={`text-[13px] font-semibold ${llmText}`}>{llmDisplay}</span>
          </div>
        </div>
      )}
    </div>
  );
}
