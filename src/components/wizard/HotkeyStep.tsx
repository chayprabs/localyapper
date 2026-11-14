// Wizard hotkey configuration -- set primary recording shortcut
import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { setHotkeyProbe, updateHotkey } from "@/lib/commands/hotkeys";
import { useHotkeyCapture } from "@/hooks/useHotkeyCapture";
import { Button } from "@/components/ui/Button";
import { Icon } from "@/components/ui/Icon";
import { Keys } from "@/components/ui/Keys";
import { cn } from "@/lib/utils";
import { Step } from "./Step";

export function HotkeyStep({
  hotkey,
  onHotkeyChange,
  onContinue,
}: {
  hotkey: string;
  onHotkeyChange: (hotkey: string) => void;
  onContinue: () => void;
}) {
  const [capturing, setCapturing] = useState(false);
  const [detected, setDetected] = useState(false);
  const [registerError, setRegisterError] = useState<string | null>(null);
  const fieldRef = useRef<HTMLButtonElement>(null);

  // Register the hotkey now and listen for it globally. While the probe
  // is on, presses only report detection and start no dictation.
  useEffect(() => {
    let cancelled = false;
    setDetected(false);
    updateHotkey("hotkey_record", hotkey)
      .then(() => {
        if (!cancelled) setRegisterError(null);
      })
      .catch((error: unknown) => {
        if (!cancelled) setRegisterError(String(error));
      });
    return () => {
      cancelled = true;
    };
  }, [hotkey]);

  useEffect(() => {
    void setHotkeyProbe(true);
    const unlisten = listen("hotkey-detected", () => setDetected(true));
    return () => {
      void setHotkeyProbe(false);
      void unlisten.then((stop) => stop());
    };
  }, []);

  const handleCapture = useCallback(
    (shortcut: string) => {
      onHotkeyChange(shortcut);
      setCapturing(false);
    },
    [onHotkeyChange],
  );
  const handleCancel = useCallback(() => setCapturing(false), []);
  useHotkeyCapture(capturing, fieldRef, handleCapture, handleCancel);

  return (
    <Step icon="keyboard" title="Choose your hotkey" text="Hold it to record, let go to paste. Click the key to change it.">
      <button
        ref={fieldRef}
        type="button"
        aria-label={`Record hotkey: ${hotkey}. Click to change.`}
        onClick={() => setCapturing(true)}
        className={cn(
          "flex h-16 min-w-[168px] items-center justify-center rounded-3xl px-6 transition-[background-color,box-shadow] duration-200",
          capturing
            ? "bg-accent/[0.1] shadow-[0_0_0_1.5px_rgb(var(--accent)/0.6)]"
            : detected
              ? "bg-ok/10 shadow-[0_0_0_1.5px_rgb(var(--ok)/0.45)]"
              : "bg-ink/[0.05] hover:bg-ink/[0.08]",
        )}
      >
        {capturing ? (
          <span className="animate-fade-in text-[13px] font-medium text-accent">Press keys</span>
        ) : (
          <Keys hotkey={hotkey} size="lg" />
        )}
      </button>

      <p
        role="status"
        className={cn(
          "flex min-h-5 items-center gap-1.5 text-[12.5px]",
          registerError ? "text-danger" : detected ? "text-ok" : "text-ink/50",
        )}
      >
        {detected && !registerError && <Icon name="check" size={14} strokeWidth={2.25} />}
        {registerError
          ? registerError
          : detected
            ? "Detected. It works from any app."
            : "Press your hotkey now to test it."}
      </p>

      <Button variant="primary" size="lg" className="w-full" onClick={onContinue} disabled={!detected}>
        Continue
      </Button>
    </Step>
  );
}
