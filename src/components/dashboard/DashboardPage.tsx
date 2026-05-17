// Dashboard page -- stats, last dictation, model status, and onboarding nudges
import { useSetAtom } from "jotai";
import { useDashboard } from "@/hooks/useDashboard";
import { activePageAtom } from "@/stores/appStore";
import { formatNumber } from "@/lib/formatters";
import { StatCard } from "./StatCard";
import { ModelStatusCard } from "./ModelStatusCard";
import { LastDictationCard } from "./LastDictationCard";

interface SpeechFilesMissingBannerProps {
  onInstall: () => void;
}

function SpeechFilesMissingBanner({ onInstall }: SpeechFilesMissingBannerProps) {
  return (
    <div className="mt-3 flex items-center justify-between gap-4 rounded-[10px] border border-[#ff9500]/30 bg-[#ff9500]/[0.08] px-4 py-3">
      <div className="flex items-center gap-3">
        <span className="material-symbols-outlined text-[20px] text-[#9a5a00]">
          download
        </span>
        <div>
          <p className="text-[13px] font-semibold text-[#7a4500]">
            Speech files aren't installed
          </p>
          <p className="text-[12px] text-[#9a5a00]">
            Dictation needs the local Parakeet model. About 458 MB, downloaded
            once.
          </p>
        </div>
      </div>
      <button
        type="button"
        onClick={onInstall}
        className="h-8 rounded-[8px] bg-[#0058bc] px-3 text-[12px] font-medium text-white shadow-sm transition-colors hover:bg-[#004ea8]"
      >
        Install now
      </button>
    </div>
  );
}

interface DashboardEmptyStateProps {
  recordHotkey: string;
}

function DashboardEmptyState({ recordHotkey }: DashboardEmptyStateProps) {
  return (
    <section className="flex min-h-[260px] flex-col items-center justify-center rounded-xl border border-black/[0.07] bg-white p-10 text-center shadow-sm">
      <div className="mb-5 flex h-14 w-14 items-center justify-center rounded-full bg-[#0058bc]/[0.10]">
        <span className="material-symbols-outlined text-[28px] text-[#0058bc]">
          mic
        </span>
      </div>
      <p className="mb-1 text-[15px] font-semibold text-black/85">
        You haven't dictated anything yet
      </p>
      <p className="max-w-[360px] text-[13px] leading-relaxed text-black/[0.50]">
        Hold{" "}
        <span className="rounded-[4px] border border-black/[0.10] bg-white px-1.5 py-0.5 font-mono text-[12px] font-semibold text-black/85 shadow-[0_1px_0_rgba(0,0,0,0.04)]">
          {recordHotkey}
        </span>{" "}
        anywhere on this device to record. Your stats, recent dictations, and
        history will show up here as you use LocalYapper.
      </p>
    </section>
  );
}

export function DashboardPage() {
  const {
    stats,
    lastDictation,
    modelStatus,
    modelFileStatus,
    recordHotkey,
    error,
    deleteLastDictation,
  } = useDashboard();
  const setActivePage = useSetAtom(activePageAtom);

  const isEmpty = !stats || stats.total_sessions === 0;
  const filesMissing =
    modelFileStatus !== null && modelFileStatus.exists === false;

  const handleDelete = (id: string) => {
    void deleteLastDictation(id);
  };

  return (
    <div className="px-8 py-6">
      <header className="mb-5">
        <h1 className="text-[24px] font-bold text-[#1C1C1E]">Dashboard</h1>
        <p className="mt-1 text-[12px] text-black/[0.45]">
          All processing happens on this device. No audio leaves your machine.
        </p>
        {error && (
          <p className="mt-3 max-w-[520px] rounded-lg border border-[#ba1a1a]/15 bg-[#ba1a1a]/[0.06] px-3 py-2 text-[12px] font-medium text-[#ba1a1a]">
            {error}
          </p>
        )}
        {filesMissing && (
          <SpeechFilesMissingBanner
            onInstall={() => setActivePage("models")}
          />
        )}
      </header>

      {isEmpty ? (
        <>
          <DashboardEmptyState recordHotkey={recordHotkey} />
          <section className="mt-4">
            <ModelStatusCard
              status={modelStatus}
              fileStatus={modelFileStatus}
            />
          </section>
        </>
      ) : (
        <>
          <section className="mb-4 grid grid-cols-4 gap-3">
            <StatCard label="WORDS TODAY" value={formatNumber(stats.words_today)} />
            <StatCard
              label="WORDS THIS WEEK"
              value={formatNumber(stats.words_week)}
            />
            <StatCard
              label="WORDS ALL TIME"
              value={formatNumber(stats.words_all_time)}
            />
            <StatCard label="AVG WPM" value={formatNumber(stats.avg_wpm)} />
          </section>

          <section className="mb-4 grid grid-cols-2 gap-3">
            <StatCard
              label="TOTAL SESSIONS"
              value={formatNumber(stats.total_sessions)}
            />
            <ModelStatusCard
              status={modelStatus}
              fileStatus={modelFileStatus}
            />
          </section>

          <LastDictationCard
            entry={lastDictation}
            onDelete={handleDelete}
          />
        </>
      )}
    </div>
  );
}
