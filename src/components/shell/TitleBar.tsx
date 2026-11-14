// Title bar: drag region, page navigation and window controls.
import { useLayoutEffect, useRef, useState } from "react";
import { useAtom, useAtomValue } from "jotai";
import { getCurrentWindow } from "@tauri-apps/api/window";
import logoImg from "@/assets/logo-nobg.png";
import { activePageAtom, pausedAtom, type PageId } from "@/stores/appStore";
import { Icon, type IconName } from "@/components/ui/Icon";
import { cn } from "@/lib/utils";

const PAGES: { id: PageId; label: string; icon: IconName }[] = [
  { id: "home", label: "Home", icon: "home" },
  { id: "history", label: "History", icon: "history" },
  { id: "dictation", label: "Dictation", icon: "keyboard" },
  { id: "voice", label: "Voice", icon: "voice" },
  { id: "general", label: "General", icon: "settings" },
];

/** The tab bar: a glass track with a lifted pill that slides to the active tab. */
function Nav() {
  const [active, setActive] = useAtom(activePageAtom);
  const trackRef = useRef<HTMLDivElement>(null);
  const tabRefs = useRef(new Map<PageId, HTMLButtonElement>());
  const [pill, setPill] = useState<{ left: number; width: number } | null>(null);

  useLayoutEffect(() => {
    const measure = () => {
      const el = tabRefs.current.get(active);
      if (el) setPill({ left: el.offsetLeft, width: el.offsetWidth });
    };
    measure();
    const track = trackRef.current;
    if (!track) return;
    const observer = new ResizeObserver(measure);
    observer.observe(track);
    return () => observer.disconnect();
  }, [active]);

  return (
    <nav
      ref={trackRef}
      aria-label="Pages"
      className="glass relative flex h-9 items-center rounded-full p-[3px]"
    >
      {pill && (
        <span
          aria-hidden="true"
          className="raised absolute bottom-[3px] top-[3px] rounded-full transition-[left,width] duration-300 ease-out"
          style={{ left: pill.left, width: pill.width }}
        />
      )}
      {PAGES.map((page) => {
        const on = page.id === active;
        return (
          <button
            key={page.id}
            ref={(el) => {
              if (el) tabRefs.current.set(page.id, el);
              else tabRefs.current.delete(page.id);
            }}
            type="button"
            aria-current={on ? "page" : undefined}
            title={page.label}
            onClick={() => setActive(page.id)}
            className={cn(
              "relative flex h-full items-center gap-1.5 rounded-full px-3 text-[12.5px] font-medium transition-colors duration-200",
              on ? "text-ink" : "text-ink/45 hover:text-ink/75",
            )}
          >
            <Icon name={page.icon} size={15} strokeWidth={on ? 2 : 1.75} />
            {/* Narrow windows keep only the active tab's label. */}
            <span className={cn(!on && "hidden md:inline")}>{page.label}</span>
          </button>
        );
      })}
    </nav>
  );
}

function WindowButton({ label, icon, onClick, danger }: { label: string; icon: IconName; onClick: () => void; danger?: boolean }) {
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      onClick={onClick}
      className={cn(
        "flex h-7 w-7 items-center justify-center rounded-full text-ink/40 transition-colors duration-150",
        danger ? "hover:bg-danger hover:text-white" : "hover:bg-ink/[0.08] hover:text-ink",
      )}
    >
      <Icon name={icon} size={icon === "maximize" ? 11 : 14} strokeWidth={2} />
    </button>
  );
}

export function TitleBar({ showNav }: { showNav: boolean }) {
  const paused = useAtomValue(pausedAtom);
  const win = () => getCurrentWindow();

  return (
    <header
      data-tauri-drag-region
      className="relative z-10 grid h-14 shrink-0 select-none grid-cols-[1fr_auto_1fr] items-center gap-3 px-3"
    >
      <div data-tauri-drag-region className="flex min-w-0 items-center gap-2 pl-2">
        <img src={logoImg} alt="" className="logo pointer-events-none h-4 w-4 object-contain" draggable={false} />
        <span data-tauri-drag-region className="hidden truncate text-[13px] font-semibold text-ink/80 sm:inline">
          LocalYapper
        </span>
        {paused === true && (
          <span
            className="inline-flex h-5 animate-fade-in items-center gap-1 rounded-full bg-warn/[0.12] px-2 text-[11px] font-medium text-warn"
            title="Dictation is paused. Resume from the tray menu to use the hotkey again."
          >
            <Icon name="pause" size={10} strokeWidth={2.5} />
            Paused
          </span>
        )}
      </div>

      {showNav ? <Nav /> : <span data-tauri-drag-region />}

      <div data-tauri-drag-region className="flex items-center justify-end gap-0.5">
        <WindowButton label="Minimize" icon="minimize" onClick={() => void win().minimize()} />
        <WindowButton label="Maximize" icon="maximize" onClick={() => void win().toggleMaximize()} />
        <WindowButton label="Close" icon="close" danger onClick={() => void win().close()} />
      </div>
    </header>
  );
}
