// Wizard accessibility step (macOS) -- prompt for the grant and advance
// automatically once System Settings reports it.
import { useEffect, useState } from "react";
import { Icon } from "@/components/ui/Icon";
import {
  checkPermissions,
  openAccessibilitySettings,
  requestAccessibilityPermission,
} from "@/lib/commands/system";

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
    <div className="flex flex-col items-center text-center">
      <div className="w-14 h-14 bg-[#0058bc]/[0.08] rounded-full flex items-center justify-center mb-5">
        <Icon name="keyboard" size={28} className="text-[#0058bc]" />
      </div>
      <h2 className="text-[20px] font-semibold text-black/85 mb-1">
        Allow typing into other apps
      </h2>
      <p className="text-[13px] text-black/50 mb-6 max-w-[340px]">
        LocalYapper pastes your words into the app you are using. macOS
        needs Accessibility access for that. Turn on LocalYapper in System
        Settings, Privacy &amp; Security, Accessibility.
      </p>
      <p role="status" className="text-[12px] text-black/50 mb-4">
        {granted ? "Access granted." : "Waiting for access..."}
      </p>
      {!granted && slow && (
        <p className="mb-4 w-full rounded-lg border border-[#9a5a00]/20 bg-[#9a5a00]/[0.06] px-3 py-2 text-left text-[12px] text-[#7a4500]">
          Already switched on in the list? After an update macOS can keep an
          old entry that no longer counts. Select LocalYapper there, remove
          it with the minus button, then add it again.
        </p>
      )}
      {granted ? (
        <button
          onClick={onContinue}
          className="w-full h-9 bg-gradient-to-b from-[#0062d0] to-[#0058bc] text-white text-[13px] font-medium rounded-[8px] hover:brightness-110 active:brightness-95 transition-all"
        >
          Continue
        </button>
      ) : (
        <>
          <button
            onClick={() => {
              void openAccessibilitySettings();
            }}
            className="w-full h-9 bg-gradient-to-b from-[#0062d0] to-[#0058bc] text-white text-[13px] font-medium rounded-[8px] hover:brightness-110 active:brightness-95 transition-all mb-2"
          >
            Open System Settings
          </button>
          <button
            onClick={onContinue}
            className="text-[12px] text-black/50 hover:underline"
          >
            Skip for now (text will be copied instead of pasted)
          </button>
        </>
      )}
    </div>
  );
}
