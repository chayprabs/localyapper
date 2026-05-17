// Curated list of system hotkey combinations that the OS is likely to
// intercept before LocalYapper sees them. The list is intentionally small and
// conservative -- false positives would be more annoying than the warning is
// useful. The check is case-insensitive and tolerant of `Cmd`/`Meta` aliases.

interface OsHotkeys {
  os: "windows" | "macos" | "linux";
  combos: readonly string[];
}

const RESERVED: readonly OsHotkeys[] = [
  {
    os: "windows",
    combos: [
      "Ctrl+Alt+Delete",
      "Ctrl+Alt+Del",
      "Ctrl+Shift+Escape",
      "Alt+Tab",
      "Alt+F4",
      "Meta+L",
      "Meta+D",
      "Meta+E",
      "Meta+R",
      "Meta+Tab",
      "Meta+I",
      "Meta+X",
      "Meta+Period",
      "Meta+Space",
      "PrintScreen",
    ],
  },
  {
    os: "macos",
    combos: [
      "Meta+Q",
      "Meta+W",
      "Meta+Space",
      "Meta+Tab",
      "Meta+Shift+3",
      "Meta+Shift+4",
      "Meta+Shift+5",
      "Meta+Ctrl+Q",
      "Meta+Alt+Escape",
      "Meta+Alt+D",
    ],
  },
  {
    os: "linux",
    combos: [
      "Ctrl+Alt+Delete",
      "Ctrl+Alt+T",
      "Alt+Tab",
      "Alt+F4",
      "Meta+L",
      "Meta+D",
      "Meta+Tab",
      "PrintScreen",
    ],
  },
];

function detectOs(): OsHotkeys["os"] {
  if (typeof navigator === "undefined") return "linux";
  const ua = navigator.userAgent.toLowerCase();
  if (ua.includes("windows")) return "windows";
  if (ua.includes("mac os") || ua.includes("macos") || ua.includes("macintosh")) {
    return "macos";
  }
  return "linux";
}

function normalize(combo: string): string {
  return combo
    .split("+")
    .map((part) => {
      const p = part.trim().toLowerCase();
      if (p === "cmd" || p === "command") return "meta";
      if (p === "control") return "ctrl";
      if (p === "option") return "alt";
      if (p === "delete") return "del";
      return p;
    })
    .filter((part) => part.length > 0)
    .join("+");
}

/**
 * Returns a soft-warning message when the captured combo overlaps with a
 * platform-reserved shortcut, or null if the combo is fine. The hotkey is
 * still saved either way -- the warning only surfaces in the UI so the user
 * is aware the OS may swallow the keystroke before it reaches LocalYapper.
 */
export function reservedHotkeyMessage(shortcut: string): string | null {
  const trimmed = shortcut.trim();
  if (trimmed.length === 0) return null;

  const target = normalize(trimmed);
  const os = detectOs();
  const entry = RESERVED.find((e) => e.os === os);
  if (!entry) return null;

  const matched = entry.combos.find((combo) => normalize(combo) === target);
  if (!matched) return null;

  return `${matched} is usually reserved by ${osLabel(os)}. The OS may handle it before LocalYapper does.`;
}

function osLabel(os: OsHotkeys["os"]): string {
  switch (os) {
    case "windows":
      return "Windows";
    case "macos":
      return "macOS";
    case "linux":
      return "your desktop environment";
  }
}
