import { useEffect } from "react";
import { useAtom } from "jotai";
import { activePageAtom, sidebarCollapsedAtom, type PageId } from "@/stores/appStore";
import { cn } from "@/lib/utils";
import { getSetting, setSetting } from "@/lib/commands/settings";

interface NavItem {
  id: PageId;
  label: string;
  icon: string;
}

const navItems: NavItem[] = [
  { id: "dashboard", label: "Dashboard", icon: "dashboard" },
  { id: "history", label: "History", icon: "history" },
  { id: "dictionary", label: "Dictionary", icon: "book_2" },
  { id: "hotkeys", label: "Hotkeys", icon: "keyboard" },
  { id: "models", label: "Models", icon: "neurology" },
];

export function Sidebar() {
  const [activePage, setActivePage] = useAtom(activePageAtom);
  const [isCollapsed, setIsCollapsed] = useAtom(sidebarCollapsedAtom);

  useEffect(() => {
    getSetting("sidebar_collapsed")
      .then((val) => setIsCollapsed(val === "true"))
      .catch(() => {});
  }, [setIsCollapsed]);

  const toggle = () => {
    const next = !isCollapsed;
    setIsCollapsed(next);
    setSetting("sidebar_collapsed", next ? "true" : "false").catch(() => {});
  };

  return (
    <aside
      className={cn(
        "bg-[#eeeeee] flex flex-col h-full shrink-0 transition-[width] duration-200 overflow-hidden",
        isCollapsed ? "w-12" : "w-[240px]"
      )}
    >
      {/* Collapse toggle button */}
      <div
        className={cn(
          "flex pt-2 pb-1",
          isCollapsed ? "justify-center" : "justify-end px-3"
        )}
      >
        <button
          onClick={toggle}
          className="w-8 h-8 flex items-center justify-center text-black/35 hover:bg-black/[0.08] rounded-md transition-colors"
          title={isCollapsed ? "Expand sidebar" : "Collapse sidebar"}
        >
          <span className="material-symbols-outlined text-[18px]">
            view_sidebar
          </span>
        </button>
      </div>

      {/* Nav items */}
      <nav
        className={cn(
          "flex flex-col gap-1",
          isCollapsed ? "px-1.5" : "px-3"
        )}
      >
        {navItems.map((item) => {
          const isActive = activePage === item.id;
          return (
            <button
              key={item.id}
              onClick={() => setActivePage(item.id)}
              title={isCollapsed ? item.label : undefined}
              className={cn(
                "flex items-center h-11 rounded-md text-[15px] transition-colors w-full",
                isCollapsed ? "justify-center" : "gap-3 px-2.5 text-left",
                isActive
                  ? "bg-[rgba(0,122,255,0.12)] text-[#0058bc] font-medium"
                  : "text-black/55 font-normal hover:bg-black/5"
              )}
            >
              <span
                className="material-symbols-outlined text-[20px] shrink-0"
                style={
                  isActive
                    ? { fontVariationSettings: "'FILL' 1" }
                    : undefined
                }
              >
                {item.icon}
              </span>
              {!isCollapsed && <span>{item.label}</span>}
            </button>
          );
        })}
      </nav>
    </aside>
  );
}
