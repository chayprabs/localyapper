import { useCallback, useState } from "react";
import type { DownloadProgress } from "@/types/commands";
import { setSetting } from "@/lib/commands/settings";
import {
  downloadSpeechModel,
  reloadModels,
  cancelModelDownload,
} from "@/lib/commands/models";
import { updateHotkey } from "@/lib/commands/hotkeys";

export type WizardStep =
  | "welcome"
  | "downloading"
  | "download-complete"
  | "hotkey"
  | "ready";

export function useWizard(onComplete: () => void) {
  const [step, setStep] = useState<WizardStep>("welcome");
  const [downloadProgress, setDownloadProgress] =
    useState<DownloadProgress | null>(null);
  const [downloadError, setDownloadError] = useState<string | null>(null);
  const [setupError, setSetupError] = useState<string | null>(null);
  const [hotkey, setHotkey] = useState("F8");

  const goToDownload = useCallback(() => {
    setStep("downloading");
  }, []);

  const handleDownloadProgress = useCallback((progress: DownloadProgress) => {
    setDownloadProgress(progress);
  }, []);

  const startDownload = useCallback(async () => {
    setDownloadError(null);
    setDownloadProgress(null);

    try {
      await downloadSpeechModel();
      try {
        await reloadModels();
      } catch (error) {
        const message =
          typeof error === "string"
            ? error
            : error instanceof Error
              ? error.message
              : "The speech engine did not start";
        setSetupError(
          `Download finished, but the speech engine needs attention. ${message}`,
        );
      }
      setStep("download-complete");
    } catch (error) {
      const message =
        typeof error === "string"
          ? error
          : error instanceof Error
            ? error.message
            : "Download failed";
      setDownloadError(message);
    }
  }, []);

  const cancelDownload = useCallback(async () => {
    try {
      await cancelModelDownload();
    } catch {
      // Ignore cancellation errors from already-finished downloads.
    }
    setStep("welcome");
  }, []);

  const goToHotkey = useCallback(() => {
    setStep("hotkey");
  }, []);

  const goToReady = useCallback(() => {
    setStep("ready");
  }, []);

  const finishWizard = useCallback(async () => {
    setSetupError(null);
    try {
      await reloadModels();
      await updateHotkey("hotkey_record", hotkey);
      await setSetting("setup_complete", "true");
      onComplete();
    } catch (error) {
      const message =
        typeof error === "string"
          ? error
          : error instanceof Error
            ? error.message
            : "Failed to finish setup";
      setSetupError(message);
      console.error("Failed to finish wizard:", error);
    }
  }, [hotkey, onComplete]);

  const skipSetup = useCallback(async () => {
    setSetupError(null);
    try {
      await setSetting("setup_complete", "true");
      onComplete();
    } catch (error) {
      const message =
        typeof error === "string"
          ? error
          : error instanceof Error
            ? error.message
            : "Failed to skip setup";
      setSetupError(message);
      console.error("Failed to skip setup:", error);
    }
  }, [onComplete]);

  return {
    step,
    downloadProgress,
    downloadError,
    setupError,
    hotkey,
    setHotkey,
    goToDownload,
    handleDownloadProgress,
    startDownload,
    cancelDownload,
    goToHotkey,
    goToReady,
    finishWizard,
    skipSetup,
  };
}
