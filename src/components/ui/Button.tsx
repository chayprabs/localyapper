// Buttons: flat, four variants.
import type { ButtonHTMLAttributes } from "react";
import { cn } from "@/lib/utils";

type Variant = "primary" | "outline" | "ghost" | "danger";
type Size = "md" | "lg";

const VARIANTS: Record<Variant, string> = {
  primary: "bg-ink text-canvas hover:bg-ink/80",
  outline: "border border-ink/20 text-ink/85 hover:border-ink/50 hover:text-ink",
  ghost: "text-ink/50 hover:text-ink",
  danger: "border border-signal/40 text-signal hover:border-signal hover:bg-signal hover:text-canvas",
};

const SIZES: Record<Size, string> = {
  md: "h-7 px-2.5 text-[12.5px]",
  lg: "h-9 px-4 text-[13px]",
};

const BASE =
  "inline-flex shrink-0 select-none items-center justify-center gap-1.5 rounded-md font-medium " +
  "transition-colors duration-100 disabled:pointer-events-none disabled:opacity-35";

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
  size?: Size;
}

export function Button({ variant = "outline", size = "md", className, type = "button", ...props }: ButtonProps) {
  return (
    <button
      type={type}
      className={cn(BASE, VARIANTS[variant], SIZES[size], variant === "ghost" && "px-1", className)}
      {...props}
    />
  );
}

/** An icon-only button. `label` is the accessible name and tooltip. */
export function IconButton({
  label,
  tone = "ink",
  className,
  type = "button",
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement> & { label: string; tone?: "ink" | "danger" }) {
  return (
    <button
      type={type}
      aria-label={label}
      title={label}
      className={cn(BASE, "h-7 w-7 text-ink/40", tone === "danger" ? "hover:text-signal" : "hover:text-ink", className)}
      {...props}
    />
  );
}
