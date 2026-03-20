import { useState, useCallback, useRef } from "react";
import type { OllamaStatus, ConnectionResult, DownloadProgress } from "@/types/commands";
import { setSetting } from "@/lib/commands/settings";
import {
  downloadModel,
  cancelModelDownload,
  checkOllama,
  testByokConnection,
} from "@/lib/commands/models";
import { updateHotkey } from "@/lib/commands/hotkeys";

export type WizardStep =
  | "welcome"
  | "model-selection"
  | "downloading"
  | "download-complete"
  | "ollama"
  | "byok"
  | "whisper-warning"
  | "hotkey"
  | "ready";

export type ModelChoice = "qwen" | "ollama" | "byok" | "whisper-only";

export function useWizard(onComplete: () => void) {
  const [step, setStep] = useState<WizardStep>("welcome");
  const [modelChoice, setModelChoice] = useState<ModelChoice | null>(null);

  // Download state
  const [downloadProgress, setDownloadProgress] = useState<DownloadProgress | null>(null);
  const [downloadError, setDownloadError] = useState<string | null>(null);

  // Ollama state
  const [ollamaStatus, setOllamaStatus] = useState<OllamaStatus | null>(null);
  const [ollamaModel, setOllamaModel] = useState("");
  const [ollamaLoading, setOllamaLoading] = useState(false);

  // BYOK state
  const [byokProvider, setByokProvider] = useState<"openai" | "anthropic" | "groq">("openai");
  const [byokApiKey, setByokApiKey] = useState("");
  const [byokResult, setByokResult] = useState<ConnectionResult | null>(null);
  const [byokTesting, setByokTesting] = useState(false);

  // Hotkey state
  const [hotkey, setHotkey] = useState("Alt+Space");

  const modelChoiceRef = useRef(modelChoice);
  modelChoiceRef.current = modelChoice;

  const goToWelcome = useCallback(() => {
    setStep("welcome");
  }, []);

  const goToModelSelection = useCallback(() => {
    setStep("model-selection");
  }, []);

  const selectModelAndContinue = useCallback((choice: ModelChoice) => {
    setModelChoice(choice);
    switch (choice) {
      case "qwen":
        setStep("downloading");
        break;
      case "ollama":
        setStep("ollama");
        break;
      case "byok":
        setStep("byok");
        break;
      case "whisper-only":
        setStep("whisper-warning");
        break;
    }
  }, []);

  // Download handlers
  const handleDownloadProgress = useCallback((progress: DownloadProgress) => {
    setDownloadProgress(progress);
  }, []);

  const handleDownloadComplete = useCallback(() => {
    setStep("download-complete");
  }, []);

  const handleDownloadError = useCallback((error: string) => {
    setDownloadError(error);
  }, []);

  const startDownload = useCallback(async () => {
    setDownloadError(null);
    setDownloadProgress(null);
    try {
      await downloadModel();
      setStep("download-complete");
    } catch (e) {
      setDownloadError(e instanceof Error ? e.message : "Download failed");
    }
  }, []);

  const cancelDownload = useCallback(async () => {
    try {
      await cancelModelDownload();
    } catch {
      // ignore
    }
    setStep("model-selection");
  }, []);

  // Ollama handlers
  const refreshOllama = useCallback(async () => {
    setOllamaLoading(true);
    try {
      const status = await checkOllama();
      setOllamaStatus(status);
      if (status.running && status.models.length > 0 && !ollamaModel && status.models[0]) {
        setOllamaModel(status.models[0]);
      }
    } catch {
      setOllamaStatus({ running: false, models: [] });
    } finally {
      setOllamaLoading(false);
    }
  }, [ollamaModel]);

  // BYOK handlers
  const testConnection = useCallback(async () => {
    setByokTesting(true);
    setByokResult(null);
    try {
      const result = await testByokConnection(byokProvider, byokApiKey);
      setByokResult(result);
    } catch (e) {
      setByokResult({
        success: false,
        latency_ms: 0,
        error: e instanceof Error ? e.message : "Connection failed",
      });
    } finally {
      setByokTesting(false);
    }
  }, [byokProvider, byokApiKey]);

  // Navigation
  const goToHotkey = useCallback(() => {
    setStep("hotkey");
  }, []);

  const goBack = useCallback(() => {
    setStep("model-selection");
  }, []);

  const goToReady = useCallback(() => {
    setStep("ready");
  }, []);

  const finishWizard = useCallback(async () => {
    try {
      // Save LLM mode based on choice
      const choice = modelChoiceRef.current;
      if (choice === "qwen" || choice === "whisper-only") {
        await setSetting("llm_mode", choice === "whisper-only" ? "local" : "local");
      } else if (choice === "ollama") {
        await setSetting("llm_mode", "ollama");
        if (ollamaModel) {
          await setSetting("ollama_model", ollamaModel);
        }
      } else if (choice === "byok") {
        await setSetting("llm_mode", "byok");
        await setSetting("byok_provider", byokProvider);
        await setSetting("byok_api_key", byokApiKey);
      }

      // Save hotkey
      await updateHotkey("hotkey_record", hotkey);

      // Mark setup complete
      await setSetting("setup_complete", "true");
      onComplete();
    } catch (e) {
      console.error("Failed to finish wizard:", e);
    }
  }, [ollamaModel, byokProvider, byokApiKey, hotkey, onComplete]);

  const skipSetup = useCallback(async () => {
    try {
      await setSetting("setup_complete", "true");
      onComplete();
    } catch (e) {
      console.error("Failed to skip setup:", e);
    }
  }, [onComplete]);

  return {
    step,
    modelChoice,
    // Download
    downloadProgress,
    downloadError,
    handleDownloadProgress,
    handleDownloadComplete,
    handleDownloadError,
    startDownload,
    cancelDownload,
    // Ollama
    ollamaStatus,
    ollamaModel,
    ollamaLoading,
    setOllamaModel,
    refreshOllama,
    // BYOK
    byokProvider,
    byokApiKey,
    byokResult,
    byokTesting,
    setByokProvider,
    setByokApiKey,
    testConnection,
    // Hotkey
    hotkey,
    setHotkey,
    // Navigation
    goToWelcome,
    goToModelSelection,
    selectModelAndContinue,
    goToHotkey,
    goBack,
    goToReady,
    finishWizard,
    skipSetup,
  };
}
