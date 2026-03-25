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

export const overlayDataAtom = atom<OverlayData>(defaultOverlayData);

export const overlayVisualStateAtom = atom<OverlayVisualState>(
  (get) => get(overlayDataAtom).visualState,
);
