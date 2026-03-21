import { getCurrentWindow } from "@tauri-apps/api/window";

const win = getCurrentWindow();

export function TitleBar() {
  return (
    <div
      data-tauri-drag-region
      className="flex items-center justify-between h-8 bg-[#f9f9f9] shrink-0 select-none"
    >
      <span
        data-tauri-drag-region
        className="pl-4 text-[13px] font-semibold text-[#1C1C1E]"
      >
        LocalYapper
      </span>

      <div className="flex items-center">
        <button
          onClick={() => win.minimize()}
          className="w-10 h-8 flex items-center justify-center text-black/35 hover:bg-black/[0.08] transition-colors"
          title="Minimize"
        >
          <span
            className="material-symbols-outlined text-[14px]"
            style={{ fontVariationSettings: "'opsz' 14" }}
          >
            remove
          </span>
        </button>
        <button
          onClick={() => win.toggleMaximize()}
          className="w-10 h-8 flex items-center justify-center text-black/35 hover:bg-black/[0.08] transition-colors"
          title="Maximize"
        >
          <span
            className="material-symbols-outlined text-[14px]"
            style={{ fontVariationSettings: "'opsz' 14" }}
          >
            crop_square
          </span>
        </button>
        <button
          onClick={() => win.close()}
          className="w-10 h-8 flex items-center justify-center text-black/35 hover:bg-red-500 hover:text-white transition-colors"
          title="Close"
        >
          <span
            className="material-symbols-outlined text-[14px]"
            style={{ fontVariationSettings: "'opsz' 14" }}
          >
            close
          </span>
        </button>
      </div>
    </div>
  );
}
