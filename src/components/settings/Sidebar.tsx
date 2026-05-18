// Navigation sidebar -- main app pages with inline SVG icons
import { useAtomValue, useSetAtom } from "jotai";
import { activePageAtom, sidebarCollapsedAtom, type PageId } from "@/stores/appStore";
import { cn } from "@/lib/utils";
import { Icon, type IconName } from "@/components/ui/Icon";

interface NavItem {
  id: PageId;
  label: string;
  icon: IconName;
}

/** Sidebar navigation entries — order defines visual arrangement. */
const navItems: NavItem[] = [
  { id: "dashboard", label: "Dashboard", icon: "dashboard" },
  { id: "history", label: "History", icon: "history" },
  { id: "hotkeys", label: "Hotkeys", icon: "keyboard" },
  { id: "models", label: "Speech", icon: "graphic_eq" },
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
              <Icon
                name={item.icon}
                size={20}
                strokeWidth={isActive ? 2.25 : 1.75}
                className="shrink-0"
              />
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
