import { useEffect, useRef, useState, useCallback } from "react";
import { useAtom } from "jotai";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { overlayDataAtom } from "@/stores/overlayStore";
import type { OverlayData, OverlayVisualState, PipelineEvent } from "@/types/overlay";

const MAX_RECORDING_SECONDS = 120;
const WARNING_THRESHOLD_SECONDS = 105;
const LONG_RECORDING_THRESHOLD_MS = 30_000;
const AUTO_INJECT_DISPLAY_MS = 3000;

export function useOverlayState() {
  const [overlayData, setOverlayData] = useAtom(overlayDataAtom);
  const [elapsedSeconds, setElapsedSeconds] = useState(0);
  const [autoInjectProgress, setAutoInjectProgress] = useState(0);

  const generationRef = useRef(0);
  const elapsedIntervalRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const hideTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const injectTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const injectAnimRef = useRef<number | null>(null);
  const recordingStartRef = useRef<number | null>(null);

  const clearAllTimers = useCallback(() => {
    if (elapsedIntervalRef.current) {
      clearInterval(elapsedIntervalRef.current);
      elapsedIntervalRef.current = null;
    }
    if (hideTimeoutRef.current) {
      clearTimeout(hideTimeoutRef.current);
      hideTimeoutRef.current = null;
    }
    if (injectTimeoutRef.current) {
      clearTimeout(injectTimeoutRef.current);
      injectTimeoutRef.current = null;
    }
    if (injectAnimRef.current) {
      cancelAnimationFrame(injectAnimRef.current);
      injectAnimRef.current = null;
    }
  }, []);

  const showOverlay = useCallback(async () => {
    try {
      console.log("[overlay] showOverlay called");
      const win = getCurrentWindow();
      await win.show();
      await win.setFocus();
    } catch (e) {
      console.error("[overlay] Failed to show overlay window:", e);
    }
  }, []);

  const hideOverlay = useCallback(async () => {
    try {
      await getCurrentWindow().hide();
    } catch (e) {
      console.error("Failed to hide overlay window:", e);
    }
  }, []);

  const transition = useCallback(
    (state: OverlayVisualState, partial?: Partial<OverlayData>) => {
      setOverlayData((prev) => ({
        ...prev,
        visualState: state,
        ...partial,
      }));
    },
    [setOverlayData],
  );

  const startElapsedTimer = useCallback(
    (gen: number, initialElapsed = 0) => {
      const startTime = Date.now() - initialElapsed * 1000;
      recordingStartRef.current = startTime;

      elapsedIntervalRef.current = setInterval(() => {
        if (generationRef.current !== gen) return;
        const elapsed = (Date.now() - startTime) / 1000;
        setElapsedSeconds(elapsed);

        if (elapsed >= WARNING_THRESHOLD_SECONDS) {
          const remaining = MAX_RECORDING_SECONDS - elapsed;
          if (remaining <= 0) return;
          transition("stopping-soon");
        }
      }, 100);
    },
    [transition],
  );

  const startAutoInjectCountdown = useCallback(
    (gen: number) => {
      const startTime = Date.now();
      setAutoInjectProgress(0);

      const animate = () => {
        if (generationRef.current !== gen) return;
        const elapsed = Date.now() - startTime;
        const progress = Math.min(elapsed / AUTO_INJECT_DISPLAY_MS, 1);
        setAutoInjectProgress(progress);

        if (progress < 1) {
          injectAnimRef.current = requestAnimationFrame(animate);
        }
      };
      injectAnimRef.current = requestAnimationFrame(animate);

      injectTimeoutRef.current = setTimeout(() => {
        if (generationRef.current !== gen) return;
        transition("hidden");
        hideOverlay();
      }, AUTO_INJECT_DISPLAY_MS);
    },
    [transition, hideOverlay],
  );

  useEffect(() => {
    console.log("[overlay] Pipeline state listener attached");
    const unlisten = listen<PipelineEvent>("pipeline-state", (event) => {
      const { state, text, duration_ms, word_count, error } = event.payload;
      console.log("[overlay] Received pipeline-state:", state, { text, duration_ms, word_count, error });
      const gen = ++generationRef.current;
      clearAllTimers();

      switch (state) {
        case "listening": {
          setElapsedSeconds(0);
          setAutoInjectProgress(0);
          transition("listening", {
            text: null,
            durationMs: null,
            wordCount: null,
            error: null,
            recordingStartedAt: Date.now(),
          });
          startElapsedTimer(gen);
          showOverlay();
          break;
        }

        case "stopping-soon": {
          setAutoInjectProgress(0);
          transition("stopping-soon", {
            text: null,
            durationMs: null,
            wordCount: null,
            error: null,
            recordingStartedAt: Date.now() - 108 * 1000,
          });
          startElapsedTimer(gen, 108);
          showOverlay();
          break;
        }

        case "processing": {
          const recordingDuration = recordingStartRef.current
            ? Date.now() - recordingStartRef.current
            : 0;
          const isLong = recordingDuration > LONG_RECORDING_THRESHOLD_MS;
          transition(isLong ? "long-recording" : "processing", {
            durationMs: duration_ms,
          });
          break;
        }

        case "transcribed": {
          transition("transcribed", {
            text: text ?? null,
            durationMs: duration_ms,
            wordCount: word_count,
          });
          startAutoInjectCountdown(gen);
          break;
        }

        case "injected": {
          transition("hidden");
          hideTimeoutRef.current = setTimeout(() => {
            if (generationRef.current !== gen) return;
            hideOverlay();
          }, 500);
          break;
        }

        case "cancelled": {
          transition("hidden");
          hideTimeoutRef.current = setTimeout(() => {
            if (generationRef.current !== gen) return;
            hideOverlay();
          }, 300);
          break;
        }

        case "error": {
          transition("hidden", { error });
          hideTimeoutRef.current = setTimeout(() => {
            if (generationRef.current !== gen) return;
            hideOverlay();
          }, 1000);
          break;
        }
      }
    });

    return () => {
      unlisten.then((fn) => fn());
      clearAllTimers();
    };
  }, [
    clearAllTimers,
    transition,
    showOverlay,
    hideOverlay,
    startElapsedTimer,
    startAutoInjectCountdown,
  ]);

  const recordingElapsed = elapsedSeconds;
  const remainingSeconds = Math.max(0, MAX_RECORDING_SECONDS - elapsedSeconds);

  return {
    overlayData,
    elapsedSeconds,
    recordingElapsed,
    remainingSeconds,
    autoInjectProgress,
  };
}
