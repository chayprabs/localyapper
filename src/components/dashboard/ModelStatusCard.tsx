import type { OllamaStatus } from "@/types/commands";

interface ModelStatusCardProps {
  status: OllamaStatus | null;
  isLoading: boolean;
}

export function ModelStatusCard({ status, isLoading }: ModelStatusCardProps) {
  const running = status?.running ?? false;
  const modelName = status?.models[0] ?? "qwen2.5:0.5b";

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
          ) : running ? (
            <>
              <span className="w-2 h-2 rounded-full bg-[#006b19]" />
              <span className="text-[14px] font-semibold text-[#006b19]">Running</span>
            </>
          ) : (
            <>
              <span className="w-2 h-2 rounded-full bg-[#ba1a1a]" />
              <span className="text-[14px] font-semibold text-[#ba1a1a]">Offline</span>
            </>
          )}
        </div>
        {!isLoading && (
          <span className="text-[12px] font-medium text-black/50">
            {modelName}
          </span>
        )}
      </div>
    </div>
  );
}
