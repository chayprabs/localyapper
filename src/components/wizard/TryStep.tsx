// Wizard "Try it" step -- a real dictation into a text box in this window
import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { isMacPlatform } from "@/lib/hotkeyDefaults";
import { Button } from "@/components/ui/Button";
import { Keys } from "@/components/ui/Keys";
import { Notice } from "@/components/settings/controls";
import { Step } from "./Step";

export function TryStep({ hotkey, onContinue }: { hotkey: string; onContinue: () => void }) {
  const [text, setText] = useState("");
  const [attempts, setAttempts] = useState(0);
  const boxRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    boxRef.current?.focus();
    // Each finished dictation reports its timings; if no text arrived
    // shortly after, the hint below explains what to check.
    const unlisten = listen("dictation-timings", () => {
      window.setTimeout(() => setAttempts((n) => n + 1), 400);
    });
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, []);

  const worked = text.trim().length > 0;
  const failedOnce = attempts > 0 && !worked;

  return (
    <Step
      icon="mic"
      title="Try it"
      text={
        <>
          Click in the box, hold <Keys hotkey={hotkey} />, say a sentence and let go.
        </>
      }
    >
      <textarea
        ref={boxRef}
        value={text}
        onChange={(e) => setText(e.target.value)}
        rows={4}
        aria-label="Try dictation here"
        placeholder="Your words appear here"
        className="selectable w-full resize-none rounded-2xl bg-ink/[0.05] px-4 py-3 text-left text-[14px] leading-relaxed text-ink/90 outline-none transition-[background-color,box-shadow] duration-200 placeholder:text-ink/35 focus:bg-ink/[0.03] focus:shadow-[0_0_0_1.5px_rgb(var(--accent)/0.6)]"
      />
      {failedOnce && (
        <Notice tone="warn">
          Nothing arrived. Make sure the box has the cursor and try again.
          {isMacPlatform && " On macOS, LocalYapper also needs Accessibility access to paste."} The overlay shows the
          reason if dictation failed.
        </Notice>
      )}
      <Button variant={worked ? "primary" : "soft"} size="lg" className="w-full" onClick={onContinue}>
        {worked ? "Continue" : "Skip for now"}
      </Button>
    </Step>
  );
}
