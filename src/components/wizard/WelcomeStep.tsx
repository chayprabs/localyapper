// Wizard welcome step -- product introduction with Start action
import { Button } from "@/components/ui/Button";
import { Notice } from "@/components/settings/controls";
import { Actions, Step } from "./Step";

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
    <Step title="LocalYapper" text="Hold a key, speak, let go. The text is pasted where your cursor is. Offline.">
      <Actions>
        <Button variant="primary" size="lg" onClick={onGetStarted}>
          Set up
        </Button>
        <Button variant="ghost" size="lg" onClick={onSkip}>
          Skip
        </Button>
      </Actions>
      {error && <Notice>{error}</Notice>}
    </Step>
  );
}
