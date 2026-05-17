// Paused-state hook -- mirrors backend AppState.paused into the Jotai atom
import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { useSetAtom } from "jotai";
import { pausedAtom } from "@/stores/appStore";
import { getPausedState } from "@/lib/commands/system";

/**
 * Subscribes to backend pause changes and seeds the initial value. Mount once
 * near the application root.
 */
export function usePausedState() {
  const setPaused = useSetAtom(pausedAtom);

  useEffect(() => {
    let cancelled = false;

    void getPausedState()
      .then((value) => {
        if (!cancelled) setPaused(value);
      })
      .catch((error) => {
        console.error("Failed to read paused state:", error);
        if (!cancelled) setPaused(false);
      });

    const unlisten = listen<boolean>("paused-state-changed", (event) => {
      setPaused(event.payload);
    });

    return () => {
      cancelled = true;
      unlisten.then((dispose) => dispose());
    };
  }, [setPaused]);
}
