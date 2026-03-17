export function App() {
  const isOverlay = window.location.pathname === "/overlay";

  if (isOverlay) {
    return (
      <div className="flex items-center justify-center h-screen bg-transparent">
        <div className="flex items-center gap-3 px-4 py-3 bg-white/95 border border-black/10 rounded-full shadow-[0_4px_24px_rgba(0,0,0,0.15)]">
          <span className="text-[13px] text-[#1C1C1E] font-sans">
            Overlay ready
          </span>
        </div>
      </div>
    );
  }

  return (
    <div className="flex items-center justify-center h-screen bg-[#EDEDED] font-sans">
      <div className="text-center">
        <h1 className="text-[26px] font-semibold text-[#1C1C1E] mb-2">
          LocalYapper
        </h1>
        <p className="text-[13px] text-black/50">
          Local-first voice dictation
        </p>
      </div>
    </div>
  );
}
