// Global application state atoms -- page navigation and paused state
import { atom } from "jotai";

/** Main settings pages — order matches sidebar nav items. */
export type PageId = "dashboard" | "history" | "dictation" | "audio" | "speech" | "general";

/** Currently active page in the settings window sidebar. */
export const activePageAtom = atom<PageId>("dashboard");

/** Sidebar collapse state — persisted in settings table as "sidebar_collapsed". */
export const sidebarCollapsedAtom = atom<boolean>(false);

/**
 * Tri-state atom mirroring the backend `state.paused` flag.
 * - null: initial state, command not yet returned
 * - false: dictation is active (default)
 * - true: dictation paused via the tray menu — global hotkeys are unregistered
 */
export const pausedAtom = atom<boolean | null>(null);
