// Wizard hotkey configuration -- set and test the record shortcut
import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { setHotkeyProbe, updateHotkey } from "@/lib/commands/hotkeys";
import { useHotkeyCapture } from "@/hooks/useHotkeyCapture";
import { Button } from "@/components/ui/Button";
import { Keys } from "@/components/ui/Keys";
import { Notice } from "@/components/settings/controls";
import { cn } from "@/lib/utils";
import { Actions, Step } from "./Step";

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
  const capturingRef = useRef(false);
  capturingRef.current = capturing;

  // Make sure the current hotkey is registered, so a key another app
  // already holds is reported here and not discovered later.
  const initialHotkey = useRef(hotkey);
  useEffect(() => {
    let cancelled = false;
    updateHotkey("hotkey_record", initialHotkey.current).catch((error: unknown) => {
      if (!cancelled) setRegisterError(String(error));
    });
    return () => {
      cancelled = true;
    };
  }, []);

  // While the probe is on, presses of the record key only report that
  // they were seen and start no dictation.
  useEffect(() => {
    void setHotkeyProbe(true);
    const unlisten = listen("hotkey-detected", () => {
      if (!capturingRef.current) setDetected(true);
    });
    return () => {
      void setHotkeyProbe(false);
      void unlisten.then((stop) => stop());
    };
  }, []);

  // A new combo replaces the old one only once it is registered. When it
  // cannot be (in use elsewhere, or already another LocalYapper hotkey)
  // the previous key stays and the reason is shown.
  const handleCapture = useCallback(
    (shortcut: string) => {
      setCapturing(false);
      if (shortcut === hotkey) return;
      setRegisterError(null);
      updateHotkey("hotkey_record", shortcut)
        .then(() => {
          setDetected(false);
          onHotkeyChange(shortcut);
        })
        .catch((error: unknown) => {
          setRegisterError(String(error));
        });
    },
    [hotkey, onHotkeyChange],
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
        className="flex min-h-16 max-w-full items-center rounded-lg text-left"
      >
        {capturing ? (
          <span className="flex h-16 animate-fade-in items-center rounded-lg border border-ink px-5 font-mono text-[14px] text-ink">
            Press keys
          </span>
        ) : (
          <Keys hotkey={hotkey} size="xl" />
        )}
      </button>

      {registerError && <Notice>{registerError}. Choose a different key.</Notice>}
      <p role="status" className="flex h-5 items-center gap-2.5 text-[13px] text-ink/90">
        <span className={cn("h-1.5 w-1.5 rounded-full", detected ? "bg-ink" : "border border-ink/30")} />
        {detected ? "Detected" : "Not pressed yet"}
      </p>

      <Actions>
        <Button variant="primary" size="lg" onClick={onContinue} disabled={!detected}>
          Continue
        </Button>
        {/* Never a dead end: some desktops (Wayland without the shortcuts
            portal) cannot deliver a global key at all. */}
        {!detected && (
          <Button variant="ghost" size="lg" onClick={onContinue}>
            Skip test
          </Button>
        )}
      </Actions>
    </Step>
  );
}
