import { useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import type { DownloadProgress } from "@/types/commands";

export function DownloadStep({
  downloadProgress,
  downloadError,
  onProgress,
  onComplete,
  onError,
  onStartDownload,
  onCancel,
}: {
  downloadProgress: DownloadProgress | null;
  downloadError: string | null;
  onProgress: (progress: DownloadProgress) => void;
  onComplete: () => void;
  onError: (error: string) => void;
  onStartDownload: () => Promise<void>;
  onCancel: () => void;
}) {
  const startedRef = useRef(false);

  useEffect(() => {
    if (startedRef.current) return;
    startedRef.current = true;

    // Listen for progress events
    const unlistenPromise = listen<DownloadProgress>(
      "model_download_progress",
      (event) => {
        onProgress(event.payload);
      },
    );

    // Start the download
    onStartDownload().then(() => {
      onComplete();
    }).catch((e) => {
      onError(e instanceof Error ? e.message : "Download failed");
    });

    return () => {
      void unlistenPromise.then((unlisten) => unlisten());
    };
  }, [onProgress, onComplete, onError, onStartDownload]);

  const percent = downloadProgress?.percent ?? 0;
  const downloadedMb = downloadProgress?.downloaded_mb ?? 0;
  const totalMb = downloadProgress?.total_mb ?? 400;
  const speedMbps = downloadProgress?.speed_mbps ?? 0;

  return (
    <div className="flex flex-col items-center text-center">
      {/* Animated icon */}
      <div className="w-14 h-14 bg-[#0058bc]/[0.08] rounded-full flex items-center justify-center mb-5">
        <span className="material-symbols-outlined text-[28px] text-[#0058bc] animate-pulse">
          download
        </span>
      </div>

      <h2 className="text-[20px] font-semibold text-black/85 mb-1">
        Downloading Qwen 2.5
      </h2>
      <p className="text-[13px] text-black/50 mb-6">
        {downloadError
          ? "Download failed"
          : `${downloadedMb.toFixed(0)} MB / ${totalMb.toFixed(0)} MB`}
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
              onStartDownload().then(onComplete).catch((e) =>
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
