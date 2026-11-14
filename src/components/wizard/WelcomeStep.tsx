// Wizard welcome step -- product introduction with Get started action
import { Button } from "@/components/ui/Button";
import { Notice } from "@/components/settings/controls";
import { Step, TextButton } from "./Step";

export function WelcomeStep({
  onGetStarted,
  onSkip,
  error,
}: {
  onGetStarted: () => void;
  onSkip: () => void;
  error: string | null;
}) {
  return (
    <Step
      icon="mic"
      title="Welcome to LocalYapper"
      text="Press a key, speak, let go. Your words appear wherever your cursor is. Everything happens on this computer."
    >
      <Button variant="primary" size="lg" className="w-full" onClick={onGetStarted}>
        Get started
      </Button>
      <TextButton onClick={onSkip}>Skip setup</TextButton>
      {error && <Notice>{error}</Notice>}
    </Step>
  );
}
