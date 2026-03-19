import { useState, useEffect, useCallback, useRef } from "react";
import type { OllamaStatus, ConnectionResult } from "@/types/commands";
import { getAllSettings, setSetting } from "@/lib/commands/settings";
import { checkOllama, testByokConnection } from "@/lib/commands/models";

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
  whisperModel: "tiny.en",
  llmMode: "local",
  ollamaModel: "qwen2.5:0.5b",
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
      const [settingsResult, ollamaResult] = await Promise.allSettled([
        getAllSettings(),
        checkOllama(),
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

  const setWhisperModel = useCallback(
    (model: WhisperModel) =>
      updateSetting("whisperModel", model, "whisper_model"),
    [updateSetting],
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

  return {
    ...settings,
    ollamaStatus,
    connectionResult,
    isLoading,
    isTesting,
    setWhisperModel,
    setLlmMode,
    setOllamaModel,
    setByokProvider,
    setByokApiKey,
    testConnection,
    refreshOllama,
  };
}
