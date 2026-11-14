// Root application component for the main window. The overlay has its own
// plain-TypeScript entry (`src/overlay-main.ts`) so the overlay WebView
// does not download or parse this module graph.
import { Suspense, lazy, useEffect } from "react";
import { useAtom } from "jotai";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Loading, Pages } from "@/components/shell/Pages";
import { TitleBar } from "@/components/shell/TitleBar";
import { setupCompleteAtom } from "@/stores/wizardStore";
import { getSetting } from "@/lib/commands/settings";
import { usePausedState } from "@/hooks/usePausedState";
import { useTheme } from "@/hooks/useTheme";

// The Wizard renders only on first launch. Lazy-importing it keeps the
// returning-user path off the wizard module graph entirely.
const Wizard = lazy(() =>
  import("@/components/wizard/Wizard").then((m) => ({ default: m.Wizard })),
);

export function App() {
  const [setupComplete, setSetupComplete] = useAtom(setupCompleteAtom);
  usePausedState();
  useTheme();

  useEffect(() => {
    getSetting("setup_complete")
      .then((value) => setSetupComplete(value === "true"))
      .catch(() => setSetupComplete(false));
  }, [setSetupComplete]);

  // Show window for wizard (first launch). Returning users stay in tray.
  useEffect(() => {
    if (setupComplete === false) {
      getCurrentWindow().show().catch(() => {});
    }
  }, [setupComplete]);

  return (
    <div className="flex h-full w-full flex-col">
      <TitleBar showNav={setupComplete === true} />
      <div className="min-h-0 flex-1">
        {setupComplete === null ? (
          <Loading />
        ) : setupComplete ? (
          <Pages />
        ) : (
          <Suspense fallback={<Loading />}>
            <Wizard />
          </Suspense>
        )}
      </div>
    </div>
  );
}
