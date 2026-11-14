// Wizard chrome -- consistent layout with step indicator and back navigation
import type { ReactNode } from "react";
import { Icon } from "@/components/ui/Icon";

interface WizardChromeProps {
  stepNumber: number;
  totalSteps: number;
  canGoBack: boolean;
  onBack: () => void;
  children: ReactNode;
}

export function WizardChrome({
  stepNumber,
  totalSteps,
  canGoBack,
  onBack,
  children,
}: WizardChromeProps) {
  // Header, card and footer are stacked, never layered, and the card
  // area scrolls: on a small window, or when a step is tall, every button
  // can still be reached and nothing is hidden behind the header or footer.
  return (
    <div className="flex h-full w-full flex-col overflow-hidden bg-gradient-to-b from-canvas to-sidebar">
      <header className="flex h-12 shrink-0 items-center justify-between px-6">
        {canGoBack ? (
          <button
            type="button"
            onClick={onBack}
            className="inline-flex items-center gap-1 rounded-full px-3 py-1.5 text-[12px] font-medium text-black/55 transition-colors hover:bg-black/[0.04] hover:text-black/85"
            aria-label="Go back to the previous step"
          >
            <Icon name="arrow_back" size={16} />
            Back
          </button>
        ) : (
          <span className="h-8 w-16" aria-hidden="true" />
        )}

        <span
          className="rounded-full bg-surface/70 px-3 py-1 text-[11px] font-semibold uppercase tracking-[0.08em] text-black/55 shadow-sm backdrop-blur"
          aria-label={`Step ${stepNumber} of ${totalSteps}`}
        >
          Step {stepNumber} of {totalSteps}
        </span>

        <span className="h-8 w-16" aria-hidden="true" />
      </header>

      <main className="wizard-scroll flex min-h-0 flex-1 overflow-y-auto px-6 py-2">
        <div className="m-auto w-full max-w-[480px] rounded-[14px] bg-surface p-6 shadow-[0_8px_40px_rgba(0,0,0,0.10)]">
          {children}
        </div>
      </main>

      <footer className="shrink-0 px-6 py-2.5 text-center text-[11px] font-medium text-black/[0.35]">
        Everything happens on your device. No audio leaves your machine.
      </footer>
    </div>
  );
}
