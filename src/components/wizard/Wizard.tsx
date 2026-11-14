// First-launch wizard -- one step at a time in a glass card, with a back
// button and progress dots
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
import { cn } from "@/lib/utils";

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
            refresh={() => {
              void wizard.refreshPermissions();
            }}
            openSettings={() => {
              void wizard.requestOpenMicSettings();
            }}
            onContinue={wizard.goNext}
          />
        );
      case "accessibility":
        return <AccessibilityStep onContinue={wizard.goNext} />;
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
      case "try":
        return <TryStep hotkey={wizard.hotkey} onContinue={wizard.goNext} />;
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
    <div className="flex h-full flex-col items-center overflow-y-auto px-6 pb-6">
      <main className="glass relative my-auto w-full max-w-[440px] rounded-[28px] px-8 pb-8 pt-7">
        <div className="mb-6 flex h-8 items-center justify-center">
          {wizard.canGoBack && (
            <IconButton label="Back" onClick={wizard.goBack} className="absolute left-5 animate-fade-in">
              <Icon name="arrow_back" size={16} />
            </IconButton>
          )}
          <div
            role="progressbar"
            aria-label="Setup progress"
            aria-valuemin={1}
            aria-valuemax={wizard.stepCount}
            aria-valuenow={wizard.stepIndex + 1}
            className="flex items-center gap-1.5"
          >
            {Array.from({ length: wizard.stepCount }, (_, i) => (
              <span
                key={i}
                className={cn(
                  "h-1.5 rounded-full transition-[width,background-color] duration-300 ease-out",
                  i === wizard.stepIndex ? "w-5 bg-ink/70" : i < wizard.stepIndex ? "w-1.5 bg-ink/35" : "w-1.5 bg-ink/[0.12]",
                )}
              />
            ))}
          </div>
        </div>
        {/* Keyed by step so each one animates in. */}
        <div key={wizard.step}>{renderStep()}</div>
      </main>
      <p className="mt-4 shrink-0 text-[11.5px] text-ink/35">Everything happens on this computer. No audio leaves it.</p>
    </div>
  );
}
