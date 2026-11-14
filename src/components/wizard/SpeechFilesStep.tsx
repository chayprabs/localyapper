// Wizard speech files step -- download progress and ready confirmation
import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import type { DownloadProgress } from "@/types/commands";
import { Button } from "@/components/ui/Button";
import { Notice } from "@/components/settings/controls";
import { Step } from "./Step";

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
      <Step
        key="ready"
        icon="check_circle"
        tone="ok"
        title="Speech model ready"
        text="The Parakeet model is on this computer. Dictation works offline from here on."
      >
        <Button variant="primary" size="lg" className="w-full" onClick={onContinue}>
          Continue
        </Button>
      </Step>
    );
  }

  if (downloading) {
    const percent = Math.min(downloadProgress?.percent ?? 0, 100);
    const downloadedMb = downloadProgress?.downloaded_mb ?? 0;
    const totalMb = downloadProgress?.total_mb ?? TOTAL_SIZE_MB;
    const speedMbps = downloadProgress?.speed_mbps ?? 0;

    return (
      <Step
        key="downloading"
        icon="download"
        title="Downloading the speech model"
        text="This happens once. You can keep using your computer."
      >
        <div className="w-full">
          <div className="text-[34px] font-semibold leading-none tracking-[-0.03em] tabular-nums text-ink">
            {percent.toFixed(0)}
            <span className="ml-0.5 text-[18px] text-ink/40">%</span>
          </div>
          <div className="mt-4 h-1.5 w-full overflow-hidden rounded-full bg-ink/[0.08]">
            <div
              className="h-full rounded-full bg-accent transition-[width] duration-300 ease-out"
              style={{ width: `${percent}%` }}
            />
          </div>
          <div className="mt-2.5 flex justify-between text-[12px] tabular-nums text-ink/45">
            <span>
              {downloadedMb.toFixed(0)} of {totalMb.toFixed(0)} MB
            </span>
            <span>{speedMbps > 0 ? `${speedMbps.toFixed(1)} MB/s` : "Starting"}</span>
          </div>
        </div>
        <Button size="lg" className="w-full" onClick={() => void onCancel()}>
          Cancel
        </Button>
      </Step>
    );
  }

  return (
    <Step
      key="start"
      icon="download"
      title="Get the speech model"
      text="Parakeet, about 458 MB, runs entirely on this computer. After this one download, dictation works offline."
    >
      {downloadError && <Notice>{downloadError}</Notice>}
      <Button variant="primary" size="lg" className="w-full" onClick={() => void onStartDownload()}>
        {downloadError ? "Try again" : "Download"}
      </Button>
    </Step>
  );
}
