// Global application state atoms -- page navigation and paused state
import { atom } from "jotai";

/** Main pages, in navigation order. */
export type PageId = "history" | "dictation" | "voice" | "general";

/** The page shown in the main window. */
export const activePageAtom = atom<PageId>("history");

/**
 * Tri-state atom mirroring the backend `state.paused` flag.
 * - null: initial state, command not yet returned
 * - false: dictation is active (default)
 * - true: dictation paused via the tray menu — global hotkeys are unregistered
 */
export const pausedAtom = atom<boolean | null>(null);
