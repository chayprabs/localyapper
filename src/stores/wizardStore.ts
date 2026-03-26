// Wizard state atom -- tracks first-launch setup completion
import { atom } from "jotai";

/**
 * Tri-state atom controlling app entry point:
 * - null: initial DB check in progress (show loading)
 * - false: first launch, setup_complete="false" in DB (show wizard)
 * - true: setup already done (show main settings window)
 */
export const setupCompleteAtom = atom<boolean | null>(null);
