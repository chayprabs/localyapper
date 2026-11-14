// Shortcut strings ("Ctrl+Alt+J"): display parts and capture from key events.
import { isMacPlatform } from "@/lib/hotkeyDefaults";

/** A modifier as the platform writes it: symbols on macOS, names elsewhere. */
function formatKey(key: string): string {
  if (!isMacPlatform) return key;
  switch (key.toLowerCase()) {
    case "alt":
      return "⌥";
    case "shift":
      return "⇧";
    case "meta":
    case "cmd":
      return "⌘";
    case "ctrl":
    case "control":
      return "⌃";
    default:
      return key;
  }
}

/** The key caps to draw for a shortcut string. */
export function hotkeyParts(hotkey: string): string[] {
  return hotkey.split("+").map(formatKey);
}

/** A Tauri-style shortcut for a key event, or null for a lone modifier. */
export function eventToShortcut(e: KeyboardEvent): string | null {
  if (["Alt", "Control", "Shift", "Meta", "AltGraph"].includes(e.key)) return null;

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
