// Model status card -- speech engine state, model name, and file size
import type { ModelsStatus, SpeechModelFileStatus } from "@/types/commands";

interface ModelStatusCardProps {
  status: ModelsStatus | null;
  fileStatus: SpeechModelFileStatus | null;
}

function formatModelName(name: string | undefined): string {
  switch (name) {
    case "parakeet-110m":
      return "Parakeet 110M";
    case "parakeet-0.6b":
      return "Parakeet 0.6B";
    default:
      return "Local speech engine";
  }
}

export function ModelStatusCard({ status, fileStatus }: ModelStatusCardProps) {
  const speechModelReady = status?.speech_model_loaded ?? false;
  const filesInstalled = fileStatus?.exists ?? false;

  const tone = speechModelReady
    ? "ready"
    : filesInstalled
      ? "needs-load"
      : "missing";

  const dotClass =
    tone === "ready"
      ? "bg-[#006b19]"
      : tone === "needs-load"
        ? "bg-[#ff9500]"
        : "bg-black/[0.26]";
  const labelClass =
    tone === "ready"
      ? "text-[#006b19]"
      : tone === "needs-load"
        ? "text-[#9a5a00]"
        : "text-black/[0.45]";
  const label =
    tone === "ready"
      ? "Engine ready"
      : tone === "needs-load"
        ? "Engine not loaded"
        : "Files not installed";

  return (
    <div className="rounded-xl border border-black/[0.07] bg-white p-4 shadow-sm">
      <p className="mb-1.5 text-[10px] font-bold uppercase tracking-[0.06em] text-black/[0.26]">
        SPEECH STATUS
      </p>
      <div className="mb-1.5 flex items-center gap-2">
        <span className={`h-2 w-2 rounded-full ${dotClass}`} />
        <span className={`text-[13px] font-medium ${labelClass}`}>
          {label}
        </span>
      </div>
      <p className="text-[12px] text-black/[0.50]">
        {formatModelName(fileStatus?.model_name)}
        {filesInstalled && fileStatus
          ? ` \u00b7 ${fileStatus.size_mb} MB on disk`
          : ""}
      </p>
    </div>
  );
}
