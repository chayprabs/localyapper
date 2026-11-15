// Hotkeys hook -- settings load, optimistic update, and reset to defaults
import { DEFAULT_RECORD_HOTKEY } from "@/lib/hotkeyDefaults";
import { useState, useEffect, useCallback } from "react";
import { getAllSettings } from "@/lib/commands/settings";
import {
  updateHotkey as updateHotkeyCmd,
  resetHotkeys as resetHotkeysCmd,
} from "@/lib/commands/hotkeys";

interface HotkeyEntry {
  key: string;
  label: string;
  description: string;
  value: string;
}

const HOTKEY_DEFINITIONS: {
  key: string;
  label: string;
  description: string;
  defaultValue: string;
}[] = [
  {
    key: "hotkey_record",
    label: "Record",
    description: "Hold to dictate. Double-tap for hands-free.",
    defaultValue: DEFAULT_RECORD_HOTKEY,
  },
  {
    key: "hotkey_cancel",
    label: "Cancel",
    description: "Discards the recording in progress.",
    defaultValue: "Escape",
  },
  {
    key: "hotkey_paste_last",
    label: "Paste last",
    description: "Pastes the latest dictation again.",
    defaultValue: "Ctrl+Alt+J",
  },
  {
    key: "hotkey_open_app",
    label: "Open app",
    description: "Shows this window.",
    defaultValue: "Ctrl+Alt+O",
  },
];

export function useHotkeys() {
  const [hotkeys, setHotkeys] = useState<Record<string, string>>({});
  const [isLoading, setIsLoading] = useState(true);
  const [editingKey, setEditingKey] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    getAllSettings()
      .then((settings) => {
        const hotkeySettings: Record<string, string> = {};
        for (const def of HOTKEY_DEFINITIONS) {
          hotkeySettings[def.key] = settings[def.key] ?? def.defaultValue;
        }
        setHotkeys(hotkeySettings);
        setError(null);
      })
      .catch((e) => {
        console.error("Failed to load hotkeys:", e);
        setError(
          e instanceof Error
            ? e.message
            : typeof e === "string"
              ? e
              : "Failed to load hotkeys",
        );
      })
      .finally(() => setIsLoading(false));
  }, []);

  const updateHotkey = useCallback(
    async (key: string, value: string) => {
      const previous = hotkeys[key];
      setError(null);
      // Optimistic update
      setHotkeys((prev) => ({ ...prev, [key]: value }));
      setEditingKey(null);

      try {
        await updateHotkeyCmd(key, value);
      } catch (e) {
        // Rollback on error
        console.error("Failed to update hotkey:", e);
        setError(
          e instanceof Error
            ? e.message
            : typeof e === "string"
              ? e
              : "Failed to update hotkey",
        );
        setHotkeys((prev) => ({ ...prev, [key]: previous ?? "" }));
      }
    },
    [hotkeys],
  );

  const resetToDefaults = useCallback(async () => {
    setError(null);
    try {
      const defaults = await resetHotkeysCmd();
      setHotkeys(defaults);
      setEditingKey(null);
    } catch (e) {
      console.error("Failed to reset hotkeys:", e);
      setError(
        e instanceof Error
          ? e.message
          : typeof e === "string"
            ? e
            : "Failed to reset hotkeys",
      );
    }
  }, []);

  const startEditing = useCallback((key: string) => {
    setError(null);
    setEditingKey(key);
  }, []);

  const stopEditing = useCallback(() => {
    setEditingKey(null);
  }, []);

  const entries: HotkeyEntry[] = HOTKEY_DEFINITIONS.map((def) => ({
    key: def.key,
    label: def.label,
    description: def.description,
    value: hotkeys[def.key] ?? def.defaultValue,
  }));

  return {
    entries,
    isLoading,
    editingKey,
    error,
    updateHotkey,
    resetToDefaults,
    startEditing,
    stopEditing,
  };
}
