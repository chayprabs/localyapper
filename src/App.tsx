import { useEffect } from "react";
import { useAtom } from "jotai";
import { Overlay } from "@/components/overlay/Overlay";
import { SettingsLayout } from "@/components/settings/SettingsLayout";
import { TitleBar } from "@/components/settings/TitleBar";
import { Wizard } from "@/components/wizard/Wizard";
import { setupCompleteAtom } from "@/stores/wizardStore";
import { getSetting } from "@/lib/commands/settings";
import { reloadModels } from "@/lib/commands/models";

function MainWindow() {
  const [setupComplete, setSetupComplete] = useAtom(setupCompleteAtom);

  useEffect(() => {
    getSetting("setup_complete")
      .then((value) => setSetupComplete(value === "true"))
      .catch(() => setSetupComplete(false));
  }, [setSetupComplete]);

  // Load models in the background once setup is complete
  useEffect(() => {
    if (setupComplete) {
      reloadModels().catch(() => {});
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
          <Wizard />
        ) : (
          <SettingsLayout />
        )}
      </div>
    </div>
  );
}

export function App() {
  const isOverlay = window.location.pathname === "/overlay";

  if (isOverlay) {
    return <Overlay />;
  }

  return <MainWindow />;
}
