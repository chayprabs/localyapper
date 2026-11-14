// Shared layout for a wizard step: glass disc, title, text, then the
// step's own content and actions.
import type { ReactNode } from "react";
import { Icon, type IconName } from "@/components/ui/Icon";
import { cn } from "@/lib/utils";

export function Step({
  icon,
  tone = "accent",
  title,
  text,
  children,
}: {
  icon: IconName;
  tone?: "accent" | "ok";
  title: string;
  text: ReactNode;
  children: ReactNode;
}) {
  return (
    <div className="stagger flex flex-col items-center text-center">
      <div className="glass flex h-16 w-16 items-center justify-center rounded-full">
        <Icon name={icon} size={26} strokeWidth={1.5} className={tone === "ok" ? "text-ok" : "text-accent"} />
      </div>
      <div className="mt-6">
        <h1 className="text-[24px] font-semibold leading-tight tracking-[-0.02em] text-ink">{title}</h1>
        <p className="mx-auto mt-2.5 max-w-[360px] text-[14px] leading-relaxed text-ink/50">{text}</p>
      </div>
      <div className="mt-7 flex w-full flex-col items-center gap-3">{children}</div>
    </div>
  );
}

/** A quiet text button under the main action. */
export function TextButton({ className, ...props }: React.ButtonHTMLAttributes<HTMLButtonElement>) {
  return (
    <button
      type="button"
      className={cn("text-[12.5px] text-ink/45 transition-colors hover:text-ink/80", className)}
      {...props}
    />
  );
}
