import { getCurrentWindow } from "@tauri-apps/api/window";

const win = getCurrentWindow();

export function TitleBar() {
  return (
    <div
      data-tauri-drag-region
      className="flex items-center justify-end h-8 bg-[#eeeeee] shrink-0 select-none overflow-hidden"
    >
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
  );
}
