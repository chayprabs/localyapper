// Default record hotkey per OS (plan P2.2): F8, or Alt+Space on macOS.
export const isMacPlatform =
  typeof navigator !== "undefined" && /mac/i.test(navigator.userAgent);

export const DEFAULT_RECORD_HOTKEY = isMacPlatform ? "Alt+Space" : "F8";
