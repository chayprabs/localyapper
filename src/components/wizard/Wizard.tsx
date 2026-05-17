// First-launch wizard -- onboarding flow with chrome, indicator, and back nav
import { useCallback } from "react";
import { useSetAtom } from "jotai";
import { setupCompleteAtom } from "@/stores/wizardStore";
import { useWizard } from "@/hooks/useWizard";
import { HotkeyStep } from "./HotkeyStep";
import { MicrophoneStep } from "./MicrophoneStep";
import { ReadyStep } from "./ReadyStep";
import { SpeechFilesStep } from "./SpeechFilesStep";
import { WelcomeStep } from "./WelcomeStep";
import { WizardChrome } from "./WizardChrome";

export function Wizard() {
  const setSetupComplete = useSetAtom(setupCompleteAtom);
  const onComplete = useCallback(() => {
    setSetupComplete(true);
  }, [setSetupComplete]);

  const wizard = useWizard(onComplete);

  if (!wizard.hydrated) {
    return (
      <div className="flex h-full w-full items-center justify-center bg-[#F2F2F4]">
        <span className="material-symbols-outlined animate-spin text-[28px] text-black/[0.30]">
          progress_activity
        </span>
      </div>
    );
  }

  function renderStep() {
    switch (wizard.step) {
      case "welcome":
        return (
          <WelcomeStep
            onGetStarted={wizard.goNext}
            onSkip={wizard.skipSetup}
            error={wizard.setupError}
          />
        );
      case "microphone":
        return (
          <MicrophoneStep
            permissions={wizard.permissions}
            loading={wizard.permissionsLoading}
            refresh={() => {
              void wizard.refreshPermissions();
            }}
            openSettings={() => {
              void wizard.requestOpenMicSettings();
            }}
            onContinue={wizard.goNext}
          />
        );
      case "hotkey":
        return (
          <HotkeyStep
            hotkey={wizard.hotkey}
            onHotkeyChange={wizard.setHotkey}
            onContinue={wizard.goNext}
          />
        );
      case "files":
        return (
          <SpeechFilesStep
            filesInstalled={wizard.filesInstalled}
            downloading={wizard.downloading}
            downloadProgress={wizard.downloadProgress}
            downloadError={wizard.downloadError}
            onProgress={wizard.handleDownloadProgress}
            onStartDownload={wizard.startDownload}
            onCancel={wizard.cancelDownload}
            onContinue={wizard.goNext}
          />
        );
      case "done":
        return (
          <ReadyStep
            hotkey={wizard.hotkey}
            permissions={wizard.permissions}
            filesInstalled={wizard.filesInstalled}
            onFinish={wizard.finishWizard}
            error={wizard.setupError}
          />
        );
    }
  }

  return (
    <WizardChrome
      stepNumber={wizard.stepIndex + 1}
      totalSteps={wizard.stepCount}
      canGoBack={wizard.canGoBack}
      onBack={wizard.goBack}
    >
      {renderStep()}
    </WizardChrome>
  );
}
