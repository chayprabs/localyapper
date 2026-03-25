// Wizard ready step -- final confirmation before completing setup
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

export function ReadyStep({
  hotkey,
  onFinish,
}: {
  hotkey: string;
  onFinish: () => void;
}) {
  const parts = parseHotkeyParts(hotkey);

  return (
    <div className="flex flex-col items-center text-center">
      {/* Success check */}
      <div className="w-16 h-16 bg-[#006b19]/[0.08] rounded-full flex items-center justify-center mb-5 shadow-[0_0_24px_rgba(40,205,65,0.25)]">
        <span className="material-symbols-outlined text-[32px] text-[#006b19]">
          check_circle
        </span>
      </div>

      <h2 className="text-[22px] font-semibold text-black/85 mb-2">
        You're all set!
      </h2>
      <p className="text-[14px] text-black/50 mb-6 leading-relaxed">
        Hold your hotkey anywhere to start dictating.
      </p>

      {/* Hotkey badge */}
      <div className="inline-flex items-center gap-1.5 px-4 h-10 bg-black/[0.03] rounded-[10px] border border-black/[0.07] mb-8">
        {parts.map((part, i) => (
          <span
            key={i}
            className="px-2.5 h-7 flex items-center bg-white rounded-[6px] text-[13px] font-medium font-mono shadow-sm border border-black/[0.06]"
          >
            {part}
          </span>
        ))}
      </div>

      <button
        onClick={onFinish}
        className="w-full h-9 bg-gradient-to-b from-[#0062d0] to-[#0058bc] text-white text-[13px] font-medium rounded-[8px] hover:brightness-110 active:brightness-95 transition-all"
      >
        Start Yapping
      </button>
    </div>
  );
}
