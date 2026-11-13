// Main settings layout -- sidebar navigation with page content area
import { Suspense, lazy, useEffect } from "react";
import { useAtom, useAtomValue } from "jotai";
import { activePageAtom, sidebarCollapsedAtom } from "@/stores/appStore";
import { getSetting, setSetting } from "@/lib/commands/settings";
import { Sidebar } from "./Sidebar";
import { Icon } from "@/components/ui/Icon";

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
const DictationPage = lazy(() =>
  import("@/components/dictation/DictationPage").then((m) => ({ default: m.DictationPage })),
);
const AudioPage = lazy(() =>
  import("@/components/audio/AudioPage").then((m) => ({ default: m.AudioPage })),
);
const SpeechPage = lazy(() =>
  import("@/components/speech/SpeechPage").then((m) => ({ default: m.SpeechPage })),
);
const GeneralPage = lazy(() =>
  import("@/components/general/GeneralPage").then((m) => ({ default: m.GeneralPage })),
);

/** Page ID → React component lookup table for content area rendering. */
const pages = {
  dashboard: DashboardPage,
  history: HistoryPage,
  dictation: DictationPage,
  audio: AudioPage,
  speech: SpeechPage,
  general: GeneralPage,
} as const;

function PageFallback() {
  return (
    <div className="h-full w-full flex items-center justify-center">
      <Icon name="progress_activity" size={24} className="text-black/[0.30] animate-spin" />
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
    <div className="flex h-full bg-sidebar relative">
      <Sidebar />
      <main className="flex-1 bg-sidebar p-3">
        <div className="bg-surface rounded-2xl h-full overflow-y-auto overflow-x-hidden">
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
        <Icon
          name={
            isCollapsed
              ? "keyboard_double_arrow_right"
              : "keyboard_double_arrow_left"
          }
          size={18}
        />
      </button>
    </div>
  );
}
