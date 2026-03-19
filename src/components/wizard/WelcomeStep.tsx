export function WelcomeStep({
  onGetStarted,
  onSkip,
}: {
  onGetStarted: () => void;
  onSkip: () => void;
}) {
  return (
    <div className="flex flex-col items-center text-center">
      {/* Logo */}
      <div className="w-16 h-16 bg-gradient-to-b from-[#0062d0] to-[#0058bc] rounded-2xl flex items-center justify-center mb-5 shadow-lg">
        <span className="material-symbols-outlined text-white text-[32px]">
          mic
        </span>
      </div>

      <h1 className="text-[22px] font-semibold text-black/85 mb-2">
        Welcome to LocalYapper
      </h1>
      <p className="text-[14px] text-black/50 mb-8 max-w-[340px] leading-relaxed">
        Voice dictation that runs entirely on your device. No cloud, no
        subscriptions, no data leaves your machine.
      </p>

      <button
        onClick={onGetStarted}
        className="w-full h-9 bg-gradient-to-b from-[#0062d0] to-[#0058bc] text-white text-[13px] font-medium rounded-[8px] hover:brightness-110 active:brightness-95 transition-all"
      >
        Get Started
      </button>

      <button
        onClick={onSkip}
        className="mt-3 text-[13px] text-black/[0.40] hover:text-black/60 transition-colors"
      >
        Skip setup
      </button>
    </div>
  );
}
