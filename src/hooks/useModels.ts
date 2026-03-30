// Models hook -- download, load, delete lifecycle for Whisper and LLM
import { useState, useEffect, useCallback, useRef } from "react";
import { useAtom } from "jotai";
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
import {
  modelsSettingsCacheAtom,
  ollamaStatusCacheAtom,
  modelStatusCacheAtom,
} from "@/stores/appStore";
import type { ModelsSettingsCache, ModelStatusCache } from "@/stores/appStore";

type LlmMode = "local" | "ollama" | "byok";
type WhisperModel = "parakeet-110m" | "parakeet-0.6b" | "tiny.en" | "base.en" | "small.en" | "medium.en";
type ByokProvider = "openai" | "anthropic" | "groq";

interface ModelsState {
  whisperModel: WhisperModel;
  llmMode: LlmMode;
  ollamaModel: string;
  byokProvider: ByokProvider;
  byokApiKey: string;
}

/** Fallback values when settings table has no entry for a key. */
const DEFAULTS: ModelsState = {
  whisperModel: "parakeet-110m",
  llmMode: "local",
  ollamaModel: "qwen2.5:1.5b",
  byokProvider: "openai",
  byokApiKey: "",
};

function settingsFromCache(cache: ModelsSettingsCache): ModelsState {
  return {
    whisperModel: (cache.whisperModel as WhisperModel) ?? DEFAULTS.whisperModel,
    llmMode: (cache.llmMode as LlmMode) ?? DEFAULTS.llmMode,
    ollamaModel: cache.ollamaModel ?? DEFAULTS.ollamaModel,
    byokProvider: (cache.byokProvider as ByokProvider) ?? DEFAULTS.byokProvider,
    byokApiKey: cache.byokApiKey ?? DEFAULTS.byokApiKey,
  };
}

