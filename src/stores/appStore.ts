import { atom } from "jotai";

export type PageId = "dashboard" | "history" | "dictionary" | "hotkeys" | "models";

export const activePageAtom = atom<PageId>("dashboard");
