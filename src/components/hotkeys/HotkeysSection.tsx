// Hotkeys section of the Dictation page -- remappable shortcuts with live key capture
import { useCallback, useRef, useState } from "react";
import { useHotkeys } from "@/hooks/useHotkeys";
import { useHotkeyCapture } from "@/hooks/useHotkeyCapture";
import { reservedHotkeyMessage } from "@/lib/hotkeyReservations";
import { Button } from "@/components/ui/Button";
import { Icon } from "@/components/ui/Icon";
import { Keys } from "@/components/ui/Keys";
import { Notice, Row, Section } from "@/components/settings/controls";

/** The shortcut as key caps; click it and press a new combination. */
function KeyField({
  label,
  value,
  editing,
  onStartEdit,
  onCapture,
  onCancel,
}: {
  label: string;
  value: string;
  editing: boolean;
  onStartEdit: () => void;
  onCapture: (shortcut: string) => void;
  onCancel: () => void;
}) {
  const ref = useRef<HTMLButtonElement>(null);
  useHotkeyCapture(editing, ref, onCapture, onCancel);

  return (
    <button
      ref={ref}
      type="button"
      title="Click, then press the new keys. Esc cancels."
      aria-label={`${label} shortcut: ${value}. Click to change.`}
      onClick={editing ? undefined : onStartEdit}
      className="flex h-7 items-center justify-end rounded-md"
    >
      {editing ? (
        <span className="flex h-5 animate-fade-in items-center rounded-[4px] border border-ink px-1.5 font-mono text-[11px] text-ink">
          Press keys
        </span>
      ) : (
        <Keys hotkey={value} />
      )}
    </button>
  );
}

export function HotkeysSection() {
  const { entries, isLoading, editingKey, error, updateHotkey, resetToDefaults, startEditing, stopEditing } =
    useHotkeys();
  const [confirmReset, setConfirmReset] = useState(false);

  const handleCapture = useCallback(
    (key: string, shortcut: string) => {
      void updateHotkey(key, shortcut);
    },
    [updateHotkey],
  );

  const action = confirmReset ? (
    <span className="flex animate-fade-in items-center gap-3">
      <Button variant="ghost" className="h-6" onClick={() => setConfirmReset(false)}>
        Cancel
      </Button>
      <Button
        variant="danger"
        className="h-6 px-2"
        onClick={() => {
          void resetToDefaults();
          setConfirmReset(false);
        }}
      >
        Reset
      </Button>
    </span>
  ) : (
    <Button variant="ghost" className="h-6" onClick={() => setConfirmReset(true)}>
      Reset
    </Button>
  );

  return (
    <Section title="Hotkeys" action={action}>
      {entries.map((entry) => {
        const reserved = reservedHotkeyMessage(entry.value);
        return (
          <Row
            key={entry.key}
            label={entry.label}
            hint={entry.description}
            below={
              reserved && (
                <p className="flex items-center gap-1.5 pb-1 text-[12px] text-ink/60">
                  <Icon name="warning" size={12} className="shrink-0" />
                  {reserved}
                </p>
              )
            }
          >
            {!isLoading && (
              <KeyField
                label={entry.label}
                value={entry.value}
                editing={editingKey === entry.key}
                onStartEdit={() => startEditing(entry.key)}
                onCapture={(shortcut) => handleCapture(entry.key, shortcut)}
                onCancel={stopEditing}
              />
            )}
          </Row>
        );
      })}
      {error && (
        <div className="py-2">
          <Notice>{error}</Notice>
        </div>
      )}
    </Section>
  );
}
