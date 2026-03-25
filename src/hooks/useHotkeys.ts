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
    defaultValue: "Ctrl+Shift+Space",
    readOnly: false,
  },
  {
    key: "hotkey_hands_free",
    label: "Hands-free",
    description: "Double-tap to toggle",
    defaultValue: "Ctrl+Shift+Space",
    readOnly: true,
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
    defaultValue: "Alt+Shift+V",
    readOnly: false,
  },
  {
    key: "hotkey_open_app",
    label: "Open App",
    description: "Show LocalYapper window",
    defaultValue: "Alt+L",
    readOnly: false,
  },
];

export function useHotkeys() {
  const [hotkeys, setHotkeys] = useState<Record<string, string>>({});
  const [isLoading, setIsLoading] = useState(true);
  const [editingKey, setEditingKey] = useState<string | null>(null);

  useEffect(() => {
    getAllSettings()
      .then((settings) => {
        const hotkeySettings: Record<string, string> = {};
        for (const def of HOTKEY_DEFINITIONS) {
          hotkeySettings[def.key] = settings[def.key] ?? def.defaultValue;
        }
        setHotkeys(hotkeySettings);
      })
      .catch((e) => console.error("Failed to load hotkeys:", e))
      .finally(() => setIsLoading(false));
  }, []);

  const updateHotkey = useCallback(
    async (key: string, value: string) => {
      const previous = hotkeys[key];
      // Optimistic update
      setHotkeys((prev) => {
        const next = { ...prev, [key]: value };
        // Auto-sync hands_free with record
        if (key === "hotkey_record") {
          next["hotkey_hands_free"] = value;
        }
        return next;
      });
      setEditingKey(null);

      try {
        await updateHotkeyCmd(key, value);
      } catch (e) {
        // Rollback on error
        console.error("Failed to update hotkey:", e);
        setHotkeys((prev) => {
          const next = { ...prev, [key]: previous ?? "" };
          if (key === "hotkey_record") {
            next["hotkey_hands_free"] = previous ?? "";
          }
          return next;
        });
      }
    },
    [hotkeys],
  );

  const resetToDefaults = useCallback(async () => {
    try {
      const defaults = await resetHotkeysCmd();
      setHotkeys(defaults);
      setEditingKey(null);
    } catch (e) {
      console.error("Failed to reset hotkeys:", e);
    }
  }, []);

  const startEditing = useCallback((key: string) => {
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
    updateHotkey,
    resetToDefaults,
    startEditing,
    stopEditing,
  };
}
