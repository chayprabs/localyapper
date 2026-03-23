import { getCurrentWindow } from "@tauri-apps/api/window";
import logoImg from "@/assets/logo-nobg.png";

export function TitleBar() {
  const handleMinimize = () => getCurrentWindow().minimize();
  const handleMaximize = () => getCurrentWindow().toggleMaximize();
  const handleClose = () => getCurrentWindow().close();

  return (
    <div
      data-tauri-drag-region
      className="flex items-center justify-between h-8 bg-[#eeeeee] shrink-0 select-none"
    >
      <div
        data-tauri-drag-region
        className="pl-4 flex items-center gap-1.5"
      >
        <img src={logoImg} alt="LocalYapper" className="w-[14px] h-[14px]" draggable={false} />
        <span data-tauri-drag-region className="text-[13px] font-semibold text-[#1C1C1E]">
          LocalYapper
        </span>
      </div>

      <div className="flex items-center">
        <button
          onClick={handleMinimize}
          className="w-10 h-8 flex items-center justify-center text-black/35 hover:bg-black/[0.08] transition-colors"
          title="Minimize"
        >
          <svg width="10" height="1" viewBox="0 0 10 1" fill="currentColor">
            <rect width="10" height="1" />
          </svg>
        </button>
        <button
          onClick={handleMaximize}
          className="w-10 h-8 flex items-center justify-center text-black/35 hover:bg-black/[0.08] transition-colors"
          title="Maximize"
        >
          <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" strokeWidth="1">
            <rect x="0.5" y="0.5" width="9" height="9" />
          </svg>
        </button>
        <button
          onClick={handleClose}
          className="w-10 h-8 flex items-center justify-center text-black/35 hover:bg-red-500 hover:text-white transition-colors"
          title="Close"
        >
          <svg width="10" height="10" viewBox="0 0 10 10" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round">
            <line x1="1" y1="1" x2="9" y2="9" />
            <line x1="9" y1="1" x2="1" y2="9" />
          </svg>
        </button>
      </div>
    </div>
  );
}
