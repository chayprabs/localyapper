// Shared layout for a wizard step: title, one line of text, then the
// step's own content and actions.
import type { ReactNode } from "react";

export function Step({ title, text, children }: { title: string; text?: ReactNode; children: ReactNode }) {
  return (
    <div className="animate-fade-in">
      <h1 className="text-[22px] font-semibold leading-tight tracking-[-0.02em] text-ink">{title}</h1>
      {text && <p className="mt-2 text-[13px] leading-relaxed text-ink/55">{text}</p>}
      <div className="mt-7 flex flex-col items-start gap-5">{children}</div>
    </div>
  );
}

/** The step's buttons, side by side. */
export function Actions({ children }: { children: ReactNode }) {
  return <div className="flex items-center gap-4">{children}</div>;
}
