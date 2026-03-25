// Hotkeys page -- remappable keyboard shortcuts with live key capture
import { useEffect, useRef, useCallback, useState } from "react";
import { useHotkeys } from "@/hooks/useHotkeys";

// Platform detection for key symbol display
const isMac =
  typeof navigator !== "undefined" && /mac/i.test(navigator.userAgent);

/** Format a modifier key for display based on platform. */
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

/** Parse "Alt+Shift+V" into displayable badge tokens. */
function parseHotkeyParts(hotkey: string): string[] {
  return hotkey.split("+").map(formatKey);
}

/** Convert a keyboard event into a Tauri-style shortcut string. */
function eventToShortcut(e: KeyboardEvent): string | null {
  // Ignore bare modifier presses
  if (
    ["Alt", "Control", "Shift", "Meta", "AltGraph"].includes(e.key)
  ) {
    return null;
  }

  const parts: string[] = [];
  if (e.ctrlKey) parts.push("Ctrl");
  if (e.altKey) parts.push("Alt");
  if (e.shiftKey) parts.push("Shift");
  if (e.metaKey) parts.push("Meta");

  // Normalize the key name
  let keyName = e.key;
  if (keyName === " ") keyName = "Space";
  else if (keyName.length === 1) keyName = keyName.toUpperCase();

  parts.push(keyName);
  return parts.join("+");
}

function KeyBadge({ label }: { label: string }) {
  return (
    <span className="px-2 h-6 flex items-center bg-[rgba(0,0,0,0.06)] rounded-[6px] text-[12px] font-medium font-mono text-[#1C1C1E]">
      {label}
    </span>
  );
}

function KeySelector({
  value,
  isEditing,
  readOnly,
  isDoubleTap,
  onStartEdit,
  onCapture,
  onCancel,
}: {
  value: string;
  isEditing: boolean;
  readOnly: boolean;
  isDoubleTap?: boolean;
  onStartEdit: () => void;
  onCapture: (shortcut: string) => void;
  onCancel: () => void;
}) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [pendingKeys, setPendingKeys] = useState<string | null>(null);

  useEffect(() => {
    if (!isEditing) {
      setPendingKeys(null);
      return;
    }

    function handleKeyDown(e: KeyboardEvent) {
      e.preventDefault();
      e.stopPropagation();

      // Escape cancels capture
      if (e.key === "Escape") {
        onCancel();
        return;
      }

      const shortcut = eventToShortcut(e);
      if (shortcut) {
        setPendingKeys(shortcut);
        onCapture(shortcut);
      }
    }

    function handleClickOutside(e: MouseEvent) {
      if (
        containerRef.current &&
        !containerRef.current.contains(e.target as Node)
      ) {
        onCancel();
      }
    }

    window.addEventListener("keydown", handleKeyDown, true);
    document.addEventListener("mousedown", handleClickOutside);

    return () => {
      window.removeEventListener("keydown", handleKeyDown, true);
      document.removeEventListener("mousedown", handleClickOutside);
    };
  }, [isEditing, onCancel, onCapture]);

  const displayValue = pendingKeys ?? value;
  const parts = parseHotkeyParts(displayValue);

  // For double-tap display: duplicate modifier keys (e.g., Ctrl → Ctrl Ctrl)
  const displayParts = isDoubleTap
    ? [...parts.slice(0, -1), ...parts.slice(0, -1), ...parts.slice(-1)]
    : parts;

  return (
    <div
      ref={containerRef}
      onClick={readOnly ? undefined : onStartEdit}
      className={`w-[200px] h-[36px] bg-white border rounded-[8px] shadow-sm px-2 flex items-center justify-between ${
        readOnly
          ? "cursor-default opacity-60"
          : "cursor-pointer hover:border-black/20"
      } ${isEditing ? "border-[#0058bc]" : "border-black/10"}`}
    >
      {isEditing ? (
        <span className="text-[12px] text-black/30 select-none">
          Press keys...
        </span>
      ) : (
        <div className="flex items-center gap-[6px]">
          {displayParts.map((part, i) => (
            <KeyBadge key={i} label={part} />
          ))}
        </div>
      )}
      {!readOnly && (
        <span className="material-symbols-outlined text-[12px] text-[rgba(0,0,0,0.30)] mr-1">
          expand_more
        </span>
      )}
    </div>
  );
}

export function HotkeysPage() {
  const {
    entries,
    isLoading,
    editingKey,
    updateHotkey,
    resetToDefaults,
    startEditing,
    stopEditing,
  } = useHotkeys();

  const [showResetConfirm, setShowResetConfirm] = useState(false);

  const handleCapture = useCallback(
    (key: string, shortcut: string) => {
      updateHotkey(key, shortcut);
    },
    [updateHotkey],
  );

  if (isLoading) {
    return (
      <div className="px-12 py-10">
        <h1 className="text-[26px] font-semibold text-black/85">Hotkeys</h1>
      </div>
    );
  }

  return (
    <div className="px-12 py-10">
      {/* Header */}
      <div className="flex items-center justify-between mb-8">
        <h1 className="text-[26px] font-semibold text-black/85">Hotkeys</h1>
        {showResetConfirm ? (
          <div className="flex items-center gap-3">
            <span className="text-[13px] text-black/50">Reset all hotkeys?</span>
            <button
              onClick={() => setShowResetConfirm(false)}
              className="bg-white border border-black/[0.15] px-3 h-8 rounded-lg text-[13px] font-medium shadow-sm hover:bg-black/[0.02] transition-colors"
            >
              Cancel
            </button>
            <button
              onClick={() => { resetToDefaults(); setShowResetConfirm(false); }}
              className="bg-[#ba1a1a] text-white px-3 h-8 rounded-lg text-[13px] font-medium shadow-sm hover:bg-[#a01616] transition-colors"
            >
              Reset
            </button>
          </div>
        ) : (
          <button
            onClick={() => setShowResetConfirm(true)}
            className="bg-white border border-black/[0.15] px-4 h-8 rounded-lg text-[13px] font-medium shadow-sm hover:bg-black/[0.02] active:bg-black/[0.04] transition-colors"
          >
            Reset to Defaults
          </button>
        )}
      </div>

      {/* Card */}
      <div className="bg-white rounded-[10px] border border-black/[0.07] shadow-sm overflow-hidden">
        {entries.map((entry, index) => (
          <div
            key={entry.key}
            className={`h-[52px] px-5 flex items-center justify-between ${
              index < entries.length - 1 ? "border-b border-black/[0.07]" : ""
            }`}
          >
            {/* Left: label + description */}
            <div className="flex flex-col">
              <span className="text-[13px] font-semibold text-black/85">
                {entry.label}
              </span>
              <span className="text-[12px] text-black/[0.40]">
                {entry.description}
              </span>
            </div>

            {/* Right: key selector */}
            <KeySelector
              value={entry.value}
              isEditing={editingKey === entry.key}
              readOnly={entry.readOnly}
              isDoubleTap={entry.key === "hotkey_hands_free"}
              onStartEdit={() => startEditing(entry.key)}
              onCapture={(shortcut) => handleCapture(entry.key, shortcut)}
              onCancel={stopEditing}
            />
          </div>
        ))}
      </div>
    </div>
  );
}
