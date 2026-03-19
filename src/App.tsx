import { Overlay } from "@/components/overlay/Overlay";

export function App() {
  const isOverlay = window.location.pathname === "/overlay";

  if (isOverlay) {
    return <Overlay />;
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
