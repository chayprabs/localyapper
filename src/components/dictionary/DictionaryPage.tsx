import { useState, useCallback } from "react";
import { exportDictionary } from "@/lib/commands/corrections";
import { CorrectionsTab } from "./CorrectionsTab";
import { TrainingTab } from "./TrainingTab";

type TabId = "corrections" | "training";

function TabButton({
  active,
  onClick,
  children,
}: {
  active: boolean;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      onClick={onClick}
      className={`text-[13px] pb-0 relative ${
        active
          ? "font-semibold text-[#1C1C1E] after:absolute after:bottom-[-13px] after:left-0 after:right-0 after:h-[2px] after:bg-[#0058bc]"
          : "font-normal text-black/[0.26] hover:text-black/85 transition-colors"
      }`}
    >
      {children}
    </button>
  );
}

export function DictionaryPage() {
  const [activeTab, setActiveTab] = useState<TabId>("corrections");
  const [showAddForm, setShowAddForm] = useState(false);
  const [exportLabel, setExportLabel] = useState("Export JSON");

  const handleExport = useCallback(async () => {
    try {
      const json = await exportDictionary();
      await navigator.clipboard.writeText(json);
      setExportLabel("Copied!");
      setTimeout(() => setExportLabel("Export JSON"), 2000);
    } catch {
      // Silently fail if clipboard is unavailable
    }
  }, []);

  const handleTrainingDone = useCallback(() => {
    setActiveTab("corrections");
  }, []);

  return (
    <div className="flex flex-col h-full px-12 py-10">
      {/* Header */}
      <header className="flex items-center justify-between mb-8 shrink-0">
        <h1 className="text-[26px] font-semibold text-[#1C1C1E]">
          Dictionary
        </h1>
        <div className="flex items-center gap-3">
          <button
            onClick={() => void handleExport()}
            className="bg-white border border-black/[0.07] px-4 py-1.5 rounded-lg text-[13px] font-medium shadow-sm hover:bg-gray-50 transition-all flex items-center gap-2"
          >
            <span className="material-symbols-outlined text-[18px]">
              download
            </span>
            {exportLabel}
          </button>
          <button
            onClick={() => setShowAddForm(true)}
            className="bg-[#0058bc] text-white px-4 py-1.5 rounded-lg text-[13px] font-medium shadow-sm hover:brightness-110 transition-all flex items-center gap-2"
          >
            <span className="material-symbols-outlined text-[18px]">add</span>
            Add Correction
          </button>
        </div>
      </header>

      {/* Tabs */}
      <div className="flex items-center gap-8 mb-6 border-b border-black/[0.07] pb-3 shrink-0">
        <TabButton
          active={activeTab === "corrections"}
          onClick={() => setActiveTab("corrections")}
        >
          Corrections
        </TabButton>
        <TabButton
          active={activeTab === "training"}
          onClick={() => setActiveTab("training")}
        >
          Training
        </TabButton>
      </div>

      {/* Tab Content */}
      <div className="flex-1 overflow-y-auto">
        {activeTab === "corrections" ? (
          <CorrectionsTab
            showAddForm={showAddForm}
            onCloseAddForm={() => setShowAddForm(false)}
            onSwitchToTraining={() => setActiveTab("training")}
          />
        ) : (
          <TrainingTab onDone={handleTrainingDone} />
        )}

        <div className="mt-6 text-center">
          <a
            href="#"
            onClick={(e) => e.preventDefault()}
            className="text-[12px] text-[#0058bc] hover:underline cursor-pointer"
          >
            Learn more about voice training
          </a>
        </div>
      </div>
    </div>
  );
}
