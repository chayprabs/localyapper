// Global application state atoms -- page navigation and speech model cache
import { atom } from "jotai";
import type { SpeechModelFileStatus } from "@/types/commands";

/** Five-page navigation enum — order matches sidebar nav items. */
export type PageId = "dashboard" | "history" | "dictionary" | "hotkeys" | "models";

/** Currently active page in the settings window sidebar. */
export const activePageAtom = atom<PageId>("dashboard");

/** Sidebar collapse state — persisted in settings table as "sidebar_collapsed". */
export const sidebarCollapsedAtom = atom<boolean>(false);

export interface ModelsSettingsCache {
  speechModel: string;
}

export interface ModelStatusCache {
  speechModelFileStatus: SpeechModelFileStatus;
  speechModelLoaded: boolean;
}

export const modelsSettingsCacheAtom = atom<ModelsSettingsCache | null>(null);
export const modelStatusCacheAtom = atom<ModelStatusCache | null>(null);
