// Hotkeys hook -- settings load, optimistic update, and reset to defaults
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
  readOnly: boolean;
}

const HOTKEY_DEFINITIONS: {
  key: string;
  label: string;
  description: string;
  defaultValue: string;
  readOnly: boolean;
}[] = [
  {
    key: "hotkey_record",
    label: "Record",
    description: "Hold to dictate",
    defaultValue: "F8",
    readOnly: false,
  },
  {
    key: "hotkey_hands_free",
    label: "Hands-free",
    description: "Toggle dictation on or off",
    defaultValue: "Ctrl+F8",
    readOnly: false,
  },
  {
    key: "hotkey_cancel",
    label: "Cancel",
    description: "Stop without injecting",
    defaultValue: "Escape",
    readOnly: false,
  },
  {
    key: "hotkey_paste_last",
    label: "Paste Last",
    description: "Re-inject last dictation",
    defaultValue: "Ctrl+Alt+J",
    readOnly: false,
  },
  {
    key: "hotkey_open_app",
    label: "Open App",
    description: "Show LocalYapper window",
    defaultValue: "Ctrl+Alt+O",
    readOnly: false,
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
    readOnly: def.readOnly,
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
