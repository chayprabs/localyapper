// Global application state atoms -- page navigation and model caches
import { atom } from "jotai";
import type { OllamaStatus, LlmFileStatus, WhisperFileStatus } from "@/types/commands";

export type PageId = "dashboard" | "history" | "dictionary" | "hotkeys" | "models";

export const activePageAtom = atom<PageId>("dashboard");

export const sidebarCollapsedAtom = atom<boolean>(false);

// Models page cache — survives page switches so the Models tab re-renders instantly without refetching
export interface ModelsSettingsCache {
  whisperModel: string;
  llmMode: string;
  ollamaModel: string;
  byokProvider: string;
  byokApiKey: string;
}

export interface ModelStatusCache {
  llmFileStatus: LlmFileStatus;
  whisperFileStatus: WhisperFileStatus;
  llmLoaded: boolean;
  whisperLoaded: boolean;
}

export const modelsSettingsCacheAtom = atom<ModelsSettingsCache | null>(null);
export const ollamaStatusCacheAtom = atom<OllamaStatus | null>(null);
export const modelStatusCacheAtom = atom<ModelStatusCache | null>(null);
