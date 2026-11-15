// Wizard hotkey configuration -- set primary recording shortcut
import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { setHotkeyProbe, updateHotkey } from "@/lib/commands/hotkeys";
import { useHotkeyCapture } from "@/hooks/useHotkeyCapture";
import { Button } from "@/components/ui/Button";
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
    <Step title="Hotkey" text="Click the key to change it. Press it to test.">
      <button
        ref={fieldRef}
        type="button"
        aria-label={`Record hotkey: ${hotkey}. Click to change.`}
        onClick={() => setCapturing(true)}
        className="flex h-16 items-center rounded-lg"
      >
        {capturing ? (
          <span className="flex h-16 animate-fade-in items-center rounded-lg border border-ink px-5 font-mono text-[14px] text-ink">
            Press keys
          </span>
        ) : (
          <Keys hotkey={hotkey} size="xl" />
        )}
      </button>

      <p role="status" className={cn("flex h-5 items-center gap-2.5 text-[13px]", registerError ? "text-signal" : "text-ink/90")}>
        {!registerError && <span className={cn("h-1.5 w-1.5 rounded-full", detected ? "bg-ink" : "border border-ink/30")} />}
        {registerError ? registerError : detected ? "Detected" : "Not pressed yet"}
      </p>

      <Button variant="primary" size="lg" onClick={onContinue} disabled={!detected}>
        Continue
      </Button>
    </Step>
  );
}
