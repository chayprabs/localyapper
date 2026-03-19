import { useAtomValue } from "jotai";
import { activePageAtom } from "@/stores/appStore";
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
  const PageComponent = pages[activePage];

  return (
    <div className="flex h-screen bg-[#eeeeee]">
      <Sidebar />
      <main className="flex-1 overflow-y-auto bg-[#f9f9f9]">
        <PageComponent />
      </main>
    </div>
  );
}
