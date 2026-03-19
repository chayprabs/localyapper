const MISSING_FEATURES = [
  "Smart punctuation and capitalization",
  "Context-aware formatting",
  "Mode-based text transformation",
  "App-specific text adaptation",
];

export function WhisperWarningStep({
  onContinue,
  onBack,
}: {
  onContinue: () => void;
  onBack: () => void;
}) {
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
            warning
          </span>
        </div>

        <h2 className="text-[20px] font-semibold text-black/85 mb-1">
          Limited Functionality
        </h2>
        <p className="text-[13px] text-black/50 mb-5">
          Without a language model, these features won't be available:
        </p>

        <div className="w-full bg-black/[0.02] rounded-[10px] border border-black/[0.07] p-4 mb-6">
          {MISSING_FEATURES.map((feature) => (
            <div
              key={feature}
              className="flex items-center gap-2.5 py-1.5"
            >
              <span className="material-symbols-outlined text-[16px] text-[#ba1a1a]">
                close
              </span>
              <span className="text-[13px] text-black/85">{feature}</span>
            </div>
          ))}
        </div>

        <button
          onClick={onContinue}
          className="w-full h-9 bg-gradient-to-b from-[#0062d0] to-[#0058bc] text-white text-[13px] font-medium rounded-[8px] hover:brightness-110 active:brightness-95 transition-all mb-2.5"
        >
          Continue Anyway
        </button>

        <button
          onClick={onBack}
          className="w-full h-9 bg-white border border-black/[0.15] text-[13px] font-medium rounded-[8px] hover:bg-black/[0.02] transition-colors"
        >
          Go Back
        </button>
      </div>
    </div>
  );
}
