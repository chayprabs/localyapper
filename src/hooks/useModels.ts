import { useState, useEffect, useCallback, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import type { OllamaStatus, ConnectionResult, DownloadProgress, LlmFileStatus, WhisperFileStatus } from "@/types/commands";
import { getAllSettings, setSetting } from "@/lib/commands/settings";
import {
  checkOllama,
  testByokConnection,
  checkModelsStatus,
  checkLlmFileExists,
  checkWhisperFileExists,
  downloadModel,
  downloadWhisperModel,
  deleteLlmModel,
  deleteWhisperModel,
  cancelModelDownload,
  reloadModels,
} from "@/lib/commands/models";

type LlmMode = "local" | "ollama" | "byok";
type WhisperModel = "tiny.en" | "base.en" | "small.en" | "medium.en";
type ByokProvider = "openai" | "anthropic" | "groq";

interface ModelsState {
  whisperModel: WhisperModel;
  llmMode: LlmMode;
  ollamaModel: string;
  byokProvider: ByokProvider;
  byokApiKey: string;
}

const DEFAULTS: ModelsState = {
  whisperModel: "base.en",
  llmMode: "local",
  ollamaModel: "qwen3:0.6b",
  byokProvider: "openai",
  byokApiKey: "",
};

export function useModels() {
  const [settings, setSettings] = useState<ModelsState>(DEFAULTS);
  const [ollamaStatus, setOllamaStatus] = useState<OllamaStatus | null>(null);
  const [connectionResult, setConnectionResult] =
    useState<ConnectionResult | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isTesting, setIsTesting] = useState(false);
  const settingsRef = useRef(settings);
  settingsRef.current = settings;

  // Local LLM state
  const [llmFileStatus, setLlmFileStatus] = useState<LlmFileStatus>({ exists: false, size_mb: 0 });
  const [llmLoaded, setLlmLoaded] = useState(false);
  const [llmDownloading, setLlmDownloading] = useState(false);
  const [llmDownloadProgress, setLlmDownloadProgress] = useState<DownloadProgress | null>(null);

  // Whisper state
  const [whisperFileStatus, setWhisperFileStatus] = useState<WhisperFileStatus>({ exists: false, size_mb: 0, model_name: "base.en" });
  const [whisperLoaded, setWhisperLoaded] = useState(false);
  const [whisperDownloading, setWhisperDownloading] = useState(false);
  const [whisperDownloadProgress, setWhisperDownloadProgress] = useState<DownloadProgress | null>(null);

  const refreshOllama = useCallback(async () => {
    try {
      const status = await checkOllama();
      setOllamaStatus(status);
    } catch (e) {
      console.error("Failed to check Ollama:", e);
    }
  }, []);

  useEffect(() => {
    async function load() {
      const [settingsResult, ollamaResult, modelsResult, llmFileResult, whisperFileResult] = await Promise.allSettled([
        getAllSettings(),
        checkOllama(),
        checkModelsStatus(),
        checkLlmFileExists(),
        checkWhisperFileExists(),
      ]);

      if (settingsResult.status === "fulfilled") {
        const s = settingsResult.value;
        setSettings({
          whisperModel:
            (s["whisper_model"] as WhisperModel) ?? DEFAULTS.whisperModel,
          llmMode: (s["llm_mode"] as LlmMode) ?? DEFAULTS.llmMode,
          ollamaModel: s["ollama_model"] ?? DEFAULTS.ollamaModel,
          byokProvider:
            (s["byok_provider"] as ByokProvider) ?? DEFAULTS.byokProvider,
          byokApiKey: s["byok_api_key"] ?? DEFAULTS.byokApiKey,
        });
      }

      if (ollamaResult.status === "fulfilled") {
        setOllamaStatus(ollamaResult.value);
      }

      if (modelsResult.status === "fulfilled") {
        setLlmLoaded(modelsResult.value.llm_loaded);
        setWhisperLoaded(modelsResult.value.whisper_loaded);
      }

      if (llmFileResult.status === "fulfilled") {
        setLlmFileStatus(llmFileResult.value);
      }

      if (whisperFileResult.status === "fulfilled") {
        setWhisperFileStatus(whisperFileResult.value);
      }

      setIsLoading(false);
    }

    void load();
  }, []);

  const updateSetting = useCallback(
    async <K extends keyof ModelsState>(
      key: K,
      value: ModelsState[K],
      settingKey: string,
    ) => {
      const previous = settingsRef.current[key];
      setSettings((prev) => ({ ...prev, [key]: value }));
      try {
        await setSetting(settingKey, String(value));
      } catch (e) {
        console.error(`Failed to update ${settingKey}:`, e);
        setSettings((prev) => ({ ...prev, [key]: previous }));
      }
    },
    [],
  );

  const setLlmMode = useCallback(
    (mode: LlmMode) => {
      updateSetting("llmMode", mode, "llm_mode");
      if (mode !== "byok") setConnectionResult(null);
      if (mode === "ollama") void refreshOllama();
    },
    [updateSetting, refreshOllama],
  );

  const setOllamaModel = useCallback(
    (model: string) =>
      updateSetting("ollamaModel", model, "ollama_model"),
    [updateSetting],
  );

  const setByokProvider = useCallback(
    (provider: ByokProvider) => {
      updateSetting("byokProvider", provider, "byok_provider");
      setConnectionResult(null);
    },
    [updateSetting],
  );

  const setByokApiKey = useCallback(
    (key: string) => {
      updateSetting("byokApiKey", key, "byok_api_key");
      setConnectionResult(null);
    },
    [updateSetting],
  );

  const testConnection = useCallback(async () => {
    const { byokProvider, byokApiKey } = settingsRef.current;
    setIsTesting(true);
    setConnectionResult(null);
    try {
      const result = await testByokConnection(byokProvider, byokApiKey);
      setConnectionResult(result);
    } catch (e) {
      setConnectionResult({
        success: false,
        latency_ms: 0,
        error: e instanceof Error ? e.message : "Connection failed",
      });
    } finally {
      setIsTesting(false);
    }
  }, []);

  const downloadLocalModel = useCallback(async () => {
    setLlmDownloading(true);
    setLlmDownloadProgress(null);
    const unlisten = await listen<DownloadProgress>("model_download_progress", (event) => {
      setLlmDownloadProgress(event.payload);
    });
    try {
      await downloadModel();
      await reloadModels();
      setLlmFileStatus({ exists: true, size_mb: 397 });
      setLlmLoaded(true);
    } catch (e) {
      console.error("Model download failed:", e);
    } finally {
      unlisten();
      setLlmDownloading(false);
    }
  }, []);

  const cancelLocalModelDownload = useCallback(async () => {
    try {
      await cancelModelDownload();
    } catch {
      // ignore
    }
    setLlmDownloading(false);
    setLlmDownloadProgress(null);
  }, []);

  const deleteLocalModel = useCallback(async () => {
    try {
      await deleteLlmModel();
      setLlmFileStatus({ exists: false, size_mb: 0 });
      setLlmLoaded(false);
    } catch (e) {
      console.error("Model delete failed:", e);
    }
  }, []);

  const loadLocalModel = useCallback(async () => {
    try {
      await reloadModels();
      const status = await checkModelsStatus();
      setLlmLoaded(status.llm_loaded);
    } catch (e) {
      console.error("Model load failed:", e);
    }
  }, []);

  // Whisper actions
  const downloadWhisperModelAction = useCallback(async () => {
    setWhisperDownloading(true);
    setWhisperDownloadProgress(null);
    const unlisten = await listen<DownloadProgress>("whisper_download_progress", (event) => {
      setWhisperDownloadProgress(event.payload);
    });
    try {
      await downloadWhisperModel();
      await reloadModels();
      const [fileResult, statusResult] = await Promise.allSettled([
        checkWhisperFileExists(),
        checkModelsStatus(),
      ]);
      if (fileResult.status === "fulfilled") setWhisperFileStatus(fileResult.value);
      if (statusResult.status === "fulfilled") setWhisperLoaded(statusResult.value.whisper_loaded);
    } catch (e) {
      console.error("Whisper download failed:", e);
    } finally {
      unlisten();
      setWhisperDownloading(false);
    }
  }, []);

  const cancelWhisperDownload = useCallback(async () => {
    try {
      await cancelModelDownload();
    } catch {
      // ignore
    }
    setWhisperDownloading(false);
    setWhisperDownloadProgress(null);
  }, []);

  const deleteWhisperModelAction = useCallback(async () => {
    try {
      await deleteWhisperModel(settingsRef.current.whisperModel);
      setWhisperFileStatus({ exists: false, size_mb: 0, model_name: settingsRef.current.whisperModel });
      setWhisperLoaded(false);
    } catch (e) {
      console.error("Whisper delete failed:", e);
    }
  }, []);

  const loadWhisperModel = useCallback(async () => {
    try {
      await reloadModels();
      const status = await checkModelsStatus();
      setWhisperLoaded(status.whisper_loaded);
    } catch (e) {
      console.error("Whisper load failed:", e);
    }
  }, []);

  return {
    ...settings,
    ollamaStatus,
    connectionResult,
    isLoading,
    isTesting,
    setLlmMode,
    setOllamaModel,
    setByokProvider,
    setByokApiKey,
    testConnection,
    refreshOllama,
    // Local LLM model
    llmFileStatus,
    llmLoaded,
    llmDownloading,
    llmDownloadProgress,
    downloadLocalModel,
    cancelLocalModelDownload,
    deleteLocalModel,
    loadLocalModel,
    // Whisper model
    whisperFileStatus,
    whisperLoaded,
    whisperDownloading,
    whisperDownloadProgress,
    downloadWhisperModelAction,
    cancelWhisperDownload,
    deleteWhisperModelAction,
    loadWhisperModel,
  };
}
