// History page -- the main page: hotkey and status on top, then the
// searchable list of past dictations grouped by day
import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { useAtomValue, useSetAtom } from "jotai";
import { useHistory } from "@/hooks/useHistory";
import { useStatus } from "@/hooks/useStatus";
import { activePageAtom, pausedAtom } from "@/stores/appStore";
import { exportHistory } from "@/lib/commands/history";
import { formatLocalDay, formatLocalTime, formatNumber } from "@/lib/formatters";
import type { HistoryEntry } from "@/types/commands";
import { Button, IconButton } from "@/components/ui/Button";
import { CopyButton } from "@/components/ui/CopyButton";
import { Icon } from "@/components/ui/Icon";
import { Keys } from "@/components/ui/Keys";
import { errorText, Notice, Page, Spinner } from "@/components/settings/controls";
import { cn } from "@/lib/utils";

/** Consecutive entries grouped by their local calendar day. */
function groupByDay(entries: HistoryEntry[]): { day: string; items: HistoryEntry[] }[] {
  const groups: { day: string; items: HistoryEntry[] }[] = [];
  for (const entry of entries) {
    const day = formatLocalDay(entry.created_at);
    const last = groups[groups.length - 1];
    if (last && last.day === day) {
      last.items.push(entry);
    } else {
      groups.push({ day, items: [entry] });
    }
  }
  return groups;
}

/** One dictation. Click the text to show all of it; delete asks once more in place. */
function Entry({ entry, onDelete }: { entry: HistoryEntry; onDelete: (id: string) => void }) {
  const [open, setOpen] = useState(false);
  const [confirming, setConfirming] = useState(false);
  const armedAt = useRef(0);

  // The delete confirmation withdraws itself after a few seconds.
  useEffect(() => {
    if (!confirming) return;
    const t = window.setTimeout(() => setConfirming(false), 3000);
    return () => window.clearTimeout(t);
  }, [confirming]);

  return (
    <li className="group flex gap-4 py-2.5">
      <time className="w-[62px] shrink-0 font-mono text-[11.5px] leading-5 text-ink/40">
        {formatLocalTime(entry.created_at)}
      </time>
      <p
        onClick={() => {
          // Dragging to select text ends in a click too; leave the text as it is.
          if (window.getSelection()?.isCollapsed !== false) setOpen((v) => !v);
        }}
        className={cn("selectable min-w-0 flex-1 text-[13px] leading-5 text-ink/90", !open && "line-clamp-2")}
      >
        {entry.final_text}
      </p>
      {/* The app name rests on the right; the actions take its place on hover. */}
      <div className="relative flex h-5 min-w-[56px] shrink-0 items-center justify-end">
        {entry.app_name && (
          <span
            className={cn(
              "max-w-[110px] truncate text-[12px] text-ink/35 group-focus-within:opacity-0 group-hover:opacity-0",
              confirming && "opacity-0",
            )}
          >
            {entry.app_name}
          </span>
        )}
        <div
          className={cn(
            "absolute right-0 flex items-center",
            !confirming && "opacity-0 focus-within:opacity-100 group-hover:opacity-100",
          )}
        >
          {confirming ? (
            <Button
              variant="danger"
              autoFocus
              className="h-6 animate-fade-in px-2"
              onClick={() => {
                // A double-click on the bin must not also confirm.
                if (Date.now() - armedAt.current > 400) onDelete(entry.id);
              }}
            >
              Delete
            </Button>
          ) : (
            <>
              <CopyButton text={entry.final_text} />
              <IconButton
                label="Delete"
                tone="danger"
                className="-mr-1.5"
                onClick={() => {
                  armedAt.current = Date.now();
                  setConfirming(true);
                }}
              >
                <Icon name="delete" size={14} />
              </IconButton>
            </>
          )}
        </div>
      </div>
    </li>
  );
}

function Figure({ value, unit }: { value: number; unit: string }) {
  return (
    <span>
      <span className="text-ink/90">{formatNumber(value)}</span> {unit}
    </span>
  );
}

