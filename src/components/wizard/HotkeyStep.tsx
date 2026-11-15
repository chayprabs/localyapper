// Wizard hotkey configuration -- set primary recording shortcut
import { useEffect, useRef, useState, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { Icon } from "@/components/ui/Icon";
import { setHotkeyProbe, updateHotkey } from "@/lib/commands/hotkeys";
import { isMacPlatform as isMac } from "@/lib/hotkeyDefaults";
import { eventToShortcut } from "@/lib/hotkeyCapture";

function formatKey(key: string): string {
  if (isMac) {
    switch (key.toLowerCase()) {
      case "alt":
        return "\u2325";
      case "shift":
        return "\u21E7";
      case "meta":
      case "cmd":
        return "\u2318";
      case "ctrl":
      case "control":
        return "\u2303";
      default:
        return key;
    }
  }
  return key;
}

function parseHotkeyParts(hotkey: string): string[] {
  return hotkey.split("+").map(formatKey);
}

export function HotkeyStep({
  hotkey,
  onHotkeyChange,
  onContinue,
}: {
  hotkey: string;
  onHotkeyChange: (hotkey: string) => void;
  onContinue: () => void;
}) {
  const [isCapturing, setIsCapturing] = useState(false);
  const [detected, setDetected] = useState(false);
  const [registerError, setRegisterError] = useState<string | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const capturingRef = useRef(false);
  capturingRef.current = isCapturing;

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
      setIsCapturing(false);
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

  const handleCancel = useCallback(() => {
    setIsCapturing(false);
  }, []);

  useEffect(() => {
    if (!isCapturing) return;

    function handleKeyDown(e: KeyboardEvent) {
      e.preventDefault();
      e.stopPropagation();

      if (e.key === "Escape") {
        handleCancel();
        return;
      }

      const shortcut = eventToShortcut(e);
      if (shortcut) {
        handleCapture(shortcut);
      }
    }

    function handleClickOutside(e: MouseEvent) {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        handleCancel();
      }
    }

    window.addEventListener("keydown", handleKeyDown, true);
    document.addEventListener("mousedown", handleClickOutside);

    return () => {
      window.removeEventListener("keydown", handleKeyDown, true);
      document.removeEventListener("mousedown", handleClickOutside);
    };
  }, [isCapturing, handleCapture, handleCancel]);

  const parts = parseHotkeyParts(hotkey);

  return (
    <div className="flex flex-col items-center text-center">
      <div className="w-14 h-14 bg-[#0058bc]/[0.08] rounded-full flex items-center justify-center mb-5">
        <Icon name="keyboard" size={28} className="text-[#0058bc]" />
      </div>

      <h2 className="text-[20px] font-semibold text-black/85 mb-1">
        Set your hotkey
      </h2>
      <p className="text-[13px] text-black/50 mb-6">
        Hold this key to record, release to inject text.
      </p>

      {/* Key display */}
      <div ref={containerRef} className="mb-6">
        <div
          onClick={() => setIsCapturing(true)}
          className={`inline-flex items-center gap-2 px-5 h-12 bg-surface border rounded-[10px] shadow-sm cursor-pointer transition-all ${
            isCapturing
              ? "border-[#0058bc] ring-2 ring-[#0058bc]/20"
              : "border-black/[0.10] hover:border-black/20"
          }`}
        >
          {isCapturing ? (
            <span className="text-[13px] text-black/30">Press keys...</span>
          ) : (
            <div className="flex items-center gap-1.5">
              {parts.map((part, i) => (
                <span
                  key={i}
                  className="px-2.5 h-7 flex items-center bg-black/[0.06] rounded-[6px] text-[13px] font-medium font-mono"
                >
                  {part}
                </span>
              ))}
            </div>
          )}
        </div>
        <button
          onClick={() => setIsCapturing(true)}
          className="block mx-auto mt-2 text-[12px] text-[#0058bc] hover:underline"
        >
          Change Hotkey
        </button>
      </div>

      {registerError && (
        <p
          role="alert"
          className="mb-3 w-full rounded-lg border border-[#ba1a1a]/15 bg-[#ba1a1a]/[0.06] px-3 py-2 text-[12px] font-medium text-[#ba1a1a]"
        >
          {registerError}. Choose a different key.
        </p>
      )}
      <p
        role="status"
        className={`text-[12px] mb-4 ${detected ? "text-[#1a7f37]" : "text-black/50"}`}
      >
        {detected
          ? "Hotkey detected. It works from any app."
          : "Press your hotkey now to test it."}
      </p>

      <button
        onClick={onContinue}
        disabled={!detected}
        className="w-full h-9 bg-gradient-to-b from-[#0062d0] to-[#0058bc] text-white text-[13px] font-medium rounded-[8px] hover:brightness-110 active:brightness-95 transition-all disabled:opacity-40 disabled:cursor-not-allowed"
      >
        Continue
      </button>
      {/* Never a dead end: some desktops (Wayland without the shortcuts
          portal) cannot deliver a global key at all. */}
      {!detected && (
        <button
          onClick={onContinue}
          className="mt-3 text-[12px] text-black/[0.45] hover:text-black/65"
        >
          Continue without testing
        </button>
      )}
    </div>
  );
}