export function useModels() {
  const [settingsCache, setSettingsCache] = useAtom(modelsSettingsCacheAtom);
  const [ollamaStatusCache, setOllamaStatusCache] = useAtom(ollamaStatusCacheAtom);
  const [statusCache, setStatusCache] = useAtom(modelStatusCacheAtom);

  // Initialize from cache if available — skip loading state entirely on revisit
  const hasCached = settingsCache !== null && statusCache !== null;

  const [settings, setSettings] = useState<ModelsState>(
    settingsCache ? settingsFromCache(settingsCache) : DEFAULTS
  );
  const [ollamaStatus, setOllamaStatus] = useState<OllamaStatus | null>(ollamaStatusCache);
  const [connectionResult, setConnectionResult] =
    useState<ConnectionResult | null>(null);
  const [isLoading, setIsLoading] = useState(!hasCached);
  const [isTesting, setIsTesting] = useState(false);
  const settingsRef = useRef(settings);
  settingsRef.current = settings;

  // Local LLM state
  const [llmFileStatus, setLlmFileStatus] = useState<LlmFileStatus>(
    statusCache?.llmFileStatus ?? { exists: false, size_mb: 0 }
  );
  const [llmLoaded, setLlmLoaded] = useState(statusCache?.llmLoaded ?? false);
  const [llmDownloading, setLlmDownloading] = useState(false);
  const [llmDownloadProgress, setLlmDownloadProgress] = useState<DownloadProgress | null>(null);
  const [llmLoading, setLlmLoading] = useState(false);
  const [llmError, setLlmError] = useState<string | null>(null);

  // Whisper state
  const [whisperFileStatus, setWhisperFileStatus] = useState<WhisperFileStatus>(
    statusCache?.whisperFileStatus ?? { exists: false, size_mb: 0, model_name: "parakeet-110m" }
  );
  const [whisperLoaded, setWhisperLoaded] = useState(statusCache?.whisperLoaded ?? false);
  const [whisperDownloading, setWhisperDownloading] = useState(false);
  const [whisperDownloadProgress, setWhisperDownloadProgress] = useState<DownloadProgress | null>(null);
  const [whisperLoading, setWhisperLoading] = useState(false);
  const [whisperError, setWhisperError] = useState<string | null>(null);

  // Track whether Ollama has been checked this mount
  const ollamaCheckedRef = useRef(false);

  const refreshOllama = useCallback(async () => {
    try {
      const status = await checkOllama();
      setOllamaStatus(status);
      setOllamaStatusCache(status);
      ollamaCheckedRef.current = true;
    } catch (e) {
      console.error("Failed to check Ollama:", e);
    }
  }, [setOllamaStatusCache]);

  // Helper to update Jotai caches
  const updateCaches = useCallback((
    newSettings: ModelsState,
    newLlmFile: LlmFileStatus,
    newWhisperFile: WhisperFileStatus,
    newLlmLoaded: boolean,
    newWhisperLoaded: boolean,
  ) => {
    setSettingsCache({
      whisperModel: newSettings.whisperModel,
      llmMode: newSettings.llmMode,
      ollamaModel: newSettings.ollamaModel,
      byokProvider: newSettings.byokProvider,
      byokApiKey: newSettings.byokApiKey,
    });
    setStatusCache({
      llmFileStatus: newLlmFile,
      whisperFileStatus: newWhisperFile,
      llmLoaded: newLlmLoaded,
      whisperLoaded: newWhisperLoaded,
    });
  }, [setSettingsCache, setStatusCache]);

  useEffect(() => {
    async function load() {
      // Fetch everything EXCEPT Ollama status
      const [settingsResult, modelsResult, llmFileResult, whisperFileResult] = await Promise.allSettled([
        getAllSettings(),
        checkModelsStatus(),
        checkLlmFileExists(),
        checkWhisperFileExists(),
      ]);

      let newSettings = settings;
      if (settingsResult.status === "fulfilled") {
        const s = settingsResult.value;
        newSettings = {
          whisperModel:
            (s["whisper_model"] as WhisperModel) ?? DEFAULTS.whisperModel,
          llmMode: (s["llm_mode"] as LlmMode) ?? DEFAULTS.llmMode,
          ollamaModel: s["ollama_model"] ?? DEFAULTS.ollamaModel,
          byokProvider:
            (s["byok_provider"] as ByokProvider) ?? DEFAULTS.byokProvider,
          byokApiKey: s["byok_api_key"] ?? DEFAULTS.byokApiKey,
        };
        setSettings(newSettings);
      }

      let newLlmLoaded = false;
      let newWhisperLoaded = false;
      if (modelsResult.status === "fulfilled") {
        newLlmLoaded = modelsResult.value.llm_loaded;
        newWhisperLoaded = modelsResult.value.whisper_loaded;
        setLlmLoaded(newLlmLoaded);
        setWhisperLoaded(newWhisperLoaded);
      }

      let newLlmFile: LlmFileStatus = { exists: false, size_mb: 0 };
      if (llmFileResult.status === "fulfilled") {
        newLlmFile = llmFileResult.value;
        setLlmFileStatus(newLlmFile);
      }

      let newWhisperFile: WhisperFileStatus = { exists: false, size_mb: 0, model_name: "parakeet-110m" };
      if (whisperFileResult.status === "fulfilled") {
        newWhisperFile = whisperFileResult.value;
        setWhisperFileStatus(newWhisperFile);
      }

      // Write to Jotai cache for instant render on revisit
      updateCaches(newSettings, newLlmFile, newWhisperFile, newLlmLoaded, newWhisperLoaded);

      setIsLoading(false);

      // If current mode is ollama and we haven't checked yet, check in background
      if (newSettings.llmMode === "ollama" && !ollamaCheckedRef.current) {
        void refreshOllama();
      }
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
      setSettings((prev) => {
        const next = { ...prev, [key]: value };
        // Update settings cache inline
        setSettingsCache({
          whisperModel: next.whisperModel,
          llmMode: next.llmMode,
          ollamaModel: next.ollamaModel,
          byokProvider: next.byokProvider,
          byokApiKey: next.byokApiKey,
        });
        return next;
      });
      try {
        await setSetting(settingKey, String(value));
      } catch (e) {
        console.error(`Failed to update ${settingKey}:`, e);
        setSettings((prev) => {
          const reverted = { ...prev, [key]: previous };
          setSettingsCache({
            whisperModel: reverted.whisperModel,
            llmMode: reverted.llmMode,
            ollamaModel: reverted.ollamaModel,
            byokProvider: reverted.byokProvider,
            byokApiKey: reverted.byokApiKey,
          });
          return reverted;
        });
      }
    },
    [setSettingsCache],
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

  // Helper to sync model status cache
  const updateStatusCache = useCallback((updates: Partial<ModelStatusCache>) => {
    setStatusCache((prev) => {
      const base = prev ?? {
        llmFileStatus: { exists: false, size_mb: 0 },
        whisperFileStatus: { exists: false, size_mb: 0, model_name: "parakeet-110m" },
        llmLoaded: false,
        whisperLoaded: false,
      };
      return { ...base, ...updates };
    });
  }, [setStatusCache]);

  const downloadLocalModel = useCallback(async () => {
    setLlmDownloading(true);
    setLlmDownloadProgress(null);
    setLlmError(null);
    const unlisten = await listen<DownloadProgress>("model_download_progress", (event) => {
      setLlmDownloadProgress(event.payload);
    });
    try {
      await downloadModel();
      await reloadModels();
      const newFile = { exists: true, size_mb: 1024 };
      setLlmFileStatus(newFile);
      setLlmLoaded(true);
      updateStatusCache({ llmFileStatus: newFile, llmLoaded: true });
    } catch (e) {
      const msg = e instanceof Error ? e.message : typeof e === "string" ? e : "Download failed";
      setLlmError(msg);
      console.error("Model download failed:", e);
    } finally {
      unlisten();
      setLlmDownloading(false);
    }
  }, [updateStatusCache]);

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
    setLlmError(null);
    try {
      await deleteLlmModel();
      const newFile = { exists: false, size_mb: 0 };
      setLlmFileStatus(newFile);
      setLlmLoaded(false);
      updateStatusCache({ llmFileStatus: newFile, llmLoaded: false });
    } catch (e) {
      console.error("Model delete failed:", e);
    }
  }, [updateStatusCache]);

  const loadLocalModel = useCallback(async () => {
    setLlmLoading(true);
    setLlmError(null);
    try {
      await reloadModels();
      const status = await checkModelsStatus();
      setLlmLoaded(status.llm_loaded);
      updateStatusCache({ llmLoaded: status.llm_loaded });
      if (!status.llm_loaded) {
        setLlmError("Model file may be corrupted. Try deleting and re-downloading.");
      }
    } catch (e) {
      const msg = e instanceof Error ? e.message : typeof e === "string" ? e : "Load failed";
      setLlmError(msg);
      console.error("Model load failed:", e);
    } finally {
      setLlmLoading(false);
    }
  }, [updateStatusCache]);

  // Whisper actions
  const downloadWhisperModelAction = useCallback(async () => {
    setWhisperDownloading(true);
    setWhisperDownloadProgress(null);
    setWhisperError(null);
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
      if (fileResult.status === "fulfilled") {
        setWhisperFileStatus(fileResult.value);
        updateStatusCache({ whisperFileStatus: fileResult.value });
      }
      if (statusResult.status === "fulfilled") {
        setWhisperLoaded(statusResult.value.whisper_loaded);
        updateStatusCache({ whisperLoaded: statusResult.value.whisper_loaded });
        if (!statusResult.value.whisper_loaded) {
          setWhisperError("Download complete but model failed to load. Try clicking Load Model.");
        }
      }
    } catch (e) {
      const msg = e instanceof Error ? e.message : typeof e === "string" ? e : "Download failed";
      setWhisperError(msg);
      console.error("Whisper download failed:", e);
    } finally {
      unlisten();
      setWhisperDownloading(false);
    }
  }, [updateStatusCache]);

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
    setWhisperError(null);
    try {
      await deleteWhisperModel(settingsRef.current.whisperModel);
      const newFile = { exists: false, size_mb: 0, model_name: settingsRef.current.whisperModel };
      setWhisperFileStatus(newFile);
      setWhisperLoaded(false);
      updateStatusCache({ whisperFileStatus: newFile, whisperLoaded: false });
    } catch (e) {
      console.error("Whisper delete failed:", e);
    }
  }, [updateStatusCache]);

  const loadWhisperModel = useCallback(async () => {
    setWhisperLoading(true);
    setWhisperError(null);
    try {
      await reloadModels();
      const status = await checkModelsStatus();
      setWhisperLoaded(status.whisper_loaded);
      updateStatusCache({ whisperLoaded: status.whisper_loaded });
      if (!status.whisper_loaded) {
        setWhisperError("Model file may be corrupted. Try deleting and re-downloading.");
      }
    } catch (e) {
      const msg = e instanceof Error ? e.message : typeof e === "string" ? e : "Load failed";
      setWhisperError(msg);
      console.error("Whisper load failed:", e);
    } finally {
      setWhisperLoading(false);
    }
  }, [updateStatusCache]);

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
    llmLoading,
    llmError,
    llmDownloading,
    llmDownloadProgress,
    downloadLocalModel,
    cancelLocalModelDownload,
    deleteLocalModel,
    loadLocalModel,
    // Whisper model
    whisperFileStatus,
    whisperLoaded,
    whisperLoading,
    whisperError,
    whisperDownloading,
    whisperDownloadProgress,
    downloadWhisperModelAction,
    cancelWhisperDownload,
    deleteWhisperModelAction,
    loadWhisperModel,
  };
}
