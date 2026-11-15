// Wizard ready step -- summary of choices made before completing setup
import { useState, type ReactNode } from "react";
import type { PermissionsStatus } from "@/types/commands";
import { Button } from "@/components/ui/Button";
import { Keys } from "@/components/ui/Keys";
import { Switch } from "@/components/ui/Switch";
import { Notice } from "@/components/settings/controls";
import { Step } from "./Step";

function SummaryRow({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="flex h-10 items-center justify-between gap-4">
      <span className="text-[13px] text-ink/90">{label}</span>
      <span className="text-[12.5px] text-ink/55">{children}</span>
    </div>
  );
}

interface ReadyStepProps {
  hotkey: string;
  permissions: PermissionsStatus | null;
  filesInstalled: boolean;
  onFinish: (launchAtLogin: boolean) => void;
  error: string | null;
}

export function ReadyStep({ hotkey, permissions, filesInstalled, onFinish, error }: ReadyStepProps) {
  const [launchAtLogin, setLaunchAtLogin] = useState(false);
  const micGranted = permissions?.microphone === true;

  return (
    <Step title="Ready">
      <div className="w-full divide-y divide-ink/[0.08] border-y border-ink/15">
        <SummaryRow label="Hotkey">
          <Keys hotkey={hotkey} />
        </SummaryRow>
        <SummaryRow label="Microphone">
          {micGranted ? "Ready" : <span className="text-signal">Not found</span>}
        </SummaryRow>
        <SummaryRow label="Speech model">
          {filesInstalled ? "Installed" : <span className="text-signal">Missing, get it under Voice</span>}
        </SummaryRow>
        <SummaryRow label="Launch at login">
          <Switch ariaLabel="Launch at login" checked={launchAtLogin} onChange={setLaunchAtLogin} />
        </SummaryRow>
      </div>
      <Button variant="primary" size="lg" onClick={() => onFinish(launchAtLogin)}>
        Done
      </Button>
      {error && <Notice>{error}</Notice>}
    </Step>
  );
}
