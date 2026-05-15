// First-launch wizard -- onboarding flow for speech model setup
import { useCallback } from "react";
import { useSetAtom } from "jotai";
import { setupCompleteAtom } from "@/stores/wizardStore";
import { useWizard } from "@/hooks/useWizard";
import { DownloadCompleteStep } from "./DownloadCompleteStep";
import { DownloadStep } from "./DownloadStep";
import { HotkeyStep } from "./HotkeyStep";
import { ReadyStep } from "./ReadyStep";
import { WelcomeStep } from "./WelcomeStep";

export function Wizard() {
  const setSetupComplete = useSetAtom(setupCompleteAtom);
  const onComplete = useCallback(() => {
    setSetupComplete(true);
  }, [setSetupComplete]);

  const wizard = useWizard(onComplete);

  function renderStep() {
    switch (wizard.step) {
      case "welcome":
        return (
          <WelcomeStep
            onGetStarted={wizard.goToDownload}
            onSkip={wizard.skipSetup}
            error={wizard.setupError}
          />
        );
      case "downloading":
        return (
          <DownloadStep
            downloadProgress={wizard.downloadProgress}
            downloadError={wizard.downloadError}
            onProgress={wizard.handleDownloadProgress}
            onStartDownload={wizard.startDownload}
            onCancel={wizard.cancelDownload}
          />
        );
      case "download-complete":
        return <DownloadCompleteStep onContinue={wizard.goToHotkey} />;
      case "hotkey":
        return (
          <HotkeyStep
            hotkey={wizard.hotkey}
            onHotkeyChange={wizard.setHotkey}
            onContinue={wizard.goToReady}
          />
        );
      case "ready":
        return (
          <ReadyStep
            hotkey={wizard.hotkey}
            onFinish={wizard.finishWizard}
            error={wizard.setupError}
          />
        );
    }
  }

  return (
    <div className="h-screen w-screen bg-[#E8E8E8] flex items-center justify-center">
      <div className="w-[480px] bg-white rounded-[12px] p-7 shadow-[0_8px_40px_rgba(0,0,0,0.15)]">
        {renderStep()}
      </div>
    </div>
  );
}
