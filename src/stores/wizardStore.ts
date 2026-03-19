import { atom } from "jotai";

// null = loading, false = show wizard, true = show settings
export const setupCompleteAtom = atom<boolean | null>(null);
