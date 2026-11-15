// Wizard accessibility step (macOS) -- prompt for the grant and advance
// automatically once System Settings reports it.
import { useEffect, useState } from "react";
import { Button } from "@/components/ui/Button";
import { Spinner } from "@/components/settings/controls";
import {
  checkPermissions,
  openAccessibilitySettings,
  requestAccessibilityPermission,
} from "@/lib/commands/system";
import { Actions, Step } from "./Step";

export function AccessibilityStep({ onContinue }: { onContinue: () => void }) {
  const [granted, setGranted] = useState(false);

  useEffect(() => {
    let cancelled = false;
    void requestAccessibilityPermission().then((ok) => {
      if (!cancelled && ok) setGranted(true);
    });
    // Poll once a second so the step advances as soon as access is given.
    const timer = window.setInterval(() => {
      void checkPermissions().then((p) => {
        if (!cancelled && p.accessibility) setGranted(true);
      });
    }, 1000);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, []);

  useEffect(() => {
    if (granted) onContinue();
  }, [granted, onContinue]);

  return (
    <Step
      title="Accessibility"
      text="macOS needs this to paste into other apps. Turn on LocalYapper in System Settings, Privacy & Security, Accessibility."
    >
      <p role="status" className="flex h-5 items-center gap-2.5 text-[13px] text-ink/90">
        {!granted && <Spinner size={13} />}
        {granted ? "Granted" : "Waiting"}
      </p>
      <Actions>
        <Button variant="primary" size="lg" onClick={() => void openAccessibilitySettings()}>
          Open System Settings
        </Button>
        <Button variant="ghost" size="lg" title="Text will be copied instead of pasted." onClick={onContinue}>
          Skip
        </Button>
      </Actions>
    </Step>
  );
}
