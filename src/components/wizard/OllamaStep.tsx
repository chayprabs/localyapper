import { useEffect, useState, useRef } from "react";
import type { OllamaStatus } from "@/types/commands";

export function OllamaStep({
  ollamaStatus,
  ollamaModel,
  ollamaLoading,
  onModelChange,
  onRefresh,
  onContinue,
  onBack,
}: {
  ollamaStatus: OllamaStatus | null;
  ollamaModel: string;
  ollamaLoading: boolean;
  onModelChange: (model: string) => void;
  onRefresh: () => Promise<void>;
  onContinue: () => void;
  onBack: () => void;
}) {
  const checkedRef = useRef(false);
  const [dropdownOpen, setDropdownOpen] = useState(false);
  const dropdownRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!checkedRef.current) {
      checkedRef.current = true;
      void onRefresh();
    }
  }, [onRefresh]);

  useEffect(() => {
    if (!dropdownOpen) return;
    function handleClick(e: MouseEvent) {
      if (dropdownRef.current && !dropdownRef.current.contains(e.target as Node)) {
        setDropdownOpen(false);
      }
    }
    document.addEventListener("mousedown", handleClick);
    return () => document.removeEventListener("mousedown", handleClick);
  }, [dropdownOpen]);

  const detected = ollamaStatus?.running ?? false;
  const models = ollamaStatus?.models ?? [];

  if (ollamaLoading && !ollamaStatus) {
    return (
      <div className="flex flex-col items-center text-center">
        <div className="w-14 h-14 bg-black/[0.04] rounded-full flex items-center justify-center mb-5">
          <span className="material-symbols-outlined text-[28px] text-black/[0.30] animate-spin">
            progress_activity
          </span>
        </div>
        <h2 className="text-[20px] font-semibold text-black/85 mb-1">
          Checking for Ollama...
        </h2>
      </div>
    );
  }

  if (!detected) {
    return (
      <div>
        <button
          onClick={onBack}
          className="flex items-center gap-1 text-[13px] text-black/[0.40] hover:text-black/60 transition-colors mb-4"
        >
          <span className="material-symbols-outlined text-[16px]">arrow_back</span>
          Back
        </button>

        <div className="flex flex-col items-center text-center">
          <div className="w-14 h-14 bg-[#ba1a1a]/[0.08] rounded-full flex items-center justify-center mb-5">
            <span className="material-symbols-outlined text-[28px] text-[#ba1a1a]">
              error
            </span>
          </div>

          <h2 className="text-[20px] font-semibold text-black/85 mb-1">
            Ollama Not Detected
          </h2>
          <p className="text-[13px] text-black/50 mb-6">
            Make sure Ollama is installed and running.
          </p>

          <div className="w-full bg-black/[0.02] rounded-[10px] border border-black/[0.07] p-4 mb-6 text-left">
            <div className="flex items-start gap-3 mb-3">
              <span className="w-5 h-5 bg-black/[0.06] rounded-full flex items-center justify-center text-[11px] font-semibold text-black/50 shrink-0 mt-0.5">
                1
              </span>
              <span className="text-[13px] text-black/85">
                Install Ollama from{" "}
                <span className="font-medium text-[#0058bc]">ollama.com</span>
              </span>
            </div>
            <div className="flex items-start gap-3 mb-3">
              <span className="w-5 h-5 bg-black/[0.06] rounded-full flex items-center justify-center text-[11px] font-semibold text-black/50 shrink-0 mt-0.5">
                2
              </span>
              <span className="text-[13px] text-black/85">
                Start the Ollama service
              </span>
            </div>
            <div className="flex items-start gap-3">
              <span className="w-5 h-5 bg-black/[0.06] rounded-full flex items-center justify-center text-[11px] font-semibold text-black/50 shrink-0 mt-0.5">
                3
              </span>
              <span className="text-[13px] text-black/85">
                Pull a model:{" "}
                <code className="text-[12px] bg-black/[0.04] px-1.5 py-0.5 rounded font-mono">
                  ollama pull qwen2.5:0.5b
                </code>
              </span>
            </div>
          </div>

          <button
            onClick={() => {
              checkedRef.current = false;
              void onRefresh();
            }}
            disabled={ollamaLoading}
            className="w-full h-9 bg-gradient-to-b from-[#0062d0] to-[#0058bc] text-white text-[13px] font-medium rounded-[8px] hover:brightness-110 transition-all disabled:opacity-50"
          >
            {ollamaLoading ? "Checking..." : "Retry"}
          </button>
        </div>
      </div>
    );
  }

  // Detected
  return (
    <div>
      <button
        onClick={onBack}
        className="flex items-center gap-1 text-[13px] text-black/[0.40] hover:text-black/60 transition-colors mb-4"
      >
        <span className="material-symbols-outlined text-[16px]">arrow_back</span>
        Back
      </button>

      <div className="flex flex-col items-center text-center">
        <div className="w-14 h-14 bg-[#006b19]/[0.08] rounded-full flex items-center justify-center mb-5">
          <span className="material-symbols-outlined text-[28px] text-[#006b19]">
            check_circle
          </span>
        </div>

        <h2 className="text-[20px] font-semibold text-black/85 mb-1">
          Ollama Detected
        </h2>
        <p className="text-[13px] text-black/50 mb-6">
          {models.length} model{models.length !== 1 ? "s" : ""} available
        </p>

        {/* Model picker */}
        <div className="w-full mb-6" ref={dropdownRef}>
          <label className="block text-[12px] text-black/[0.40] mb-1.5 text-left">
            Select a model
          </label>
          <div className="relative">
            <div
              onClick={() => setDropdownOpen(!dropdownOpen)}
              className="w-full h-9 bg-white border border-black/[0.12] rounded-[8px] px-3 flex items-center justify-between cursor-pointer hover:border-black/20"
            >
              <span className="text-[13px] text-black/85 truncate">
                {ollamaModel || "Choose a model..."}
              </span>
              <span className="material-symbols-outlined text-[16px] text-black/[0.40]">
                expand_more
              </span>
            </div>
            {dropdownOpen && (
              <div className="absolute left-0 right-0 top-full mt-1 bg-white rounded-lg border border-black/[0.07] shadow-lg z-10 py-1 max-h-[200px] overflow-y-auto">
                {models.map((model) => (
                  <div
                    key={model}
                    onClick={() => {
                      onModelChange(model);
                      setDropdownOpen(false);
                    }}
                    className={`px-3 py-1.5 text-[13px] cursor-pointer hover:bg-black/[0.04] ${
                      model === ollamaModel
                        ? "text-[#0058bc] font-medium"
                        : "text-black/85"
                    }`}
                  >
                    {model}
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>

        <button
          onClick={onContinue}
          disabled={!ollamaModel}
          className="w-full h-9 bg-gradient-to-b from-[#0062d0] to-[#0058bc] text-white text-[13px] font-medium rounded-[8px] hover:brightness-110 transition-all disabled:opacity-40 disabled:cursor-not-allowed"
        >
          Continue
        </button>
      </div>
    </div>
  );
}
