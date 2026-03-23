import type { ModelsStatus } from "@/types/commands";

interface ModelStatusCardProps {
  status: ModelsStatus | null;
  llmMode: string;
  llmLabel: string;
  isLoading: boolean;
}

export function ModelStatusCard({ status, llmMode, llmLabel }: ModelStatusCardProps) {
  const whisperReady = status?.whisper_loaded ?? false;
  const llmReady = status?.llm_loaded ?? false;

  // Determine LLM display name
  let llmName = "Qwen3 0.6B";
  if (llmMode === "ollama" && llmLabel) {
    llmName = `Ollama: ${llmLabel}`;
  } else if (llmMode === "byok" && llmLabel) {
    llmName = `BYOK: ${llmLabel}`;
  }

  const whisperDot = whisperReady ? "bg-[#006b19]" : "bg-black/[0.26]";
  const whisperText = whisperReady ? "text-[#006b19]" : "text-black/[0.26]";
  const llmDot = llmReady ? "bg-[#006b19]" : "bg-black/[0.26]";
  const llmText = llmReady ? "text-[#006b19]" : "text-black/[0.26]";

  return (
    <div className="bg-white p-4 rounded-xl border border-black/[0.07] shadow-sm">
      <p className="text-[10px] font-bold text-black/[0.26] tracking-[0.06em] uppercase mb-1.5">
        MODEL STATUS
      </p>
      <div className="flex items-center gap-2 mb-1.5">
        <span className={`w-2 h-2 rounded-full ${whisperDot}`} />
        <span className={`text-[13px] font-medium ${whisperText}`}>Whisper base.en</span>
      </div>
      <div className="flex items-center gap-2">
        <span className={`w-2 h-2 rounded-full ${llmDot}`} />
        <span className={`text-[13px] font-medium ${llmText}`}>{llmName}</span>
      </div>
    </div>
  );
}
