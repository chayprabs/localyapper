// Buttons: pill-shaped, four variants.
import type { ButtonHTMLAttributes } from "react";
import { cn } from "@/lib/utils";

type Variant = "primary" | "soft" | "ghost" | "danger";
type Size = "sm" | "md" | "lg";

const VARIANTS: Record<Variant, string> = {
  primary: "bg-ink text-canvas hover:bg-ink/85",
  soft: "bg-ink/[0.06] text-ink/80 hover:bg-ink/[0.1] hover:text-ink",
  ghost: "text-ink/55 hover:bg-ink/[0.06] hover:text-ink",
  danger: "bg-danger/10 text-danger hover:bg-danger/[0.16]",
};

const SIZES: Record<Size, string> = {
  sm: "h-7 px-3 text-[12px]",
  md: "h-8 px-3.5 text-[13px]",
  lg: "h-10 px-5 text-[14px]",
};

const BASE =
  "inline-flex shrink-0 select-none items-center justify-center gap-1.5 rounded-full font-medium " +
  "transition-[background-color,color,transform,opacity] duration-150 ease-out active:scale-[0.97] " +
  "disabled:pointer-events-none disabled:opacity-40";

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
  size?: Size;
}

export function Button({ variant = "soft", size = "md", className, type = "button", ...props }: ButtonProps) {
  return <button type={type} className={cn(BASE, VARIANTS[variant], SIZES[size], className)} {...props} />;
}

/** A round, icon-only button. `label` is the accessible name and tooltip. */
export function IconButton({
  label,
  tone = "ghost",
  className,
  type = "button",
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement> & { label: string; tone?: "ghost" | "danger" }) {
  return (
    <button
      type={type}
      aria-label={label}
      title={label}
      className={cn(
        BASE,
        "h-8 w-8 text-ink/45",
        tone === "danger" ? "hover:bg-danger/10 hover:text-danger" : "hover:bg-ink/[0.07] hover:text-ink",
        className,
      )}
      {...props}
    />
  );
}
