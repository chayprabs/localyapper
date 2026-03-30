// Wizard model selection -- Local, Ollama, BYOK, or Whisper-only
import { useState } from "react";
import type { ModelChoice } from "@/hooks/useWizard";

const MODEL_OPTIONS: {
  value: ModelChoice;
  icon: string;
  title: string;
  description: string;
}[] = [
  {
    value: "qwen",
    icon: "download",
    title: "Download Qwen2.5 1.5B (Recommended)",
    description: "~1.0GB local model for smart text formatting",
  },
  {
    value: "ollama",
    icon: "hub",
    title: "Use Ollama",
    description: "Connect to your local Ollama instance",
  },
  {
    value: "byok",
    icon: "key",
    title: "Bring Your Own Key",
    description: "OpenAI, Anthropic, or Groq API key",
  },
  {
    value: "whisper-only",
    icon: "mic",
    title: "Whisper Only",
    description: "Raw transcription without LLM formatting",
  },
];

export function ModelSelectionStep({
  onSelect,
  onBack,
}: {
  onSelect: (choice: ModelChoice) => void;
  onBack: () => void;
}) {
  const [selected, setSelected] = useState<ModelChoice | null>(null);

  return (
    <div>
      <button
        onClick={onBack}
        className="flex items-center gap-1 text-[13px] text-black/[0.40] hover:text-black/60 transition-colors mb-4"
      >
        <span className="material-symbols-outlined text-[16px]">
          arrow_back
        </span>
        Back
      </button>

      <h2 className="text-[20px] font-semibold text-black/85 mb-1">
        Choose your language model
      </h2>
      <p className="text-[13px] text-black/50 mb-5">
        This model improves transcription with smart formatting and
        punctuation.
      </p>

      <div className="flex flex-col gap-2.5 mb-6">
        {MODEL_OPTIONS.map((opt) => (
          <button
            key={opt.value}
            onClick={() => setSelected(opt.value)}
            className={`w-full flex items-center gap-3.5 p-3.5 rounded-[10px] border text-left transition-all ${
              selected === opt.value
                ? "border-[#0058bc] bg-[#0058bc]/[0.04]"
                : "border-black/[0.07] bg-white hover:border-black/[0.15]"
            }`}
          >
            <div
              className={`w-9 h-9 rounded-[8px] flex items-center justify-center shrink-0 ${
                selected === opt.value
                  ? "bg-[#0058bc]/[0.10]"
                  : "bg-black/[0.04]"
              }`}
            >
              <span
                className={`material-symbols-outlined text-[20px] ${
                  selected === opt.value
                    ? "text-[#0058bc]"
                    : "text-black/[0.40]"
                }`}
              >
                {opt.icon}
              </span>
            </div>
            <div className="flex flex-col min-w-0">
              <span className="text-[13px] font-semibold text-black/85">
                {opt.title}
              </span>
              <span className="text-[12px] text-black/[0.40]">
                {opt.description}
              </span>
            </div>
          </button>
        ))}
      </div>

      <button
        onClick={() => selected && onSelect(selected)}
        disabled={!selected}
        className="w-full h-9 bg-gradient-to-b from-[#0062d0] to-[#0058bc] text-white text-[13px] font-medium rounded-[8px] hover:brightness-110 active:brightness-95 transition-all disabled:opacity-40 disabled:cursor-not-allowed"
      >
        Continue
      </button>
    </div>
  );
}
