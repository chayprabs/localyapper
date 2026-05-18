// Apple-style switch toggle. Single primitive, no shadcn dep.
import { cn } from "@/lib/utils";

interface SwitchProps {
  checked: boolean;
  onChange: (next: boolean) => void;
  disabled?: boolean;
  /** Accessible label; falls back to a generic toggle label. */
  ariaLabel?: string;
  className?: string;
}

export function Switch({
  checked,
  onChange,
  disabled = false,
  ariaLabel = "Toggle",
  className,
}: SwitchProps) {
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
        "relative inline-flex h-[24px] w-[40px] shrink-0 items-center rounded-full transition-colors",
        "focus:outline-none focus-visible:ring-2 focus-visible:ring-[#0058bc]/40",
        checked ? "bg-[#0058bc]" : "bg-black/[0.20]",
        disabled && "opacity-50 cursor-not-allowed",
        !disabled && "cursor-pointer",
        className,
      )}
    >
      <span
        className={cn(
          "inline-block h-[20px] w-[20px] rounded-full bg-white shadow-[0_1px_2px_rgba(0,0,0,0.20)] transition-transform",
          checked ? "translate-x-[18px]" : "translate-x-[2px]",
        )}
      />
    </button>
  );
}
