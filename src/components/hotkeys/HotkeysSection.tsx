// Hotkeys section of the Dictation page -- remappable shortcuts with live key capture
import { useCallback, useRef, useState } from "react";
import { useHotkeys } from "@/hooks/useHotkeys";
import { useHotkeyCapture } from "@/hooks/useHotkeyCapture";
import { reservedHotkeyMessage } from "@/lib/hotkeyReservations";
import { Button } from "@/components/ui/Button";
import { Icon } from "@/components/ui/Icon";
import { Keys } from "@/components/ui/Keys";
import { Row, Section } from "@/components/settings/controls";
import { cn } from "@/lib/utils";

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
      aria-label={`${label} shortcut: ${value}. Click to change.`}
      onClick={editing ? undefined : onStartEdit}
      className={cn(
        "flex h-9 min-w-[120px] items-center justify-end gap-1 rounded-full px-1.5 transition-[background-color,box-shadow] duration-200",
        editing ? "bg-accent/[0.1] shadow-[0_0_0_1.5px_rgb(var(--accent)/0.6)]" : "hover:bg-ink/[0.06]",
      )}
    >
      {editing ? (
        <span className="w-full animate-fade-in px-2 text-center text-[12px] font-medium text-accent">
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
    <span className="flex animate-fade-in items-center gap-1.5">
      <Button size="sm" variant="ghost" onClick={() => setConfirmReset(false)}>
        Cancel
      </Button>
      <Button
        size="sm"
        variant="danger"
        onClick={() => {
          void resetToDefaults();
          setConfirmReset(false);
        }}
      >
        Reset hotkeys
      </Button>
    </span>
  ) : (
    <Button size="sm" variant="ghost" onClick={() => setConfirmReset(true)}>
      Reset
    </Button>
  );

  return (
    <Section title="Hotkeys" action={action}>
      {entries.map((entry) => {
        const reserved = reservedHotkeyMessage(entry.value);
        const editing = editingKey === entry.key;
        return (
          <Row
            key={entry.key}
            label={entry.label}
            hint={editing ? "Press the new combination, or Esc to keep the current one." : entry.description}
            below={
              reserved && (
                <p className="mt-1.5 flex items-center gap-1.5 text-[12px] font-medium text-warn">
                  <Icon name="warning" size={13} className="shrink-0" />
                  {reserved}
                </p>
              )
            }
          >
            {!isLoading && (
              <KeyField
                label={entry.label}
                value={entry.value}
                editing={editing}
                onStartEdit={() => startEditing(entry.key)}
                onCapture={(shortcut) => handleCapture(entry.key, shortcut)}
                onCancel={stopEditing}
              />
            )}
          </Row>
        );
      })}
      {error && (
        <p role="alert" className="px-4 py-3 text-[12px] font-medium text-danger">
          {error}
        </p>
      )}
    </Section>
  );
}
