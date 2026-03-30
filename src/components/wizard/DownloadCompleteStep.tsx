// Wizard download complete -- confirmation with continue action
export function DownloadCompleteStep({
  onContinue,
}: {
  onContinue: () => void;
}) {
  return (
    <div className="flex flex-col items-center text-center">
      {/* Success check */}
      <div className="w-14 h-14 bg-[#006b19]/[0.08] rounded-full flex items-center justify-center mb-5 shadow-[0_0_20px_rgba(40,205,65,0.2)]">
        <span className="material-symbols-outlined text-[28px] text-[#006b19]">
          check_circle
        </span>
      </div>

      <h2 className="text-[20px] font-semibold text-black/85 mb-1">
        Download Complete
      </h2>
      <p className="text-[13px] text-black/50 mb-2">
        Qwen2.5 1.5B is ready to use.
      </p>
      <p className="text-[12px] text-black/[0.30] mb-8">
        qwen2.5-1.5b-instruct-q4_k_m.gguf (~1.0GB)
      </p>

      <button
        onClick={onContinue}
        className="w-full h-9 bg-gradient-to-b from-[#0062d0] to-[#0058bc] text-white text-[13px] font-medium rounded-[8px] hover:brightness-110 active:brightness-95 transition-all"
      >
        Continue
      </button>
    </div>
  );
}
