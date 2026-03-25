// Navigation sidebar -- 5 pages with Material Symbols icons
import { useAtomValue, useSetAtom } from "jotai";
import { activePageAtom, sidebarCollapsedAtom, type PageId } from "@/stores/appStore";
import { cn } from "@/lib/utils";

interface NavItem {
  id: PageId;
  label: string;
  icon: string;
}

// Sidebar navigation entries -- order matches design spec
const navItems: NavItem[] = [
  { id: "dashboard", label: "Dashboard", icon: "dashboard" },
  { id: "history", label: "History", icon: "history" },
  { id: "dictionary", label: "Dictionary", icon: "book_2" },
  { id: "hotkeys", label: "Hotkeys", icon: "keyboard" },
  { id: "models", label: "Models", icon: "neurology" },
];

export function Sidebar() {
  const activePage = useAtomValue(activePageAtom);
  const setActivePage = useSetAtom(activePageAtom);
  const isCollapsed = useAtomValue(sidebarCollapsedAtom);

  return (
    <aside
      className={cn(
        "bg-[#eeeeee] flex flex-col h-full shrink-0 transition-[width] duration-200 overflow-hidden",
        isCollapsed ? "w-12" : "w-[220px]"
      )}
    >
      {/* Nav items */}
      <nav
        className={cn(
          "flex flex-col gap-1 pt-4 flex-1",
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

      {/* Bottom spacer for the toggle button positioned in SettingsLayout */}
      <div className="h-14 shrink-0" />
    </aside>
  );
}
