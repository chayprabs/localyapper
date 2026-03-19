import { useAtom } from "jotai";
import { activePageAtom, type PageId } from "@/stores/appStore";
import { cn } from "@/lib/utils";

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

  return (
    <aside className="w-[240px] bg-[#eeeeee] flex flex-col h-screen shrink-0">
      <div className="px-5 py-6 flex flex-col gap-3">
        <div className="flex items-center pt-5 pb-4 px-1">
          <span className="text-[18px] font-semibold text-black/85">
            LocalYapper
          </span>
        </div>
        <nav className="flex flex-col gap-1">
          {navItems.map((item) => {
            const isActive = activePage === item.id;
            return (
              <button
                key={item.id}
                onClick={() => setActivePage(item.id)}
                className={cn(
                  "flex items-center gap-3 px-2.5 h-11 rounded-md text-[15px] transition-colors w-full text-left",
                  isActive
                    ? "bg-[rgba(0,122,255,0.12)] text-[#0058bc] font-medium"
                    : "text-black/55 font-normal hover:bg-black/5"
                )}
              >
                <span
                  className="material-symbols-outlined text-[20px]"
                  style={
                    isActive
                      ? { fontVariationSettings: "'FILL' 1" }
                      : undefined
                  }
                >
                  {item.icon}
                </span>
                <span>{item.label}</span>
              </button>
            );
          })}
        </nav>
      </div>
    </aside>
  );
}
