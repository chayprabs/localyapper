// Turns a key press into a shortcut string the backend can register.
//
// The physical key (`event.code`) names the main key, never the character
// it types: with Shift or Alt held, `event.key` is "!" or "∆", which no
// shortcut parser accepts. Every name returned here is covered by a test
// in src-tauri/src/platform/hotkey/shortcut.rs.

const NAMED_KEYS: ReadonlySet<string> = new Set([
  "Space",
  "Enter",
  "Tab",
  "Escape",
  "Backspace",
  "Delete",
  "Insert",
  "Home",
  "End",
  "PageUp",
  "PageDown",
  "ArrowUp",
  "ArrowDown",
  "ArrowLeft",
  "ArrowRight",
  "Backquote",
  "Minus",
  "Equal",
  "BracketLeft",
  "BracketRight",
  "Backslash",
  "Semicolon",
  "Quote",
  "Comma",
  "Period",
  "Slash",
  "CapsLock",
  "ScrollLock",
  "Pause",
  "PrintScreen",
  "NumpadAdd",
  "NumpadSubtract",
  "NumpadMultiply",
  "NumpadDivide",
  "NumpadDecimal",
  "NumpadEnter",
]);

/** The main key's name, or null for modifiers and keys that cannot be bound. */
export function mainKeyName(code: string): string | null {
  const letter = /^Key([A-Z])$/.exec(code);
  if (letter?.[1]) return letter[1];
  const digit = /^Digit([0-9])$/.exec(code);
  if (digit?.[1]) return digit[1];
  if (/^F([1-9]|1[0-9]|2[0-4])$/.test(code)) return code;
  if (/^Numpad[0-9]$/.test(code)) return code;
  return NAMED_KEYS.has(code) ? code : null;
}

interface KeyLike {
  code: string;
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
  metaKey: boolean;
}

/** "Ctrl+Alt+J" for a key press, or null while only modifiers are held. */
export function eventToShortcut(e: KeyLike): string | null {
  const key = mainKeyName(e.code);
  if (!key) return null;
  const parts: string[] = [];
  if (e.ctrlKey) parts.push("Ctrl");
  if (e.altKey) parts.push("Alt");
  if (e.shiftKey) parts.push("Shift");
  if (e.metaKey) parts.push("Meta");
  parts.push(key);
  return parts.join("+");
}
