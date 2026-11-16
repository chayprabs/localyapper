// Wizard microphone step -- shows whether a microphone can be used. The
// wizard hook asks for access and keeps the status up to date.
import type { PermissionsStatus } from "@/types/commands";
import { Button } from "@/components/ui/Button";
import { Spinner } from "@/components/settings/controls";
import { cn } from "@/lib/utils";
import { Actions, Step } from "./Step";

interface MicrophoneStepProps {
  permissions: PermissionsStatus | null;
  loading: boolean;
  refresh: () => Promise<void>;
  openSettings: () => Promise<void>;
  onContinue: () => void;
}

export function MicrophoneStep({ permissions, loading, refresh, openSettings, onContinue }: MicrophoneStepProps) {
  // Only the first check shows "Checking"; the hook re-reads the status
  // every two seconds and later answers update in place.
  const checking = permissions == null;
  const granted = permissions?.microphone === true;
  const denied = permissions != null && permissions.microphone === false;

  return (
    <Step title="Microphone" text="Used only while you hold the hotkey.">
      <p role="status" className="flex h-5 items-center gap-2.5 text-[13px] text-ink/90">
        {checking ? (
          <Spinner size={13} />
        ) : (
          <span className={cn("h-1.5 w-1.5 rounded-full", granted ? "bg-ink" : "bg-signal")} />
        )}
        {checking ? "Checking" : granted ? "Ready" : "No microphone available"}
      </p>
      <Actions>
        <Button variant="primary" size="lg" onClick={onContinue}>
          {denied ? "Continue anyway" : "Continue"}
        </Button>
        {denied && (
          <Button size="lg" onClick={() => void openSettings()}>
            Open settings
          </Button>
        )}
        <Button variant="ghost" size="lg" disabled={loading} onClick={() => void refresh()}>
          Check again
        </Button>
      </Actions>
    </Step>
  );
}
