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
import { Step, TextButton } from "./Step";

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
      icon="keyboard"
      title="Allow typing into other apps"
      text="LocalYapper pastes your words into the app you are using. macOS needs Accessibility access for that: turn on LocalYapper in System Settings, Privacy & Security, Accessibility."
    >
      <p role="status" className="flex items-center gap-2 text-[12.5px] text-ink/50">
        {!granted && <Spinner size={14} />}
        {granted ? "Access granted." : "Waiting for access"}
      </p>
      <Button variant="primary" size="lg" className="w-full" onClick={() => void openAccessibilitySettings()}>
        Open System Settings
      </Button>
      <TextButton onClick={onContinue}>Skip for now (text will be copied instead of pasted)</TextButton>
    </Step>
  );
}
