// Key caps for a shortcut string such as "Ctrl+Alt+J".
import { hotkeyParts } from "@/lib/hotkeyFormat";
import { cn } from "@/lib/utils";

export function Keys({ hotkey, size = "sm" }: { hotkey: string; size?: "sm" | "lg" }) {
  return (
    <span className="inline-flex items-center gap-1 align-middle">
      {hotkeyParts(hotkey).map((part, i) => (
        <kbd
          key={i}
          className={cn(
            "raised inline-flex items-center justify-center rounded-[7px] font-mono font-medium text-ink/85",
            size === "lg" ? "h-9 min-w-9 px-2.5 text-[15px]" : "h-6 min-w-6 px-1.5 text-[11px]",
          )}
        >
          {part}
        </kbd>
      ))}
    </span>
  );
}
