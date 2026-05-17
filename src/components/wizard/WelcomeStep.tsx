// Wizard welcome step -- product introduction with Get Started action
export function WelcomeStep({
  onGetStarted,
  onSkip,
  error,
}: {
  onGetStarted: () => void;
  onSkip: () => void;
  error: string | null;
}) {
  return (
    <div className="flex flex-col items-center text-center">
      <div className="mb-5 flex h-16 w-16 items-center justify-center rounded-2xl bg-gradient-to-b from-[#0062d0] to-[#0058bc] shadow-lg">
        <span className="material-symbols-outlined text-[32px] text-white">
          mic
        </span>
      </div>

      <h1 className="mb-2 text-[22px] font-semibold text-black/85">
        Welcome to LocalYapper
      </h1>
      <p className="mb-8 max-w-[360px] text-[14px] leading-relaxed text-black/50">
        Press a key, speak, let go. Your words appear wherever your cursor is.
        Everything happens on this device.
      </p>

      <button
        type="button"
        onClick={onGetStarted}
        className="h-9 w-full rounded-[8px] bg-gradient-to-b from-[#0062d0] to-[#0058bc] text-[13px] font-medium text-white transition-all hover:brightness-110 active:brightness-95"
      >
        Get started
      </button>

      <button
        type="button"
        onClick={onSkip}
        className="mt-3 text-[13px] text-black/[0.40] transition-colors hover:text-black/60"
      >
        Skip setup
      </button>

      {error && (
        <p className="mt-4 w-full rounded-lg border border-[#ba1a1a]/15 bg-[#ba1a1a]/[0.06] px-3 py-2 text-[12px] font-medium text-[#ba1a1a]">
          {error}
        </p>
      )}
    </div>
  );
}
