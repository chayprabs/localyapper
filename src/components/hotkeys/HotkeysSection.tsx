// Hotkeys section of the Dictation page -- remappable shortcuts with live key capture
import { useEffect, useRef, useCallback, useState } from "react";
import { useHotkeys } from "@/hooks/useHotkeys";
import { reservedHotkeyMessage } from "@/lib/hotkeyReservations";
import { eventToShortcut } from "@/lib/hotkeyCapture";
import { Icon } from "@/components/ui/Icon";

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

/** Parse a shortcut string into displayable badge tokens. */
function parseHotkeyParts(hotkey: string): string[] {
  return hotkey.split("+").map(formatKey);
}

function KeyBadge({ label }: { label: string }) {
  return (
    <span className="inline-flex h-7 max-w-full items-center rounded-[7px] border border-black/[0.06] bg-black/[0.04] px-2.5 text-[12px] font-medium font-mono text-black/90 shadow-[inset_0_1px_0_rgba(255,255,255,0.65)]">
      {label}
    </span>
  );
}

function KeySelector({
  value,
  isEditing,
  readOnly,
  onStartEdit,
  onCapture,
  onCancel,
}: {
  value: string;
  isEditing: boolean;
  readOnly: boolean;
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

  return (
    <div
      ref={containerRef}
      onClick={readOnly ? undefined : onStartEdit}
      className={`flex w-full max-w-[320px] min-h-[46px] items-center rounded-[10px] border bg-surface px-3 py-2 shadow-sm transition-colors ${
        readOnly
          ? "cursor-default border-black/[0.08] bg-black/[0.015]"
          : "cursor-pointer hover:border-black/20"
      } ${isEditing ? "border-[#0058bc] ring-2 ring-[#0058bc]/10" : "border-black/10"}`}
    >
      {isEditing ? (
        <div className="flex min-w-0 flex-1 items-center justify-between gap-2">
          <span className="text-[12px] text-black/30 select-none">
            Press shortcut...
          </span>
          <span className="text-[11px] text-black/[0.26] select-none">
            Esc to cancel
          </span>
        </div>
      ) : (
        <div className="flex min-w-0 flex-1 flex-wrap items-center gap-1.5">
          {parts.map((part, i) => (
            <KeyBadge key={i} label={part} />
          ))}
        </div>
      )}
      {!readOnly && (
        <Icon name="expand_more" size={14} className="ml-3 shrink-0 text-black/[0.30]" />
      )}
    </div>
  );
}

export function HotkeysSection() {
  const {
    entries,
    isLoading,
    editingKey,
    error,
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
      <section className="mb-8">
        <h2 className="text-[12px] font-semibold uppercase tracking-wide text-black/40">Hotkeys</h2>
      </section>
    );
  }

  return (
    <section className="mb-8">
      <div className="mb-2 flex flex-wrap items-end justify-between gap-4">
        <div>
          <h2 className="text-[12px] font-semibold uppercase tracking-wide text-black/40">Hotkeys</h2>
          {error && (
            <p className="mt-3 max-w-[520px] rounded-lg border border-[#ba1a1a]/15 bg-[#ba1a1a]/[0.06] px-3 py-2 text-[12px] font-medium text-[#ba1a1a]">
              {error}
            </p>
          )}
        </div>

        {showResetConfirm ? (
          <div className="flex flex-wrap items-center justify-end gap-2">
            <span className="text-[13px] text-black/50">
              Reset all hotkeys?
            </span>
            <button
              onClick={() => setShowResetConfirm(false)}
              className="h-8 rounded-lg border border-black/[0.15] bg-surface px-3 text-[13px] font-medium shadow-sm transition-colors hover:bg-black/[0.02]"
            >
              Cancel
            </button>
            <button
              onClick={() => {
                resetToDefaults();
                setShowResetConfirm(false);
              }}
              className="h-8 rounded-lg bg-[#ba1a1a] px-3 text-[13px] font-medium text-white shadow-sm transition-colors hover:bg-[#a01616]"
            >
              Reset
            </button>
          </div>
        ) : (
          <button
            onClick={() => setShowResetConfirm(true)}
            className="h-8 rounded-lg border border-black/[0.15] bg-surface px-4 text-[13px] font-medium shadow-sm transition-colors hover:bg-black/[0.02] active:bg-black/[0.04]"
          >
            Reset to Defaults
          </button>
        )}
      </div>

      <div className="overflow-hidden rounded-[10px] border border-black/[0.07] bg-surface shadow-sm">
        {entries.map((entry, index) => {
          const reservedMessage = reservedHotkeyMessage(entry.value);
          return (
            <div
              key={entry.key}
              className={`grid gap-4 px-5 py-4 md:grid-cols-[minmax(0,1fr)_minmax(240px,320px)] md:items-center ${
                index < entries.length - 1 ? "border-b border-black/[0.07]" : ""
              }`}
            >
              <div className="min-w-0">
                <span className="text-[13px] font-semibold text-black/85">
                  {entry.label}
                </span>
                <span className="mt-1 block text-[12px] text-black/[0.40]">
                  {entry.description}
                </span>
              </div>

              <div className="w-full md:justify-self-end">
                <KeySelector
                  value={entry.value}
                  isEditing={editingKey === entry.key}
                  readOnly={entry.readOnly}
                  onStartEdit={() => startEditing(entry.key)}
                  onCapture={(shortcut) => handleCapture(entry.key, shortcut)}
                  onCancel={stopEditing}
                />
                {reservedMessage && (
                  <p className="mt-1.5 inline-flex items-center gap-1 text-[11px] font-medium text-[#9a5a00]">
                    <Icon name="warning" size={14} />
                    {reservedMessage}
                  </p>
                )}
              </div>
            </div>
          );
        })}
      </div>
    </section>
  );
}
