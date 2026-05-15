// Dashboard page -- stats cards, model status, and last dictation preview
import { useDashboard } from "@/hooks/useDashboard";
import { formatNumber } from "@/lib/formatters";
import { StatCard } from "./StatCard";
import { ModelStatusCard } from "./ModelStatusCard";
import { LastDictationCard } from "./LastDictationCard";

export function DashboardPage() {
  const { stats, lastDictation, modelStatus, error, deleteLastDictation } =
    useDashboard();

  const isEmpty = !stats || stats.total_sessions === 0;

  const handleDelete = (id: string) => {
    void deleteLastDictation(id);
  };

  return (
    <div className="px-8 py-6">
      {/* Header */}
      <header className="mb-5">
        <h1 className="text-[24px] font-bold text-[#1C1C1E]">Dashboard</h1>
        {error && (
          <p className="mt-3 max-w-[520px] rounded-lg border border-[#ba1a1a]/15 bg-[#ba1a1a]/[0.06] px-3 py-2 text-[12px] font-medium text-[#ba1a1a]">
            {error}
          </p>
        )}
      </header>

      {/* Stat Cards Row */}
      <section className="grid grid-cols-4 gap-3 mb-4">
        <StatCard
          label="WORDS TODAY"
          value={isEmpty ? null : formatNumber(stats.words_today)}
        />
        <StatCard
          label="WORDS THIS WEEK"
          value={isEmpty ? null : formatNumber(stats.words_week)}
        />
        <StatCard
          label="WORDS ALL TIME"
          value={isEmpty ? null : formatNumber(stats.words_all_time)}
        />
        <StatCard
          label="AVG WPM"
          value={isEmpty ? null : formatNumber(stats.avg_wpm)}
        />
      </section>

      {/* Sessions & Model Status Row */}
      <section className="grid grid-cols-2 gap-3 mb-4">
        <StatCard
          label="TOTAL SESSIONS"
          value={isEmpty ? null : formatNumber(stats.total_sessions)}
        />
        <ModelStatusCard status={modelStatus} />
      </section>

      {/* Last Dictation */}
      <LastDictationCard
        entry={isEmpty ? null : lastDictation}
        onDelete={handleDelete}
      />
    </div>
  );
}
