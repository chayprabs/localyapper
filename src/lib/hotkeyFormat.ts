// Shortcut strings ("Ctrl+Alt+J") as key caps. Capturing a shortcut from a
// key press lives in hotkeyCapture.ts.
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
