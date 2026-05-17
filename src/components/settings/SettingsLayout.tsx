// Main settings layout -- sidebar navigation with page content area
import { Suspense, lazy, useEffect } from "react";
import { useAtom, useAtomValue } from "jotai";
import { activePageAtom, sidebarCollapsedAtom } from "@/stores/appStore";
import { getSetting, setSetting } from "@/lib/commands/settings";
import { Sidebar } from "./Sidebar";

// Settings pages are lazy-loaded so the main bundle stays small.
// Each page imports its own data hooks and command wrappers; pulling them
// only when the user navigates means the dashboard's initial paint is
// faster and unused pages never touch the parse/compile budget.
const DashboardPage = lazy(() =>
  import("@/components/dashboard/DashboardPage").then((m) => ({ default: m.DashboardPage })),
);
const HistoryPage = lazy(() =>
  import("@/components/history/HistoryPage").then((m) => ({ default: m.HistoryPage })),
);
const HotkeysPage = lazy(() =>
  import("@/components/hotkeys/HotkeysPage").then((m) => ({ default: m.HotkeysPage })),
);
const ModelsPage = lazy(() =>
  import("@/components/models/ModelsPage").then((m) => ({ default: m.ModelsPage })),
);

/** Page ID → React component lookup table for content area rendering. */
const pages = {
  dashboard: DashboardPage,
  history: HistoryPage,
  hotkeys: HotkeysPage,
  models: ModelsPage,
} as const;

function PageFallback() {
  return (
    <div className="h-full w-full flex items-center justify-center">
      <span className="material-symbols-outlined text-[24px] text-black/[0.30] animate-spin">
        progress_activity
      </span>
    </div>
  );
}

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
          <Suspense fallback={<PageFallback />}>
            <PageComponent />
          </Suspense>
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
