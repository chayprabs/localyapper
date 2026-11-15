// The page area of the main window. Pages are lazy-loaded so each one's
// hooks and command wrappers are parsed only when it is opened.
import { Suspense, lazy } from "react";
import { useAtomValue } from "jotai";
import { activePageAtom } from "@/stores/appStore";
import { Spinner } from "@/components/settings/controls";

const pages = {
  history: lazy(() => import("@/components/history/HistoryPage").then((m) => ({ default: m.HistoryPage }))),
  dictation: lazy(() => import("@/components/dictation/DictationPage").then((m) => ({ default: m.DictationPage }))),
  voice: lazy(() => import("@/components/voice/VoicePage").then((m) => ({ default: m.VoicePage }))),
  general: lazy(() => import("@/components/general/GeneralPage").then((m) => ({ default: m.GeneralPage }))),
} as const;

export function Loading() {
  return (
    <div className="flex h-full w-full items-center justify-center">
      <Spinner />
    </div>
  );
}

export function Pages() {
  const activePage = useAtomValue(activePageAtom);
  const PageComponent = pages[activePage];

  return (
    // Keyed by page so the scroll position resets.
    <main key={activePage} className="h-full overflow-y-auto overflow-x-hidden">
      <Suspense fallback={<Loading />}>
        <PageComponent />
      </Suspense>
    </main>
  );
}
