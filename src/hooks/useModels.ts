// Models hook -- download, load, and delete lifecycle for the speech model only
import { useCallback, useEffect, useState } from "react";
import { useAtom } from "jotai";
import { listen } from "@tauri-apps/api/event";
import type {
  DownloadProgress,
  SpeechModelFileStatus,
} from "@/types/commands";
import { getAllSettings } from "@/lib/commands/settings";
import {
  cancelModelDownload,
  checkModelsStatus,
  checkSpeechModelFileExists,
  deleteSpeechModel,
  downloadSpeechModel,
  reloadModels,
} from "@/lib/commands/models";
import {
  modelStatusCacheAtom,
  modelsSettingsCacheAtom,
} from "@/stores/appStore";

const DEFAULT_SPEECH_MODEL = "parakeet-110m";

export function useModels() {
  const [settingsCache, setSettingsCache] = useAtom(modelsSettingsCacheAtom);
  const [statusCache, setStatusCache] = useAtom(modelStatusCacheAtom);

  const hasCached = settingsCache !== null && statusCache !== null;

  const [speechModel, setSpeechModel] = useState(
    settingsCache?.speechModel ?? DEFAULT_SPEECH_MODEL,
  );
  const [speechModelFileStatus, setSpeechModelFileStatus] =
    useState<SpeechModelFileStatus>(
      statusCache?.speechModelFileStatus ?? {
        exists: false,
        size_mb: 0,
        model_name: DEFAULT_SPEECH_MODEL,
      },
    );
  const [speechModelLoaded, setSpeechModelLoaded] = useState(
    statusCache?.speechModelLoaded ?? false,
  );
  const [speechModelDownloading, setSpeechModelDownloading] = useState(false);
  const [speechModelDownloadProgress, setSpeechModelDownloadProgress] =
    useState<DownloadProgress | null>(null);
  const [speechModelLoading, setSpeechModelLoading] = useState(false);
  const [speechModelError, setSpeechModelError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(!hasCached);

  const updateCaches = useCallback(
    (model: string, fileStatus: SpeechModelFileStatus, loaded: boolean) => {
      setSettingsCache({ speechModel: model });
      setStatusCache({
        speechModelFileStatus: fileStatus,
        speechModelLoaded: loaded,
      });
    },
    [setSettingsCache, setStatusCache],
  );

  useEffect(() => {
    async function load() {
      let nextModel = DEFAULT_SPEECH_MODEL;

      try {
        const settings = await getAllSettings();
        nextModel =
          settings["speech_model"] ?? DEFAULT_SPEECH_MODEL;
        setSpeechModel(nextModel);
      } catch (error) {
        console.error("Failed to load model settings:", error);
      }

      const [statusResult, fileResult] = await Promise.allSettled([
        checkModelsStatus(),
        checkSpeechModelFileExists(),
      ]);

      let nextLoaded = false;
      if (statusResult.status === "fulfilled") {
        nextLoaded = statusResult.value.speech_model_loaded;
        setSpeechModelLoaded(nextLoaded);
      }

      let nextFileStatus: SpeechModelFileStatus = {
        exists: false,
        size_mb: 0,
        model_name: nextModel,
      };
      if (fileResult.status === "fulfilled") {
        nextFileStatus = fileResult.value;
        setSpeechModelFileStatus(nextFileStatus);
      }

      updateCaches(nextModel, nextFileStatus, nextLoaded);
      setIsLoading(false);
    }

    void load();
  }, [updateCaches]);

  const updateStatusCache = useCallback(
    (fileStatus: SpeechModelFileStatus, loaded: boolean) => {
      setStatusCache({
        speechModelFileStatus: fileStatus,
        speechModelLoaded: loaded,
      });
    },
    [setStatusCache],
  );

  const downloadSpeechModelAction = useCallback(async () => {
    setSpeechModelDownloading(true);
    setSpeechModelDownloadProgress(null);
    setSpeechModelError(null);

    const unlisten = await listen<DownloadProgress>(
      "speech_model_download_progress",
      (event) => {
        setSpeechModelDownloadProgress(event.payload);
      },
    );

    try {
      await downloadSpeechModel();
      await reloadModels();

      const [fileResult, statusResult] = await Promise.allSettled([
        checkSpeechModelFileExists(),
        checkModelsStatus(),
      ]);

      let nextFileStatus: SpeechModelFileStatus = {
        exists: true,
        size_mb: 0,
        model_name: speechModel,
      };
      if (fileResult.status === "fulfilled") {
        nextFileStatus = fileResult.value;
        setSpeechModelFileStatus(nextFileStatus);
      }

      let nextLoaded = false;
      if (statusResult.status === "fulfilled") {
        nextLoaded = statusResult.value.speech_model_loaded;
        setSpeechModelLoaded(nextLoaded);
      }

      updateCaches(speechModel, nextFileStatus, nextLoaded);

      if (!nextLoaded) {
        setSpeechModelError(
          "Download finished, but the speech engine did not start. Try clicking Load Engine.",
        );
      }
    } catch (error) {
      const message =
        error instanceof Error
          ? error.message
          : typeof error === "string"
            ? error
            : "Download failed";
      setSpeechModelError(message);
      console.error("Speech model download failed:", error);
    } finally {
      unlisten();
      setSpeechModelDownloading(false);
    }
  }, [speechModel, updateCaches]);

  const cancelSpeechModelDownload = useCallback(async () => {
    try {
      await cancelModelDownload();
    } catch {
      // Ignore cancellation errors from already-finished downloads.
    }
    setSpeechModelDownloading(false);
    setSpeechModelDownloadProgress(null);
  }, []);

  const deleteSpeechModelAction = useCallback(async () => {
    setSpeechModelError(null);
    try {
      await deleteSpeechModel();
      const nextFileStatus = {
        exists: false,
        size_mb: 0,
        model_name: speechModel,
      };
      setSpeechModelFileStatus(nextFileStatus);
      setSpeechModelLoaded(false);
      updateStatusCache(nextFileStatus, false);
    } catch (error) {
      console.error("Speech model delete failed:", error);
    }
  }, [speechModel, updateStatusCache]);

  const loadSpeechModel = useCallback(async () => {
    setSpeechModelLoading(true);
    setSpeechModelError(null);

    try {
      await reloadModels();
      const [status, fileStatus] = await Promise.all([
        checkModelsStatus(),
        checkSpeechModelFileExists(),
      ]);

      setSpeechModelLoaded(status.speech_model_loaded);
      setSpeechModelFileStatus(fileStatus);
      updateCaches(speechModel, fileStatus, status.speech_model_loaded);

      if (!status.speech_model_loaded) {
        setSpeechModelError(
          "The speech engine did not start. Try removing the local speech files and downloading them again.",
        );
      }
    } catch (error) {
      const message =
        error instanceof Error
          ? error.message
          : typeof error === "string"
            ? error
            : "Load failed";
      setSpeechModelError(message);
      console.error("Speech model load failed:", error);
    } finally {
      setSpeechModelLoading(false);
    }
  }, [speechModel, updateCaches]);

  return {
    speechModel,
    speechModelFileStatus,
    speechModelLoaded,
    speechModelLoading,
    speechModelError,
    speechModelDownloading,
    speechModelDownloadProgress,
    downloadSpeechModelAction,
    cancelSpeechModelDownload,
    deleteSpeechModelAction,
    loadSpeechModel,
    isLoading,
  };
}
