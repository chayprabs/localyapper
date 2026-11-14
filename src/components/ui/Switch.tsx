// Switch toggle.
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
        "group relative inline-flex h-[22px] w-[38px] shrink-0 items-center rounded-full transition-colors duration-200",
        checked ? "bg-accent" : "bg-ink/[0.16]",
        disabled && "opacity-50",
        className,
      )}
    >
      <span
        className={cn(
          "inline-block h-[18px] w-[18px] rounded-full bg-white shadow-[0_1px_3px_rgba(0,0,0,0.3)]",
          "transition-[transform,width] duration-300 ease-spring group-active:w-[22px]",
          checked ? "translate-x-[18px] group-active:translate-x-[14px]" : "translate-x-[2px]",
        )}
      />
    </button>
  );
}
