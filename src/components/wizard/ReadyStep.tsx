// Wizard ready step -- summary of choices made before completing setup
import { useState, type ReactNode } from "react";
import type { PermissionsStatus } from "@/types/commands";
import { Button } from "@/components/ui/Button";
import { Icon } from "@/components/ui/Icon";
import { Keys } from "@/components/ui/Keys";
import { Switch } from "@/components/ui/Switch";
import { Notice } from "@/components/settings/controls";
import { Step } from "./Step";

function SummaryRow({ ok, label, children }: { ok: boolean; label: string; children: ReactNode }) {
  return (
    <div className="flex min-h-11 items-center gap-3 px-4 py-2">
      <Icon
        name={ok ? "check" : "warning"}
        size={15}
        strokeWidth={2.25}
        className={ok ? "shrink-0 text-ok" : "shrink-0 text-warn"}
      />
      <span className="flex-1 text-[13px] font-medium text-ink/90">{label}</span>
      <span className="text-right text-[12.5px] text-ink/50">{children}</span>
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
    <Step
      icon="check_circle"
      tone="ok"
      title="You're ready to dictate"
      text="Hold your hotkey in any app and speak. The words appear the moment you let go."
    >
      <div className="w-full divide-y divide-ink/[0.06] rounded-2xl bg-ink/[0.04] text-left">
        <SummaryRow ok label="Hotkey">
          <Keys hotkey={hotkey} />
        </SummaryRow>
        <SummaryRow ok={micGranted} label="Microphone">
          {micGranted ? "Ready" : "Not found yet"}
        </SummaryRow>
        <SummaryRow ok={filesInstalled} label="Speech model">
          {filesInstalled ? "Installed" : "Get it later under Voice"}
        </SummaryRow>
        <label className="flex min-h-11 cursor-pointer items-center gap-3 px-4 py-2">
          <span className="flex-1 pl-[27px] text-[13px] font-medium text-ink/90">Launch when I sign in</span>
          <Switch ariaLabel="Launch LocalYapper when I sign in" checked={launchAtLogin} onChange={setLaunchAtLogin} />
        </label>
      </div>

      <Button variant="primary" size="lg" className="w-full" onClick={() => onFinish(launchAtLogin)}>
        Start yapping
      </Button>
      {error && <Notice>{error}</Notice>}
    </Step>
  );
}