export function HistoryPage() {
  const [query, setQuery] = useState("");
  const [search, setSearch] = useState("");
  const { entries, isLoading, hasMore, error, loadMore, deleteEntry, clearAll } = useHistory(search);
  const app = useStatus();
  const paused = useAtomValue(pausedAtom) === true;
  const setActivePage = useSetAtom(activePageAtom);
  const [confirmClear, setConfirmClear] = useState(false);
  const [exportNote, setExportNote] = useState<{ ok: boolean; text: string } | null>(null);
  const groups = useMemo(() => groupByDay(entries), [entries]);

  // Search as the user types, without a query per keystroke.
  useEffect(() => {
    const t = window.setTimeout(() => setSearch(query.trim()), 250);
    return () => window.clearTimeout(t);
  }, [query]);

  const isEmpty = !isLoading && entries.length === 0;
  const noHistory = isEmpty && search === "";

  useEffect(() => {
    if (isEmpty) setConfirmClear(false);
  }, [isEmpty]);

  if (isLoading || app.isLoading) {
    return (
      <div className="flex h-full items-center justify-center">
        <Spinner />
      </div>
    );
  }

  const handleExport = () => {
    setExportNote(null);
    exportHistory()
      .then((path) => setExportNote(path ? { ok: true, text: `Saved to ${path}` } : null))
      .catch((err: unknown) => setExportNote({ ok: false, text: errorText(err) }));
  };

  const noModel = app.modelFileStatus !== null && !app.modelFileStatus.exists;
  const status: ReactNode = noModel ? (
    <>
      No speech model
      <Button onClick={() => setActivePage("voice")}>Download</Button>
    </>
  ) : paused ? (
    "Paused"
  ) : (
    "Hold to dictate"
  );
  const notices = (
    <>
      {app.error && <Notice>{app.error}</Notice>}
      {error && <Notice>{error}</Notice>}
      {exportNote && <Notice tone={exportNote.ok ? "note" : "error"}>{exportNote.text}</Notice>}
    </>
  );

  // Nothing dictated yet: the key, and what to do with it.
  if (noHistory) {
    return (
      <div className="flex h-full animate-fade-in flex-col items-center justify-center gap-5 px-6 pb-10">
        <Keys hotkey={app.recordHotkey} size="xl" />
        <div className="flex items-center gap-3 text-[13px] text-ink/55">{status}</div>
        {notices}
      </div>
    );
  }

  const stats = app.stats;
  return (
    <Page className="gap-0">
      <div className="flex min-h-8 flex-wrap items-center justify-between gap-x-6 gap-y-2">
        <div className="flex items-center gap-3 text-[13px] text-ink/55">
          <Keys hotkey={app.recordHotkey} size="lg" />
          {status}
        </div>
        {/* The download prompt takes the counts' place until a model is installed. */}
        {stats && !noModel && (
          <div className="flex gap-4 font-mono text-[11.5px] text-ink/40">
            <Figure value={stats.words_today} unit="today" />
            <Figure value={stats.words_week} unit="week" />
            <Figure value={stats.words_all_time} unit="total" />
            <Figure value={Math.round(stats.avg_wpm)} unit="wpm" />
          </div>
        )}
      </div>

      <div className="mt-7 flex h-9 items-center gap-4 border-b border-ink/15">
        <label className="flex min-w-0 flex-1 cursor-text items-center gap-2 text-ink/35 focus-within:text-ink/70">
          <Icon name="search" size={13} className="shrink-0" />
          <input
            type="search"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search"
            aria-label="Search dictations"
            className="h-8 min-w-0 flex-1 bg-transparent text-[13px] text-ink/90 outline-none placeholder:text-ink/35 [&::-webkit-search-cancel-button]:hidden"
          />
        </label>
        {confirmClear ? (
          <span className="flex animate-fade-in items-center gap-3">
            <Button variant="ghost" onClick={() => setConfirmClear(false)}>
              Cancel
            </Button>
            <Button
              variant="danger"
              className="h-6 px-2"
              onClick={() => {
                setConfirmClear(false);
                void clearAll().then(app.refresh);
              }}
            >
              Delete all
            </Button>
          </span>
        ) : (
          <span className="flex items-center gap-3">
            <Button variant="ghost" onClick={handleExport}>
              Export
            </Button>
            <Button variant="ghost" onClick={() => setConfirmClear(true)}>
              Clear
            </Button>
          </span>
        )}
      </div>

      <div className="mt-2 empty:hidden">{notices}</div>

      {isEmpty ? (
        <p className="mt-6 text-[13px] text-ink/45">No matches.</p>
      ) : (
        <>
          {groups.map((group) => (
            <section key={group.day} className="mt-6">
              <h2 className="pb-1 text-[12px] text-ink/45">{group.day}</h2>
              <ul className="divide-y divide-ink/[0.08]">
                {group.items.map((entry) => (
                  <Entry key={entry.id} entry={entry} onDelete={(id) => void deleteEntry(id).then(app.refresh)} />
                ))}
              </ul>
            </section>
          ))}
          {hasMore && (
            <div className="mt-6">
              <Button onClick={loadMore}>More</Button>
            </div>
          )}
        </>
      )}
    </Page>
  );
}
