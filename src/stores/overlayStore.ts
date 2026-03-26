// Overlay state atoms -- visual state and transcription data
import { atom } from "jotai";
import type { OverlayData, OverlayVisualState } from "@/types/overlay";

// Initial overlay state — hidden until first pipeline event
const defaultOverlayData: OverlayData = {
  visualState: "hidden",
  text: null,
  durationMs: null,
  wordCount: null,
  error: null,
  recordingStartedAt: null,
};

/** Primary overlay state atom — updated by useOverlayState on each pipeline event. */
export const overlayDataAtom = atom<OverlayData>(defaultOverlayData);

/** Derived read-only atom for components that only need the visual state string. */
export const overlayVisualStateAtom = atom<OverlayVisualState>(
  (get) => get(overlayDataAtom).visualState,
);
