// Wizard chrome -- consistent layout with step indicator and back navigation
import type { ReactNode } from "react";

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
  return (
    <div className="relative h-full w-full overflow-hidden bg-gradient-to-b from-[#F2F2F4] to-[#E6E6EA]">
      <header className="absolute top-0 left-0 right-0 flex h-14 items-center justify-between px-6">
        {canGoBack ? (
          <button
            type="button"
            onClick={onBack}
            className="inline-flex items-center gap-1 rounded-full px-3 py-1.5 text-[12px] font-medium text-black/55 transition-colors hover:bg-black/[0.04] hover:text-black/85"
            aria-label="Go back to the previous step"
          >
            <span className="material-symbols-outlined text-[16px]">
              arrow_back
            </span>
            Back
          </button>
        ) : (
          <span className="h-8" aria-hidden="true" />
        )}

        <span
          className="rounded-full bg-white/70 px-3 py-1 text-[11px] font-semibold uppercase tracking-[0.08em] text-black/55 shadow-sm backdrop-blur"
          aria-label={`Step ${stepNumber} of ${totalSteps}`}
        >
          Step {stepNumber} of {totalSteps}
        </span>

        <span className="h-8 w-16" aria-hidden="true" />
      </header>

      <main className="flex h-full w-full items-center justify-center px-6">
        <div className="w-[480px] rounded-[14px] bg-white p-7 shadow-[0_8px_40px_rgba(0,0,0,0.10)]">
          {children}
        </div>
      </main>

      <footer className="absolute bottom-4 left-0 right-0 text-center text-[11px] font-medium text-black/[0.35]">
        Everything happens on your device. No audio leaves your machine.
      </footer>
    </div>
  );
}
