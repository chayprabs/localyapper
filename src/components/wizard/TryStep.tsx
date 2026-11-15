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
      title="Try it"
      text={
        <>
          Hold <Keys hotkey={hotkey} /> and speak.
        </>
      }
    >
      <textarea
        ref={boxRef}
        value={text}
        onChange={(e) => setText(e.target.value)}
        rows={4}
        aria-label="Try dictation here"
        className="selectable w-full resize-none rounded-md border border-ink/20 bg-transparent px-3 py-2 text-[13px] leading-5 text-ink/90 outline-none transition-colors focus:border-ink/70"
      />
      {failedOnce && (
        <Notice tone="note">
          Nothing arrived. Click in the box and try again.
          {isMacPlatform && " LocalYapper also needs Accessibility access to paste."}
        </Notice>
      )}
      <Button variant={worked ? "primary" : "outline"} size="lg" onClick={onContinue}>
        {worked ? "Continue" : "Skip"}
      </Button>
    </Step>
  );
}
