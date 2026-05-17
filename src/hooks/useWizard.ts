import { useCallback, useEffect, useMemo, useState } from "react";
import type { DownloadProgress, PermissionsStatus } from "@/types/commands";
import { getAllSettings, setSetting } from "@/lib/commands/settings";
import {
  downloadSpeechModel,
  reloadModels,
  cancelModelDownload,
  checkSpeechModelFileExists,
} from "@/lib/commands/models";
import { updateHotkey } from "@/lib/commands/hotkeys";
import {
  checkPermissions,
  openMicSettings,
} from "@/lib/commands/system";

/**
 * Onboarding step identifiers, in display order. The last entry, `done`,
 * is the summary screen the user sees just before the wizard closes.
 */
export const WIZARD_STEPS = [
  "welcome",
  "microphone",
  "hotkey",
  "files",
  "done",
] as const;

export type WizardStep = (typeof WIZARD_STEPS)[number];

const DEFAULT_HOTKEY = "F8";

function isWizardStep(value: string | undefined): value is WizardStep {
  if (value == null) return false;
  return (WIZARD_STEPS as readonly string[]).includes(value);
}

function errorToMessage(error: unknown, fallback: string): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  return fallback;
}

export function useWizard(onComplete: () => void) {
  const [step, setStepState] = useState<WizardStep>("welcome");
  const [hydrated, setHydrated] = useState(false);

  const [hotkey, setHotkey] = useState(DEFAULT_HOTKEY);
  const [permissions, setPermissions] = useState<PermissionsStatus | null>(
    null,
  );
  const [permissionsLoading, setPermissionsLoading] = useState(false);

  const [downloadProgress, setDownloadProgress] =
    useState<DownloadProgress | null>(null);
  const [downloadError, setDownloadError] = useState<string | null>(null);
  const [filesInstalled, setFilesInstalled] = useState(false);
  const [downloading, setDownloading] = useState(false);

  const [setupError, setSetupError] = useState<string | null>(null);

  // Load persisted setup_step + current hotkey on first mount.
  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const settings = await getAllSettings();
        if (cancelled) return;

        const persisted = settings["setup_step"];
        if (isWizardStep(persisted) && persisted !== "done") {
          setStepState(persisted);
        }

        const persistedHotkey = settings["hotkey_record"];
        if (typeof persistedHotkey === "string" && persistedHotkey.length > 0) {
          setHotkey(persistedHotkey);
        }
      } catch (error) {
        console.error("Failed to load wizard settings:", error);
      } finally {
        if (!cancelled) setHydrated(true);
      }

      try {
        const status = await checkSpeechModelFileExists();
        if (!cancelled) setFilesInstalled(status.exists);
      } catch (error) {
        console.error("Failed to check speech model files:", error);
      }
    })();

    return () => {
      cancelled = true;
    };
  }, []);

  // Persist step transitions, ignoring the noop case.
  const persistStep = useCallback(async (next: WizardStep) => {
    try {
      await setSetting("setup_step", next);
    } catch (error) {
      console.error("Failed to persist setup_step:", error);
    }
  }, []);

  const setStep = useCallback(
    (next: WizardStep) => {
      setStepState((current) => {
        if (current === next) return current;
        void persistStep(next);
        return next;
      });
    },
    [persistStep],
  );

  // Step navigation helpers
  const goNext = useCallback(() => {
    const idx = WIZARD_STEPS.indexOf(step);
    if (idx < 0 || idx >= WIZARD_STEPS.length - 1) return;
    const next = WIZARD_STEPS[idx + 1];
    if (next) setStep(next);
  }, [step, setStep]);

  const goBack = useCallback(() => {
    const idx = WIZARD_STEPS.indexOf(step);
    if (idx <= 0) return;
    const prev = WIZARD_STEPS[idx - 1];
    if (prev) setStep(prev);
  }, [step, setStep]);

  const canGoBack = useMemo(() => {
    if (step === "welcome") return false;
    if (step === "files" && downloading) return false;
    return true;
  }, [step, downloading]);

  // Microphone step actions
  const refreshPermissions = useCallback(async () => {
    setPermissionsLoading(true);
    try {
      const status = await checkPermissions();
      setPermissions(status);
    } catch (error) {
      console.error("Failed to check permissions:", error);
    } finally {
      setPermissionsLoading(false);
    }
  }, []);

  const requestOpenMicSettings = useCallback(async () => {
    try {
      await openMicSettings();
    } catch (error) {
      console.error("Failed to open mic settings:", error);
    }
  }, []);

  // Speech files step actions
  const handleDownloadProgress = useCallback((progress: DownloadProgress) => {
    setDownloadProgress(progress);
  }, []);

  const startDownload = useCallback(async () => {
    setDownloadError(null);
    setDownloadProgress(null);
    setDownloading(true);

    try {
      await downloadSpeechModel();
      setFilesInstalled(true);
      try {
        await reloadModels();
      } catch (error) {
        const message = errorToMessage(
          error,
          "The speech engine did not start",
        );
        setSetupError(
          `Download finished, but the speech engine needs attention. ${message}`,
        );
      }
    } catch (error) {
      const message = errorToMessage(error, "Download failed");
      setDownloadError(message);
    } finally {
      setDownloading(false);
    }
  }, []);

  const cancelDownload = useCallback(async () => {
    try {
      await cancelModelDownload();
    } catch {
      // Ignore cancellation errors from already-finished downloads.
    }
    setDownloading(false);
    setDownloadProgress(null);
  }, []);

  const finishWizard = useCallback(async () => {
    setSetupError(null);
    try {
      await updateHotkey("hotkey_record", hotkey);
      await setSetting("setup_step", "done");
      await setSetting("setup_complete", "true");
      onComplete();
    } catch (error) {
      const message = errorToMessage(error, "Failed to finish setup");
      setSetupError(message);
      console.error("Failed to finish wizard:", error);
    }
  }, [hotkey, onComplete]);

  const skipSetup = useCallback(async () => {
    setSetupError(null);
    try {
      await setSetting("setup_step", "done");
      await setSetting("setup_complete", "true");
      onComplete();
    } catch (error) {
      const message = errorToMessage(error, "Failed to skip setup");
      setSetupError(message);
      console.error("Failed to skip setup:", error);
    }
  }, [onComplete]);

  return {
    hydrated,
    step,
    stepIndex: WIZARD_STEPS.indexOf(step),
    stepCount: WIZARD_STEPS.length,
    canGoBack,
    goNext,
    goBack,

    hotkey,
    setHotkey,

    permissions,
    permissionsLoading,
    refreshPermissions,
    requestOpenMicSettings,

    downloadProgress,
    downloadError,
    filesInstalled,
    downloading,
    handleDownloadProgress,
    startDownload,
    cancelDownload,

    setupError,
    finishWizard,
    skipSetup,
  };
}
