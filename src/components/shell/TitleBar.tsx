// Title bar: drag region, page tabs and window controls.
import { useLayoutEffect, useRef, useState } from "react";
import { useAtom, useAtomValue } from "jotai";
import { getCurrentWindow } from "@tauri-apps/api/window";
import logoImg from "@/assets/logo-nobg.png";
import { activePageAtom, pausedAtom, type PageId } from "@/stores/appStore";
import { Icon, type IconName } from "@/components/ui/Icon";
import { cn } from "@/lib/utils";

const PAGES: { id: PageId; label: string }[] = [
  { id: "history", label: "History" },
  { id: "dictation", label: "Dictation" },
  { id: "voice", label: "Voice" },
  { id: "general", label: "General" },
];

/** Text tabs. A line on the bar's bottom edge slides under the active one. */
function Tabs() {
  const [active, setActive] = useAtom(activePageAtom);
  const navRef = useRef<HTMLElement>(null);
  const tabRefs = useRef(new Map<PageId, HTMLButtonElement>());
  const [line, setLine] = useState<{ left: number; width: number } | null>(null);

  useLayoutEffect(() => {
    const measure = () => {
      const el = tabRefs.current.get(active);
      if (el) setLine({ left: el.offsetLeft, width: el.offsetWidth });
    };
    measure();
    const nav = navRef.current;
    if (!nav) return;
    const observer = new ResizeObserver(measure);
    observer.observe(nav);
    return () => observer.disconnect();
  }, [active]);

  return (
    <nav ref={navRef} aria-label="Pages" className="relative flex h-full items-stretch gap-5">
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
              "text-[13px] transition-colors duration-100 focus-visible:-outline-offset-2",
              on ? "text-ink" : "text-ink/45 hover:text-ink/80",
            )}
          >
            {page.label}
          </button>
        );
      })}
      {line && (
        <span
          aria-hidden="true"
          className="absolute -bottom-px h-px bg-ink transition-[left,width] duration-200 ease-out"
          style={{ left: line.left, width: line.width }}
        />
      )}
    </nav>
  );
}

function WindowButton({ label, icon, onClick, close }: { label: string; icon: IconName; onClick: () => void; close?: boolean }) {
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      onClick={onClick}
      className={cn(
        "flex h-full w-10 items-center justify-center text-ink/40 transition-colors duration-100 focus-visible:-outline-offset-2",
        close ? "hover:bg-signal hover:text-canvas" : "hover:bg-ink/[0.07] hover:text-ink",
      )}
    >
      <Icon name={icon} size={icon === "maximize" ? 10 : 13} strokeWidth={1.75} />
    </button>
  );
}

export function TitleBar({ showTabs }: { showTabs: boolean }) {
  const paused = useAtomValue(pausedAtom);
  const win = () => getCurrentWindow();

  return (
    <header
      data-tauri-drag-region
      className="relative z-10 flex h-10 shrink-0 select-none items-stretch border-b border-ink/15"
    >
      <div data-tauri-drag-region className="flex items-center gap-5 pl-4">
        <img src={logoImg} alt="LocalYapper" className="logo pointer-events-none h-3.5 w-3.5 object-contain" draggable={false} />
        {showTabs && <Tabs />}
      </div>
      <div data-tauri-drag-region className="flex min-w-0 flex-1 items-center justify-end pr-3">
        {paused === true && (
          <span
            data-tauri-drag-region
            className="animate-fade-in text-[12px] text-ink/45"
            title="Resume from the tray menu to use the hotkey again."
          >
            Paused
          </span>
        )}
      </div>
      <div className="flex items-stretch">
        <WindowButton label="Minimize" icon="minimize" onClick={() => void win().minimize()} />
        <WindowButton label="Maximize" icon="maximize" onClick={() => void win().toggleMaximize()} />
        <WindowButton label="Close" icon="close" close onClick={() => void win().close()} />
      </div>
    </header>
  );
}
