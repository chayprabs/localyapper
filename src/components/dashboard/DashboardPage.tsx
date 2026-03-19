import { useDashboard } from "@/hooks/useDashboard";
import { formatNumber } from "@/lib/formatters";
import { StatCard } from "./StatCard";
import { ModelStatusCard } from "./ModelStatusCard";
import { LastDictationCard } from "./LastDictationCard";

export function DashboardPage() {
  const { stats, lastDictation, modelStatus, isLoading, deleteLastDictation } = useDashboard();

  const isEmpty = !stats || stats.total_sessions === 0;

  const handleCopy = (text: string) => {
    void navigator.clipboard.writeText(text);
  };

  const handleDelete = (id: string) => {
    void deleteLastDictation(id);
  };

  return (
    <div className="px-12 py-10">
      {/* Header */}
      <header className="mb-10">
        <h1 className="text-[26px] font-bold text-[#1C1C1E]">Dashboard</h1>
      </header>

      {/* Stat Cards Row */}
      <section className="grid grid-cols-4 gap-4 mb-6">
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
      <section className="grid grid-cols-2 gap-4 mb-6">
        <StatCard
          label="TOTAL SESSIONS"
          value={isEmpty ? null : formatNumber(stats.total_sessions)}
        />
        <ModelStatusCard status={modelStatus} isLoading={isLoading} />
      </section>

      {/* Last Dictation */}
      <LastDictationCard
        entry={isEmpty ? null : lastDictation}
        onCopy={handleCopy}
        onDelete={handleDelete}
      />
    </div>
  );
}
