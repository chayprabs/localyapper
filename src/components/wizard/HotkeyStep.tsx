import { useEffect, useRef, useState, useCallback } from "react";

const isMac =
  typeof navigator !== "undefined" && /mac/i.test(navigator.userAgent);

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

function eventToShortcut(e: KeyboardEvent): string | null {
  if (["Alt", "Control", "Shift", "Meta", "AltGraph"].includes(e.key)) {
    return null;
  }
  const parts: string[] = [];
  if (e.ctrlKey) parts.push("Ctrl");
  if (e.altKey) parts.push("Alt");
  if (e.shiftKey) parts.push("Shift");
  if (e.metaKey) parts.push("Meta");

  let keyName = e.key;
  if (keyName === " ") keyName = "Space";
  else if (keyName.length === 1) keyName = keyName.toUpperCase();

  parts.push(keyName);
  return parts.join("+");
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
  const containerRef = useRef<HTMLDivElement>(null);

  const handleCapture = useCallback(
    (shortcut: string) => {
      onHotkeyChange(shortcut);
      setIsCapturing(false);
    },
    [onHotkeyChange],
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
        <span className="material-symbols-outlined text-[28px] text-[#0058bc]">
          keyboard
        </span>
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
          className={`inline-flex items-center gap-2 px-5 h-12 bg-white border rounded-[10px] shadow-sm cursor-pointer transition-all ${
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

      <button
        onClick={onContinue}
        className="w-full h-9 bg-gradient-to-b from-[#0062d0] to-[#0058bc] text-white text-[13px] font-medium rounded-[8px] hover:brightness-110 active:brightness-95 transition-all"
      >
        Continue
      </button>
    </div>
  );
}
