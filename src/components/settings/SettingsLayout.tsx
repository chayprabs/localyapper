import { useEffect } from "react";
import { useAtom, useAtomValue } from "jotai";
import { activePageAtom, sidebarCollapsedAtom } from "@/stores/appStore";
import { getSetting, setSetting } from "@/lib/commands/settings";
import { Sidebar } from "./Sidebar";
import { DashboardPage } from "@/components/dashboard/DashboardPage";
import { HistoryPage } from "@/components/history/HistoryPage";
import { DictionaryPage } from "@/components/dictionary/DictionaryPage";
import { HotkeysPage } from "@/components/hotkeys/HotkeysPage";
import { ModelsPage } from "@/components/models/ModelsPage";

const pages = {
  dashboard: DashboardPage,
  history: HistoryPage,
  dictionary: DictionaryPage,
  hotkeys: HotkeysPage,
  models: ModelsPage,
} as const;

export function SettingsLayout() {
  const activePage = useAtomValue(activePageAtom);
  const [isCollapsed, setIsCollapsed] = useAtom(sidebarCollapsedAtom);
  const PageComponent = pages[activePage];

  useEffect(() => {
    getSetting("sidebar_collapsed")
      .then((val) => setIsCollapsed(val === "true"))
      .catch(() => {});
  }, [setIsCollapsed]);

  const toggleSidebar = () => {
    const next = !isCollapsed;
    setIsCollapsed(next);
    setSetting("sidebar_collapsed", next ? "true" : "false").catch(() => {});
  };

  return (
    <div className="flex h-full bg-[#eeeeee] relative">
      <Sidebar />
      <main className="flex-1 bg-[#eeeeee] p-3">
        <div className="bg-white rounded-2xl h-full overflow-y-auto overflow-x-hidden">
          <PageComponent />
        </div>
      </main>
      {/* Toggle button — fixed position at bottom-left */}
      <button
        onClick={toggleSidebar}
        className="absolute bottom-3 left-[8px] w-8 h-8 flex items-center justify-center text-black/35 hover:bg-black/[0.08] rounded-md transition-colors z-10"
        title={isCollapsed ? "Expand sidebar" : "Collapse sidebar"}
      >
        <span className="material-symbols-outlined text-[18px]">
          {isCollapsed
            ? "keyboard_double_arrow_right"
            : "keyboard_double_arrow_left"}
        </span>
      </button>
    </div>
  );
}
