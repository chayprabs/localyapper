// Applies the `theme` setting to <html>: "light" or "dark" force a theme,
// "system" follows the OS through prefers-color-scheme (see index.css).
import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { getSetting } from "@/lib/commands/settings";

export function applyTheme(theme: string): void {
  const root = document.documentElement;
  if (theme === "light" || theme === "dark") {
    root.dataset.theme = theme;
  } else {
    delete root.dataset.theme;
  }
}

export function useTheme(): void {
  useEffect(() => {
    getSetting("theme")
      .then(applyTheme)
      .catch(() => applyTheme("system"));
    const unlisten = listen<string>("theme-changed", (e) => applyTheme(e.payload));
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, []);
}
