// Wizard speech files step -- download progress and ready confirmation
import { useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import type { DownloadProgress } from "@/types/commands";
import { Icon } from "@/components/ui/Icon";

const TOTAL_SIZE_MB = 458;

interface SpeechFilesStepProps {
  filesInstalled: boolean;
  downloading: boolean;
  downloadProgress: DownloadProgress | null;
  downloadError: string | null;
  onProgress: (progress: DownloadProgress) => void;
  onStartDownload: () => Promise<void>;
  onCancel: () => Promise<void>;
  onContinue: () => void;
}

export function SpeechFilesStep({
  filesInstalled,
  downloading,
  downloadProgress,
  downloadError,
  onProgress,
  onStartDownload,
  onCancel,
  onContinue,
}: SpeechFilesStepProps) {
  const subscribedRef = useRef(false);

  useEffect(() => {
    if (subscribedRef.current) return;
    subscribedRef.current = true;

    let unlisten: (() => void) | null = null;
    void (async () => {
      unlisten = await listen<DownloadProgress>(
        "speech_model_download_progress",
        (event) => {
          onProgress(event.payload);
        },
      );
    })();

    return () => {
      unlisten?.();
    };
  }, [onProgress]);

  if (filesInstalled && !downloading) {
    return (
      <div className="flex flex-col items-center text-center">
        <div className="mb-5 flex h-16 w-16 items-center justify-center rounded-2xl bg-[#006b19]/[0.10] shadow-[0_0_24px_rgba(40,205,65,0.18)]">
          <Icon name="check_circle" size={32} className="text-[#006b19]" />
        </div>
        <h2 className="mb-2 text-[22px] font-semibold text-black/85">
          Speech files ready
        </h2>
        <p className="mb-6 max-w-[360px] text-[13px] leading-relaxed text-black/50">
          The local Parakeet speech engine is installed on this device.
          Dictation will start the moment you press your hotkey.
        </p>

        <button
          type="button"
          onClick={onContinue}
          className="h-9 w-full rounded-[8px] bg-gradient-to-b from-[#0062d0] to-[#0058bc] text-[13px] font-medium text-white transition-all hover:brightness-110 active:brightness-95"
        >
          Continue
        </button>
      </div>
    );
  }

  if (downloading) {
    const percent = downloadProgress?.percent ?? 0;
    const downloadedMb = downloadProgress?.downloaded_mb ?? 0;
    const totalMb = downloadProgress?.total_mb ?? TOTAL_SIZE_MB;
    const speedMbps = downloadProgress?.speed_mbps ?? 0;

    return (
      <div className="flex flex-col items-center text-center">
        <div className="mb-5 flex h-16 w-16 items-center justify-center rounded-2xl bg-[#0058bc]/[0.10]">
          <Icon name="download" size={32} className="text-[#0058bc] animate-pulse" />
        </div>

        <h2 className="mb-2 text-[22px] font-semibold text-black/85">
          Downloading speech files
        </h2>
        <p className="mb-6 text-[13px] leading-relaxed text-black/50">
          {downloadedMb.toFixed(0)} MB / {totalMb.toFixed(0)} MB
        </p>

        <div className="mb-2 h-2 w-full rounded-full bg-black/[0.06]">
          <div
            className="h-full rounded-full bg-[#0058bc] transition-all duration-300"
            style={{ width: `${Math.min(percent, 100)}%` }}
          />
        </div>

        <div className="mb-6 flex w-full justify-between text-[11px] text-black/[0.40]">
          <span>{percent.toFixed(0)}%</span>
          <span>
            {speedMbps > 0 ? `${speedMbps.toFixed(1)} MB/s` : "Starting..."}
          </span>
        </div>

        <button
          type="button"
          onClick={() => {
            void onCancel();
          }}
          className="h-9 w-full rounded-[8px] border border-black/[0.10] bg-white text-[13px] font-medium text-black/75 transition-colors hover:bg-black/[0.02]"
        >
          Cancel download
        </button>
      </div>
    );
  }

  return (
    <div className="flex flex-col items-center text-center">
      <div className="mb-5 flex h-16 w-16 items-center justify-center rounded-2xl bg-[#0058bc]/[0.10]">
        <Icon name="cloud_download" size={32} className="text-[#0058bc]" />
      </div>

      <h2 className="mb-2 text-[22px] font-semibold text-black/85">
        Install the speech engine
      </h2>
      <p className="mb-6 max-w-[360px] text-[13px] leading-relaxed text-black/50">
        Parakeet (~458 MB) runs entirely on your device. After this one-time
        download, dictation works offline and forever.
      </p>

      {downloadError && (
        <p className="mb-4 w-full rounded-lg border border-[#ba1a1a]/15 bg-[#ba1a1a]/[0.06] px-3 py-2 text-[12px] font-medium text-[#ba1a1a]">
          {downloadError}
        </p>
      )}

      <button
        type="button"
        onClick={() => {
          void onStartDownload();
        }}
        className="h-9 w-full rounded-[8px] bg-gradient-to-b from-[#0062d0] to-[#0058bc] text-[13px] font-medium text-white transition-all hover:brightness-110 active:brightness-95"
      >
        {downloadError ? "Retry download" : "Start download"}
      </button>
    </div>
  );
}
