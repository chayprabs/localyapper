// Root application component for the main settings window. The overlay
// has its own entry point (`src/overlay-main.tsx`) so the overlay WebView
// does not download or parse the settings/wizard module graph.
import { Suspense, lazy, useEffect } from "react";
import { useAtom } from "jotai";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { SettingsLayout } from "@/components/settings/SettingsLayout";
import { TitleBar } from "@/components/settings/TitleBar";
import { setupCompleteAtom } from "@/stores/wizardStore";
import { getSetting } from "@/lib/commands/settings";
import { usePausedState } from "@/hooks/usePausedState";

// The Wizard renders only on first launch. Lazy-importing it keeps the
// returning-user path off the wizard module graph entirely.
const Wizard = lazy(() =>
  import("@/components/wizard/Wizard").then((m) => ({ default: m.Wizard })),
);

export function App() {
  const [setupComplete, setSetupComplete] = useAtom(setupCompleteAtom);
  usePausedState();

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
    <div className="flex flex-col h-screen w-screen">
      <TitleBar />
      <div className="flex-1 min-h-0">
        {setupComplete === null ? (
          <div className="h-full bg-[#f9f9f9] flex items-center justify-center">
            <span className="material-symbols-outlined text-[32px] text-black/[0.30] animate-spin">
              progress_activity
            </span>
          </div>
        ) : !setupComplete ? (
          <Suspense
            fallback={
              <div className="h-full bg-[#f9f9f9] flex items-center justify-center">
                <span className="material-symbols-outlined text-[32px] text-black/[0.30] animate-spin">
                  progress_activity
                </span>
              </div>
            }
          >
            <Wizard />
          </Suspense>
        ) : (
          <SettingsLayout />
        )}
      </div>
    </div>
  );
}
