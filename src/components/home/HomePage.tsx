// Home page -- what to do next, a few numbers and the latest dictation
import type { ReactNode } from "react";
import { useAtomValue, useSetAtom } from "jotai";
import { useHome } from "@/hooks/useHome";
import { useCountUp } from "@/hooks/useCountUp";
import { activePageAtom, pausedAtom } from "@/stores/appStore";
import { formatNumber, formatRelativeTime } from "@/lib/formatters";
import type { HistoryEntry } from "@/types/commands";
import { Button, IconButton } from "@/components/ui/Button";
import { CopyButton } from "@/components/ui/CopyButton";
import { Icon, type IconName } from "@/components/ui/Icon";
import { Keys } from "@/components/ui/Keys";
import { Chip, Notice, Page, Panel, Spinner } from "@/components/settings/controls";
import { cn } from "@/lib/utils";

function modelLabel(name: string | undefined): string {
  switch (name) {
    case "parakeet-110m":
      return "Parakeet 110M";
    case "parakeet-0.6b":
      return "Parakeet 0.6B";
    default:
      return "Speech model";
  }
}

/** The glass disc at the top of the page. Rings pulse while dictation is available. */
function Orb({ icon, live }: { icon: IconName; live: boolean }) {
  return (
    <div className="relative flex h-[88px] w-[88px] items-center justify-center">
      {live && (
        <>
          <span className="absolute inset-0 animate-ring rounded-full border border-accent/50" />
          <span className="absolute inset-0 animate-ring rounded-full border border-accent/50 [animation-delay:1.6s]" />
        </>
      )}
      <div className="glass relative flex h-full w-full items-center justify-center rounded-full">
        <Icon name={icon} size={30} strokeWidth={1.5} className={live ? "text-accent" : "text-ink/40"} />
      </div>
    </div>
  );
}

function Hero({ icon, live, title, children }: { icon: IconName; live: boolean; title: ReactNode; children: ReactNode }) {
  return (
    <section className="flex flex-col items-center px-4 pb-2 pt-6 text-center">
      <Orb icon={icon} live={live} />
      <h1 className="mt-7 flex flex-wrap items-center justify-center gap-x-2.5 gap-y-2 text-[28px] font-semibold leading-tight tracking-[-0.025em] text-ink">
        {title}
      </h1>
      {children}
    </section>
  );
}

function Stat({ label, value, title }: { label: string; value: number; title?: string }) {
  const shown = useCountUp(value);
  return (
    <div className="px-5 py-4" title={title}>
      <div className="text-[26px] font-semibold leading-none tracking-[-0.02em] tabular-nums text-ink">
        {formatNumber(shown)}
      </div>
      <div className="mt-2 text-[12px] text-ink/45">{label}</div>
    </div>
  );
}

function Latest({ entry, onDelete }: { entry: HistoryEntry; onDelete: (id: string) => void }) {
  const words = entry.word_count ?? 0;
  return (
    <Panel className="p-5">
      <div className="flex items-center gap-2">
        <div className="flex min-w-0 flex-1 flex-wrap items-center gap-x-2 gap-y-1 text-[12px] text-ink/45">
          <span className="font-medium text-ink/70">Latest</span>
          <span>{formatRelativeTime(entry.created_at)}</span>
          <span>
            {formatNumber(words)} {words === 1 ? "word" : "words"}
          </span>
          {entry.app_name && <Chip>{entry.app_name}</Chip>}
        </div>
        <div className="-my-1 -mr-2 flex shrink-0">
          <CopyButton text={entry.final_text} />
          <IconButton label="Delete" tone="danger" onClick={() => onDelete(entry.id)}>
            <Icon name="delete" size={16} />
          </IconButton>
        </div>
      </div>
      <p className="selectable mt-3 line-clamp-4 text-[15px] leading-relaxed text-ink/85">{entry.final_text}</p>
    </Panel>
  );
}

export function HomePage() {
  const { stats, lastDictation, modelStatus, modelFileStatus, recordHotkey, error, deleteLastDictation, isLoading } =
    useHome();
  const setActivePage = useSetAtom(activePageAtom);
  const paused = useAtomValue(pausedAtom) === true;

  if (isLoading) {
    return (
      <div className="flex h-full items-center justify-center">
        <Spinner size={22} />
      </div>
    );
  }

  const filesMissing = modelFileStatus !== null && !modelFileStatus.exists;
  const loaded = modelStatus?.speech_model_loaded ?? false;
  const hasHistory = stats !== null && stats.total_sessions > 0;

  return (
    // With nothing to show yet, the hero sits in the middle of the window.
    <Page className={cn(!hasHistory && "min-h-full justify-center pb-16")}>
      {error && <Notice>{error}</Notice>}

      {paused ? (
        <Hero icon="pause" live={false} title="Dictation is paused">
          <p className="mt-3 max-w-[380px] text-[14px] leading-relaxed text-ink/50">
            Resume from the tray menu to use your hotkey again.
          </p>
        </Hero>
      ) : filesMissing ? (
        <Hero icon="download" live={false} title="One download to go">
          <p className="mt-3 max-w-[380px] text-[14px] leading-relaxed text-ink/50">
            Speech is recognised on this computer. Download a model once and dictate offline from then on.
          </p>
          <Button variant="primary" size="lg" className="mt-6" onClick={() => setActivePage("voice")}>
            Choose a model
          </Button>
        </Hero>
      ) : (
        <Hero
          icon="mic"
          live
          title={
            <>
              Hold <Keys hotkey={recordHotkey} size="lg" /> and speak
            </>
          }
        >
          <p className="mt-3 max-w-[380px] text-[14px] leading-relaxed text-ink/50">
            Let go and your words land in the app you are using. Nothing leaves this computer.
          </p>
          <div className="mt-5 flex items-center gap-2 text-[12px] text-ink/50">
            <span className={cn("h-1.5 w-1.5 rounded-full", loaded ? "bg-ok" : "bg-ink/25")} />
            {modelLabel(modelFileStatus?.model_name)}
            <span className="text-ink/25">/</span>
            {loaded ? "Ready" : "Loads when you start"}
          </div>
        </Hero>
      )}

      {hasHistory && stats && (
        <Panel className="grid grid-cols-2 divide-ink/[0.06] max-sm:[&>*:nth-child(-n+2)]:border-b max-sm:[&>*:nth-child(odd)]:border-r max-sm:[&>*]:border-ink/[0.06] sm:grid-cols-4 sm:divide-x">
          <Stat label="Words today" value={stats.words_today} />
          <Stat label="This week" value={stats.words_week} />
          <Stat
            label="All time"
            value={stats.words_all_time}
            title={`${formatNumber(stats.total_sessions)} dictations`}
          />
          <Stat label="Words a minute" value={Math.round(stats.avg_wpm)} />
        </Panel>
      )}

      {hasHistory && lastDictation && (
        <Latest entry={lastDictation} onDelete={(id) => void deleteLastDictation(id)} />
      )}
    </Page>
  );
}
