// Wizard accessibility step (macOS) -- prompt for the grant and advance
// automatically once System Settings reports it.
import { useEffect, useState } from "react";
import { Button } from "@/components/ui/Button";
import { Notice, Spinner } from "@/components/settings/controls";
import {
  checkPermissions,
  openAccessibilitySettings,
  requestAccessibilityPermission,
} from "@/lib/commands/system";
import { Actions, Step } from "./Step";

export function AccessibilityStep({
  autoAdvance,
  onContinue,
}: {
  /** Move on as soon as access is granted (not when the user came Back). */
  autoAdvance: boolean;
  onContinue: () => void;
}) {
  const [granted, setGranted] = useState(false);
  // After a while without the grant, explain the usual reason.
  const [slow, setSlow] = useState(false);
  useEffect(() => {
    const timer = window.setTimeout(() => setSlow(true), 12000);
    return () => window.clearTimeout(timer);
  }, []);

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
    if (granted && autoAdvance) onContinue();
  }, [granted, autoAdvance, onContinue]);

  return (
    <Step
      title="Accessibility"
      text="macOS needs this to paste into other apps. Turn on LocalYapper in System Settings, Privacy & Security, Accessibility."
    >
      <p role="status" className="flex h-5 items-center gap-2.5 text-[13px] text-ink/90">
        {granted ? <span className="h-1.5 w-1.5 rounded-full bg-ink" /> : <Spinner size={13} />}
        {granted ? "Granted" : "Waiting"}
      </p>
      {!granted && slow && (
        <Notice tone="note">
          Already on in the list? After an update macOS can keep an old entry that no longer counts. Remove
          LocalYapper with the minus button, then add it again.
        </Notice>
      )}
      {granted ? (
        <Button variant="primary" size="lg" onClick={onContinue}>
          Continue
        </Button>
      ) : (
        <Actions>
          <Button variant="primary" size="lg" onClick={() => void openAccessibilitySettings()}>
            Open System Settings
          </Button>
          <Button variant="ghost" size="lg" title="Text will be copied instead of pasted." onClick={onContinue}>
            Skip
          </Button>
        </Actions>
      )}
    </Step>
  );
}
