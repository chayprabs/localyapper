// Wizard "Try it" step -- a real dictation into a text box in this window
import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { Icon } from "@/components/ui/Icon";

const isMac = typeof navigator !== "undefined" && /mac/i.test(navigator.userAgent);

export function TryStep({
  hotkey,
  filesInstalled,
  onContinue,
}: {
  hotkey: string;
  /** Whether the speech model is downloaded; without it nothing can arrive. */
  filesInstalled: boolean;
  onContinue: () => void;
}) {
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
    <div className="flex flex-col items-center text-center">
      <div className="mb-5 flex h-16 w-16 items-center justify-center rounded-full bg-[#0058bc]/[0.08]">
        <Icon name="mic" size={30} className="text-[#0058bc]" />
      </div>
      <h2 className="mb-2 text-[22px] font-semibold text-black/85">Try it</h2>
      <p className="mb-5 max-w-[380px] text-[14px] leading-relaxed text-black/50">
        Click in the box, hold <span className="font-mono text-black/70">{hotkey}</span>, say a
        sentence, and let go.
      </p>
      <textarea
        ref={boxRef}
        value={text}
        onChange={(e) => setText(e.target.value)}
        rows={4}
        aria-label="Try dictation here"
        placeholder="Your words appear here"
        className="mb-4 w-full resize-none rounded-[10px] border border-black/[0.12] bg-surface px-3 py-2 text-[14px] text-black/85 outline-none focus:border-[#0058bc]"
      />
      {!filesInstalled && (
        <p className="mb-4 w-full rounded-lg border border-[#9a5a00]/20 bg-[#9a5a00]/[0.06] px-3 py-2 text-left text-[12px] text-[#7a4500]">
          The speech model is not downloaded yet, so dictation cannot work.
          Go back one step to download it, or do it later from the Speech
          page.
        </p>
      )}
      {filesInstalled && failedOnce && (
        <p className="mb-4 w-full rounded-lg border border-[#9a5a00]/20 bg-[#9a5a00]/[0.06] px-3 py-2 text-left text-[12px] text-[#7a4500]">
          Nothing arrived. Make sure the box has the cursor and try again.
          {isMac && " On macOS, LocalYapper also needs Accessibility access to paste."}
          {" "}The overlay shows the reason if dictation failed.
        </p>
      )}
      <button
        type="button"
        onClick={onContinue}
        className="h-9 w-full rounded-[8px] bg-gradient-to-b from-[#0062d0] to-[#0058bc] text-[13px] font-medium text-white transition-all hover:brightness-110 active:brightness-95"
      >
        {worked ? "Continue" : "Skip for now"}
      </button>
    </div>
  );
}
