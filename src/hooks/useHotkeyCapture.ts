// While active, the next key combination is captured as a shortcut. Escape
// or a click outside `containerRef` cancels.
import { useEffect, type RefObject } from "react";
import { eventToShortcut } from "@/lib/hotkeyCapture";

export function useHotkeyCapture(
  active: boolean,
  containerRef: RefObject<HTMLElement | null>,
  onCapture: (shortcut: string) => void,
  onCancel: () => void,
): void {
  useEffect(() => {
    if (!active) return;

    function handleKeyDown(e: KeyboardEvent) {
      e.preventDefault();
      e.stopPropagation();
      // A key still held from opening the field (Enter, Space) is not a choice.
      if (e.repeat) return;
      if (e.key === "Escape") {
        onCancel();
        return;
      }
      const shortcut = eventToShortcut(e);
      if (shortcut) onCapture(shortcut);
    }

    function handleMouseDown(e: MouseEvent) {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        onCancel();
      }
    }

    window.addEventListener("keydown", handleKeyDown, true);
    document.addEventListener("mousedown", handleMouseDown);
    return () => {
      window.removeEventListener("keydown", handleKeyDown, true);
      document.removeEventListener("mousedown", handleMouseDown);
    };
  }, [active, containerRef, onCapture, onCancel]);
}
