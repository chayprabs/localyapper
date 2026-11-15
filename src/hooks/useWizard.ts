import { DEFAULT_RECORD_HOTKEY, isMacPlatform } from "@/lib/hotkeyDefaults";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { DownloadProgress, PermissionsStatus } from "@/types/commands";
import { getAllSettings, setSetting } from "@/lib/commands/settings";
import {
  downloadSpeechModel,
  reloadModels,
  cancelModelDownload,
  checkSpeechModelFileExists,
  modelDownloadActive,
} from "@/lib/commands/models";
import {
  checkPermissions,
  openMicSettings,
  requestMicrophonePermission,
} from "@/lib/commands/system";

/**
 * Onboarding step identifiers, in display order. The last entry, `done`,
 * is the summary screen the user sees just before the wizard closes.
 */
const ALL_WIZARD_STEPS = [
  "welcome",
  "microphone",
  "accessibility",
  "hotkey",
  "files",
  "try",
  "done",
] as const;

export type WizardStep = (typeof ALL_WIZARD_STEPS)[number];

// The Accessibility grant only exists on macOS.
export const WIZARD_STEPS: readonly WizardStep[] = isMacPlatform
  ? ALL_WIZARD_STEPS
  : ALL_WIZARD_STEPS.filter((s) => s !== "accessibility");

const DEFAULT_HOTKEY = DEFAULT_RECORD_HOTKEY;

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
  // Whether the current step was reached by going back. Steps that move
  // on by themselves must not do so then, or Back could never pass them.
  const [cameBack, setCameBack] = useState(false);

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
  const [cancelling, setCancelling] = useState(false);
  const [modelSizeMb, setModelSizeMb] = useState<number | null>(null);
  // Set by Cancel so the rejection that follows is not shown as an error.
  const cancelRequested = useRef(false);
  const downloadRunning = useRef(false);

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
        if (!cancelled) {
          setFilesInstalled(status.exists);
          setModelSizeMb(status.size_mb);
        }
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
    if (next) {
      setCameBack(false);
      setStep(next);
    }
  }, [step, setStep]);

  const goBack = useCallback(() => {
    const idx = WIZARD_STEPS.indexOf(step);
    if (idx <= 0) return;
    const prev = WIZARD_STEPS[idx - 1];
    if (prev) {
      setCameBack(true);
      setStep(prev);
    }
  }, [step, setStep]);

  const canGoBack = useMemo(() => {
    if (step === "welcome") return false;
    if (step === "files" && downloading) return false;
    return true;
  }, [step, downloading]);

  // Microphone step actions
  const readPermissions = useCallback(async () => {
    try {
      setPermissions(await checkPermissions());
    } catch (error) {
      console.error("Failed to check permissions:", error);
    }
  }, []);

  const refreshPermissions = useCallback(async () => {
    setPermissionsLoading(true);
    try {
      await readPermissions();
    } finally {
      setPermissionsLoading(false);
    }
  }, [readPermissions]);

  // Entering the microphone step asks the OS for access (macOS prompts
  // once through AVFoundation), then reads the result. The answer to the
  // prompt, a microphone plugged in, or a change made in system settings
  // arrives later, so the status is read again every two seconds for as
  // long as the step is open.
  useEffect(() => {
    if (step !== "microphone") return;
    let live = true;
    void requestMicrophonePermission()
      .catch((error: unknown) => {
        console.error("Failed to request microphone access:", error);
      })
      .then(() => {
        if (live) void refreshPermissions();
      });
    const timer = window.setInterval(() => {
      void readPermissions();
    }, 2000);
    return () => {
      live = false;
      window.clearInterval(timer);
    };
  }, [step, refreshPermissions, readPermissions]);

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
    // One download at a time; a second click while one runs does nothing.
    if (downloadRunning.current) return;
    downloadRunning.current = true;
    cancelRequested.current = false;
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
        setDownloadError(
          `The download finished, but the speech engine did not start. ${message}`,
        );
      }
    } catch (error) {
      // A cancelled download is not a failure: the part is kept and the
      // step goes back to offering the download.
      if (!cancelRequested.current) {
        setDownloadError(errorToMessage(error, "Download failed"));
      }
    } finally {
      downloadRunning.current = false;
      setDownloading(false);
      setCancelling(false);
    }
  }, []);

  // The step shows "downloading" until the download has really stopped,
  // so a new one can never start while the old one still writes.
  const cancelDownload = useCallback(async () => {
    cancelRequested.current = true;
    setCancelling(true);
    try {
      await cancelModelDownload();
    } catch (error) {
      console.error("Failed to cancel the download:", error);
      setCancelling(false);
    }
  }, []);

  // A download started before this window was closed keeps running in the
  // background. Reopening the wizard joins it instead of starting over.
  useEffect(() => {
    if (!hydrated) return;
    let live = true;
    void modelDownloadActive()
      .then((active) => {
        if (live && active) void startDownload();
      })
      .catch(() => undefined);
    return () => {
      live = false;
    };
  }, [hydrated, startDownload]);

  // The hotkey was registered on its own step. Launch at login can fail
  // (some Linux packages, unsigned macOS builds); that must not keep the
  // user in the wizard, and General has the same switch for later.
  const finishWizard = useCallback(async (launchAtLogin: boolean) => {
    setSetupError(null);
    try {
      await setSetting("auto_start", launchAtLogin ? "true" : "false");
    } catch (error) {
      console.error("Failed to set launch at login:", error);
    }
    try {
      await setSetting("setup_step", "done");
      await setSetting("setup_complete", "true");
      onComplete();
    } catch (error) {
      const message = errorToMessage(error, "Failed to finish setup");
      setSetupError(message);
      console.error("Failed to finish wizard:", error);
    }
  }, [onComplete]);

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
    cameBack,
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
    cancelling,
    modelSizeMb,
    handleDownloadProgress,
    startDownload,
    cancelDownload,

    setupError,
    finishWizard,
    skipSetup,
  };
}
