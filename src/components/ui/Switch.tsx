// Switch toggle: hollow when off, solid ink when on.
import { cn } from "@/lib/utils";

interface SwitchProps {
  checked: boolean;
  onChange: (next: boolean) => void;
  disabled?: boolean;
  /** Accessible label; falls back to a generic toggle label. */
  ariaLabel?: string;
  className?: string;
}

export function Switch({ checked, onChange, disabled = false, ariaLabel = "Toggle", className }: SwitchProps) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={ariaLabel}
      disabled={disabled}
      onClick={() => {
        if (!disabled) onChange(!checked);
      }}
      className={cn(
        "relative inline-flex h-[18px] w-[30px] shrink-0 items-center rounded-full border transition-colors duration-150",
        checked ? "border-ink bg-ink" : "border-ink/30 bg-transparent",
        disabled && "opacity-40",
        className,
      )}
    >
      <span
        className={cn(
          "inline-block h-3 w-3 rounded-full transition-transform duration-150 ease-out",
          checked ? "translate-x-[14px] bg-canvas" : "translate-x-[2px] bg-ink/45",
        )}
      />
    </button>
  );
}
