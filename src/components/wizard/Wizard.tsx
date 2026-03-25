// First-launch wizard -- 9-step onboarding flow for model setup
import { useCallback } from "react";
import { useSetAtom } from "jotai";
import { setupCompleteAtom } from "@/stores/wizardStore";
import { useWizard } from "@/hooks/useWizard";
import { WelcomeStep } from "./WelcomeStep";
import { ModelSelectionStep } from "./ModelSelectionStep";
import { DownloadStep } from "./DownloadStep";
import { DownloadCompleteStep } from "./DownloadCompleteStep";
import { OllamaStep } from "./OllamaStep";
import { ByokStep } from "./ByokStep";
import { WhisperWarningStep } from "./WhisperWarningStep";
import { HotkeyStep } from "./HotkeyStep";
import { ReadyStep } from "./ReadyStep";

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
            onGetStarted={wizard.goToModelSelection}
            onSkip={wizard.skipSetup}
          />
        );
      case "model-selection":
        return (
          <ModelSelectionStep
            onSelect={wizard.selectModelAndContinue}
            onBack={wizard.goToWelcome}
          />
        );
      case "downloading":
        return (
          <DownloadStep
            downloadProgress={wizard.downloadProgress}
            downloadError={wizard.downloadError}
            onProgress={wizard.handleDownloadProgress}
            onError={wizard.handleDownloadError}
            onStartDownload={wizard.startDownload}
            onCancel={wizard.cancelDownload}
          />
        );
      case "download-complete":
        return (
          <DownloadCompleteStep onContinue={wizard.goToHotkey} />
        );
      case "ollama":
        return (
          <OllamaStep
            ollamaStatus={wizard.ollamaStatus}
            ollamaModel={wizard.ollamaModel}
            ollamaLoading={wizard.ollamaLoading}
            onModelChange={wizard.setOllamaModel}
            onRefresh={wizard.refreshOllama}
            onContinue={wizard.goToHotkey}
            onBack={wizard.goBack}
          />
        );
      case "byok":
        return (
          <ByokStep
            provider={wizard.byokProvider}
            apiKey={wizard.byokApiKey}
            connectionResult={wizard.byokResult}
            isTesting={wizard.byokTesting}
            onProviderChange={wizard.setByokProvider}
            onApiKeyChange={wizard.setByokApiKey}
            onTestConnection={wizard.testConnection}
            onContinue={wizard.goToHotkey}
            onBack={wizard.goBack}
          />
        );
      case "whisper-warning":
        return (
          <WhisperWarningStep
            onContinue={wizard.goToHotkey}
            onBack={wizard.goBack}
          />
        );
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
