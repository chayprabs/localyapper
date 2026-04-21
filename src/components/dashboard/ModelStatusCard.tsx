// Model status card -- speech model loaded indicator
import type { ModelsStatus } from "@/types/commands";

interface ModelStatusCardProps {
  status: ModelsStatus | null;
}

export function ModelStatusCard({ status }: ModelStatusCardProps) {
  const speechModelReady = status?.speech_model_loaded ?? false;
  const dotClass = speechModelReady ? "bg-[#006b19]" : "bg-black/[0.26]";
  const textClass = speechModelReady ? "text-[#006b19]" : "text-black/[0.26]";

  return (
    <div className="bg-white p-4 rounded-xl border border-black/[0.07] shadow-sm">
      <p className="text-[10px] font-bold text-black/[0.26] tracking-[0.06em] uppercase mb-1.5">
        SPEECH STATUS
      </p>
      <div className="flex items-center gap-2 mb-1.5">
        <span className={`w-2 h-2 rounded-full ${dotClass}`} />
        <span className={`text-[13px] font-medium ${textClass}`}>
          {speechModelReady ? "Engine ready" : "Engine not ready"}
        </span>
      </div>
      <p className="text-[12px] text-black/[0.40]">
        {speechModelReady
          ? "Local speech dictation is ready to use."
          : "Open Settings > Speech to download or reload the local speech package."}
      </p>
    </div>
  );
}
