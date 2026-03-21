import type { ModelsStatus } from "@/types/commands";

interface ModelStatusCardProps {
  status: ModelsStatus | null;
  isLoading: boolean;
}

export function ModelStatusCard({ status, isLoading }: ModelStatusCardProps) {
  const whisperReady = status?.whisper_loaded ?? false;

  return (
    <div className="bg-white p-5 rounded-xl border border-black/[0.07] shadow-sm">
      <p className="text-[10px] font-bold text-black/[0.26] tracking-[0.06em] uppercase mb-2">
        MODEL STATUS
      </p>
      <div className="flex items-center justify-between w-full">
        <div className="flex items-center gap-2">
          {isLoading ? (
            <span className="text-[14px] font-medium text-black/50">
              Checking...
            </span>
          ) : whisperReady ? (
            <>
              <span className="w-2 h-2 rounded-full bg-[#006b19]" />
              <span className="text-[14px] font-semibold text-[#006b19]">Ready</span>
            </>
          ) : (
            <>
              <span className="w-2 h-2 rounded-full bg-[#ba1a1a]" />
              <span className="text-[14px] font-semibold text-[#ba1a1a]">Not loaded</span>
            </>
          )}
        </div>
        {!isLoading && (
          <span className="text-[12px] font-medium text-black/50">
            Whisper tiny.en
          </span>
        )}
      </div>
    </div>
  );
}
