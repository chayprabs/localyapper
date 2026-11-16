// First-launch wizard -- one step at a time, with a back button and a
// step counter
import { useCallback } from "react";
import { useSetAtom } from "jotai";
import { setupCompleteAtom } from "@/stores/wizardStore";
import { useWizard } from "@/hooks/useWizard";
import { AccessibilityStep } from "./AccessibilityStep";
import { HotkeyStep } from "./HotkeyStep";
import { MicrophoneStep } from "./MicrophoneStep";
import { ReadyStep } from "./ReadyStep";
import { SpeechFilesStep } from "./SpeechFilesStep";
import { TryStep } from "./TryStep";
import { WelcomeStep } from "./WelcomeStep";
import { IconButton } from "@/components/ui/Button";
import { Icon } from "@/components/ui/Icon";
import { Loading } from "@/components/shell/Pages";

export function Wizard() {
  const setSetupComplete = useSetAtom(setupCompleteAtom);
  const onComplete = useCallback(() => {
    setSetupComplete(true);
  }, [setSetupComplete]);

  const wizard = useWizard(onComplete);

  if (!wizard.hydrated) return <Loading />;

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
            refresh={wizard.refreshPermissions}
            openSettings={wizard.requestOpenMicSettings}
            onContinue={wizard.goNext}
          />
        );
      case "accessibility":
        return (
          <AccessibilityStep
            autoAdvance={!wizard.cameBack}
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
            cancelling={wizard.cancelling}
            modelSizeMb={wizard.modelSizeMb}
            downloadProgress={wizard.downloadProgress}
            downloadError={wizard.downloadError}
            onProgress={wizard.handleDownloadProgress}
            onStartDownload={wizard.startDownload}
            onCancel={wizard.cancelDownload}
            onContinue={wizard.goNext}
          />
        );
      case "try":
        return (
          <TryStep
            hotkey={wizard.hotkey}
            filesInstalled={wizard.filesInstalled}
            onContinue={wizard.goNext}
          />
        );
      case "done":
        return (
          <ReadyStep
            hotkey={wizard.hotkey}
            permissions={wizard.permissions}
            filesInstalled={wizard.filesInstalled}
            onFinish={(launchAtLogin) => void wizard.finishWizard(launchAtLogin)}
            error={wizard.setupError}
          />
        );
    }
  }

  return (
    <div className="flex h-full flex-col overflow-y-auto px-6">
      <main className="m-auto w-full max-w-[380px] py-8">
        <div className="mb-8 flex h-7 items-center gap-2 font-mono text-[11.5px] text-ink/40">
          {wizard.canGoBack && (
            <IconButton label="Back" onClick={wizard.goBack} className="-ml-2 animate-fade-in">
              <Icon name="arrow_back" size={14} />
            </IconButton>
          )}
          <span aria-label={`Step ${wizard.stepIndex + 1} of ${wizard.stepCount}`}>
            {wizard.stepIndex + 1}/{wizard.stepCount}
          </span>
        </div>
        {/* Keyed by step so each one fades in. */}
        <div key={wizard.step}>{renderStep()}</div>
      </main>
    </div>
  );
}
