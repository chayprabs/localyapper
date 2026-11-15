// Wizard speech files step -- download progress and ready confirmation
import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import type { DownloadProgress } from "@/types/commands";
import { Button } from "@/components/ui/Button";
import { Notice } from "@/components/settings/controls";
import { Step } from "./Step";

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
  useEffect(() => {
    // Guard against the cleanup running before the async listen()
    // resolves (fast unmount during navigation). If `cancelled` is true
    // by the time we have the unlisten handle, call it immediately so
    // we don't leak a dead subscription.
    let cancelled = false;
    let unlisten: (() => void) | null = null;

    void listen<DownloadProgress>("speech_model_download_progress", (event) => {
      onProgress(event.payload);
    }).then((dispose) => {
      if (cancelled) {
        dispose();
      } else {
        unlisten = dispose;
      }
    });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [onProgress]);

  if (filesInstalled && !downloading) {
    return (
      <Step key="ready" title="Speech model" text="Installed.">
        <Button variant="primary" size="lg" onClick={onContinue}>
          Continue
        </Button>
      </Step>
    );
  }

  if (downloading) {
    const percent = Math.min(downloadProgress?.percent ?? 0, 100);
    const downloadedMb = downloadProgress?.downloaded_mb ?? 0;
    const totalMb = downloadProgress?.total_mb ?? 0;
    const speedMbps = downloadProgress?.speed_mbps ?? 0;

    return (
      <Step key="downloading" title="Speech model">
        <div className="w-full font-mono">
          <div className="text-[40px] leading-none tracking-[-0.03em] text-ink">
            {percent.toFixed(0)}
            <span className="text-ink/35">%</span>
          </div>
          <div className="mt-5 h-px w-full bg-ink/15">
            <div className="-mt-px h-[3px] bg-ink transition-[width] duration-300 ease-out" style={{ width: `${percent}%` }} />
          </div>
          <div className="mt-3 flex justify-between text-[11.5px] text-ink/45">
            <span>{totalMb > 0 ? `${downloadedMb.toFixed(0)} / ${totalMb.toFixed(0)} MB` : "Starting"}</span>
            <span>{speedMbps > 0 ? `${speedMbps.toFixed(1)} MB/s` : ""}</span>
          </div>
        </div>
        <Button size="lg" onClick={() => void onCancel()}>
          Cancel
        </Button>
      </Step>
    );
  }

  return (
    <Step key="start" title="Speech model" text="One download, then it runs offline.">
      {downloadError && <Notice>{downloadError}</Notice>}
      <Button variant="primary" size="lg" onClick={() => void onStartDownload()}>
        {downloadError ? "Try again" : "Download"}
      </Button>
    </Step>
  );
}
