// Key caps for a shortcut string such as "Ctrl+Alt+J".
import { hotkeyParts } from "@/lib/hotkeyFormat";
import { cn } from "@/lib/utils";

const SIZES = {
  sm: "h-5 min-w-5 rounded-[4px] px-1.5 text-[11px]",
  lg: "h-8 min-w-8 rounded-[5px] px-2.5 text-[14px]",
  xl: "h-16 min-w-16 rounded-lg px-5 text-[26px]",
};

export function Keys({ hotkey, size = "sm" }: { hotkey: string; size?: keyof typeof SIZES }) {
  return (
    <span className={cn("inline-flex items-center align-middle", size === "xl" ? "gap-2" : "gap-1")}>
      {hotkeyParts(hotkey).map((part, i) => (
        <kbd
          key={i}
          className={cn("inline-flex items-center justify-center border border-ink/25 font-mono text-ink/90", SIZES[size])}
        >
          {part}
        </kbd>
      ))}
    </span>
  );
}
