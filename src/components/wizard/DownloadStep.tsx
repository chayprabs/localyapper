// Wizard download step -- progress bars for Whisper and LLM downloads
import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import type { DownloadProgress } from "@/types/commands";

/** Approximate Parakeet 110M FP32 model size for progress bar calculation. */
const WHISPER_SIZE_MB = 458;
/** Combined download size: Parakeet 110M (458MB) + Qwen2.5 1.5B Q4 (1024MB). */
const TOTAL_SIZE_MB = 1482;

export function DownloadStep({
  downloadError,
  onProgress,
  onError,
  onStartDownload,
  onCancel,
}: {
  downloadProgress: DownloadProgress | null;
  downloadError: string | null;
  onProgress: (progress: DownloadProgress) => void;
  onError: (error: string) => void;
  onStartDownload: () => Promise<void>;
  onCancel: () => void;
}) {
  const startedRef = useRef(false);
  const [combinedMb, setCombinedMb] = useState(0);
  const [speedMbps, setSpeedMbps] = useState(0);

  useEffect(() => {
    if (startedRef.current) return;
    startedRef.current = true;

    let unlistenWhisperFn: (() => void) | null = null;
    let unlistenLlmFn: (() => void) | null = null;

    async function run() {
      // Register listeners FIRST and wait for them to be ready
      unlistenWhisperFn = await listen<DownloadProgress>(
        "whisper_download_progress",
        (event) => {
          setCombinedMb(event.payload.downloaded_mb);
          setSpeedMbps(event.payload.speed_mbps);
          onProgress(event.payload);
        },
      );
      unlistenLlmFn = await listen<DownloadProgress>(
        "model_download_progress",
        (event) => {
          setCombinedMb(WHISPER_SIZE_MB + event.payload.downloaded_mb);
          setSpeedMbps(event.payload.speed_mbps);
          onProgress(event.payload);
        },
      );

      // NOW start the download
      onStartDownload().catch((e) => {
        onError(
          typeof e === "string"
            ? e
            : e instanceof Error
              ? e.message
              : "Download failed",
        );
      });
    }

    run();

    return () => {
      unlistenWhisperFn?.();
      unlistenLlmFn?.();
    };
  }, [onProgress, onError, onStartDownload]);

  const percent = TOTAL_SIZE_MB > 0 ? (combinedMb / TOTAL_SIZE_MB) * 100 : 0;

  return (
    <div className="flex flex-col items-center text-center">
      {/* Animated icon */}
      <div className="w-14 h-14 bg-[#0058bc]/[0.08] rounded-full flex items-center justify-center mb-5">
        <span className="material-symbols-outlined text-[28px] text-[#0058bc] animate-pulse">
          download
        </span>
      </div>

      <h2 className="text-[20px] font-semibold text-black/85 mb-1">
        Downloading Models
      </h2>
      <p className="text-[13px] text-black/50 mb-6">
        {downloadError
          ? "Download failed"
          : `${combinedMb.toFixed(0)} MB / ${TOTAL_SIZE_MB > 0 ? TOTAL_SIZE_MB.toFixed(0) : "?"} MB`}
      </p>

      {/* Progress bar */}
      <div className="w-full h-2 rounded-full bg-black/[0.06] mb-2">
        <div
          className="h-full rounded-full bg-[#0058bc] transition-all duration-300"
          style={{ width: `${Math.min(percent, 100)}%` }}
        />
      </div>

      <div className="w-full flex justify-between text-[11px] text-black/[0.40] mb-6">
        <span>{percent.toFixed(0)}%</span>
        <span>
          {downloadError
            ? ""
            : speedMbps > 0
              ? `${speedMbps.toFixed(1)} MB/s`
              : "Starting..."}
        </span>
      </div>

      {downloadError ? (
        <div className="w-full">
          <p className="text-[13px] text-[#ba1a1a] mb-4">{downloadError}</p>
          <button
            onClick={() => {
              startedRef.current = false;
              onStartDownload().catch((e) =>
                onError(e instanceof Error ? e.message : "Download failed"),
              );
            }}
            className="w-full h-9 bg-gradient-to-b from-[#0062d0] to-[#0058bc] text-white text-[13px] font-medium rounded-[8px] hover:brightness-110 transition-all"
          >
            Retry
          </button>
        </div>
      ) : (
        <button
          onClick={onCancel}
          className="w-full h-9 bg-white border border-black/[0.15] text-[13px] font-medium rounded-[8px] hover:bg-black/[0.02] transition-colors"
        >
          Cancel
        </button>
      )}
    </div>
  );
}
