// Wizard state atom -- tracks first-launch setup completion
import { atom } from "jotai";

// Tri-state: null = initial DB check in progress, false = first launch (show wizard), true = setup done (show settings)
export const setupCompleteAtom = atom<boolean | null>(null);
