interface TrainingCompleteProps {
  correctionsCount: number;
  onDone: () => void;
}

export function TrainingComplete({
  correctionsCount,
  onDone,
}: TrainingCompleteProps) {
  return (
    <div className="flex flex-col items-center py-8">
      {/* Green checkmark circle */}
      <div
        className="w-[56px] h-[56px] bg-[#28CD41] rounded-full flex items-center justify-center text-white mb-5"
        style={{ boxShadow: "0 4px 16px rgba(40,205,65,0.25)" }}
      >
        <span
          className="material-symbols-outlined text-[32px]"
          style={{ fontVariationSettings: "'wght' 600" }}
        >
          check
        </span>
      </div>

      <h2 className="text-[20px] font-semibold text-[#1C1C1E] mb-2.5">
        Your voice profile is ready
      </h2>
      <p className="text-[13px] text-black/50 mb-8">
        LocalYapper learned{" "}
        <span className="font-semibold text-[#1C1C1E]">
          {correctionsCount}
        </span>{" "}
        corrections from your voice.
      </p>

      <button
        onClick={onDone}
        className="bg-[#0058bc] text-white text-[13px] font-medium w-[120px] h-[36px] rounded-[8px] shadow-sm hover:brightness-110 transition-all"
      >
        Done
      </button>
    </div>
  );
}
