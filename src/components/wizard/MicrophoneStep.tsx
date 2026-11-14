// Wizard microphone step -- shows whether a microphone is available. The
// wizard hook asks the OS for access when this step opens.
import type { PermissionsStatus } from "@/types/commands";
import { Button } from "@/components/ui/Button";
import { Icon } from "@/components/ui/Icon";
import { Spinner } from "@/components/settings/controls";
import { cn } from "@/lib/utils";
import { Step, TextButton } from "./Step";

interface MicrophoneStepProps {
  permissions: PermissionsStatus | null;
  loading: boolean;
  refresh: () => void;
  openSettings: () => void;
  onContinue: () => void;
}

export function MicrophoneStep({ permissions, loading, refresh, openSettings, onContinue }: MicrophoneStepProps) {
  const granted = permissions?.microphone === true;
  const denied = permissions != null && permissions.microphone === false;

  return (
    <Step
      icon={denied ? "mic_off" : "mic"}
      title="Allow the microphone"
      text="LocalYapper listens only while you hold the hotkey. Audio stays in memory on this computer."
    >
      <div
        role="status"
        className={cn(
          "flex w-full items-center gap-3 rounded-2xl px-4 py-3 text-left transition-colors duration-300",
          granted ? "bg-ok/10" : denied ? "bg-danger/[0.08]" : "bg-ink/[0.05]",
        )}
      >
        {loading ? (
          <Spinner size={18} />
        ) : (
          <Icon
            name={granted ? "check_circle" : denied ? "error" : "mic"}
            size={18}
            className={granted ? "text-ok" : denied ? "text-danger" : "text-ink/40"}
          />
        )}
        <div className="min-w-0 flex-1">
          <p className="text-[13px] font-medium text-ink/90">
            {loading
              ? "Checking the microphone"
              : granted
                ? "Microphone ready"
                : denied
                  ? "No microphone available"
                  : "Microphone not checked yet"}
          </p>
          <p className="mt-0.5 text-[12px] leading-snug text-ink/50">
            {granted
              ? "Your default input device will be used."
              : "Allow LocalYapper in system settings, or plug in a microphone."}
          </p>
        </div>
        {denied && (
          <Button size="sm" onClick={openSettings}>
            Open settings
          </Button>
        )}
      </div>

      <Button variant="primary" size="lg" className="w-full" onClick={onContinue} disabled={loading}>
        {denied ? "Continue anyway" : "Continue"}
      </Button>
      <TextButton onClick={refresh}>Check again</TextButton>
    </Step>
  );
}
