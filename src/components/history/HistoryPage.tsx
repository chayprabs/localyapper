// History page -- searchable list of past dictations, grouped by day
import { useEffect, useMemo, useState } from "react";
import { useSetAtom } from "jotai";
import { useHistory } from "@/hooks/useHistory";
import { activePageAtom } from "@/stores/appStore";
import { exportHistory } from "@/lib/commands/history";
import { formatLocalDay, formatLocalTime } from "@/lib/formatters";
import type { HistoryEntry } from "@/types/commands";
import { Button, IconButton } from "@/components/ui/Button";
import { CopyButton } from "@/components/ui/CopyButton";
import { Icon } from "@/components/ui/Icon";
import { Chip, errorText, Notice, Page, PageHeader, Panel, Spinner } from "@/components/settings/controls";
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

  // The delete confirmation withdraws itself after a few seconds.
  useEffect(() => {
    if (!confirming) return;
    const t = window.setTimeout(() => setConfirming(false), 3000);
    return () => window.clearTimeout(t);
  }, [confirming]);

  return (
    <li className="group flex gap-3 px-4 py-3">
      <time className="w-[58px] shrink-0 pt-px text-[12px] tabular-nums leading-[21px] text-ink/40">
        {formatLocalTime(entry.created_at)}
      </time>
      <div className="min-w-0 flex-1">
        <p
          onClick={() => setOpen((v) => !v)}
          className={cn("selectable text-[13.5px] leading-[21px] text-ink/85", !open && "line-clamp-2")}
        >
          {entry.final_text}
        </p>
      </div>
      {/* The app name rests on the right; the actions take its place on hover. */}
      <div className="relative flex min-w-[64px] shrink-0 items-start justify-end">
        {entry.app_name && (
          <span
            className={cn(
              "mt-px flex max-w-[110px] transition-opacity duration-150 group-focus-within:opacity-0 group-hover:opacity-0",
              confirming && "opacity-0",
            )}
          >
            <Chip>{entry.app_name}</Chip>
          </span>
        )}
      <div
        className={cn(
          "absolute -right-2 -top-1 flex transition-opacity duration-150",
          !confirming && "opacity-0 focus-within:opacity-100 group-hover:opacity-100",
        )}
      >
        {confirming ? (
          <Button variant="danger" size="sm" className="mr-2 mt-1 animate-fade-in" onClick={() => onDelete(entry.id)}>
            Delete
          </Button>
        ) : (
          <>
            <CopyButton text={entry.final_text} />
            <IconButton label="Delete dictation" tone="danger" onClick={() => setConfirming(true)}>
              <Icon name="delete" size={16} />
            </IconButton>
          </>
        )}
      </div>
      </div>
    </li>
  );
}

function Empty({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="flex flex-1 flex-col items-center justify-center pb-16 text-center">
      <div className="glass mb-5 flex h-16 w-16 items-center justify-center rounded-full">
        <Icon name="history" size={24} strokeWidth={1.5} className="text-ink/40" />
      </div>
      <p className="text-[17px] font-semibold tracking-[-0.01em] text-ink">{title}</p>
      {children}
    </div>
  );
}

export function HistoryPage() {
  const [query, setQuery] = useState("");
  const [search, setSearch] = useState("");
  const { entries, isLoading, hasMore, error, loadMore, deleteEntry, clearAll } = useHistory(search);
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

  const handleExport = () => {
    setExportNote(null);
    exportHistory()
      .then((path) => setExportNote(path ? { ok: true, text: `Saved to ${path}` } : null))
      .catch((err: unknown) => setExportNote({ ok: false, text: errorText(err) }));
  };

  return (
    <Page className="min-h-full">
      <PageHeader title="History">
        {confirmClear ? (
          <div className="flex animate-fade-in items-center gap-2">
            <span className="text-[13px] text-ink/55">Delete all history?</span>
            <Button onClick={() => setConfirmClear(false)}>Cancel</Button>
            <Button
              variant="danger"
              onClick={() => {
                setConfirmClear(false);
                void clearAll();
              }}
            >
              Delete all
            </Button>
          </div>
        ) : (
          !noHistory && (
            <div className="flex items-center gap-2">
              <label className="glass flex h-8 w-[200px] cursor-text items-center gap-2 rounded-full px-3 text-ink/40 transition-[width] duration-300 ease-out focus-within:w-[260px] focus-within:text-ink/70">
                <Icon name="search" size={14} className="shrink-0" />
                <input
                  type="search"
                  value={query}
                  onChange={(e) => setQuery(e.target.value)}
                  placeholder="Search"
                  aria-label="Search dictations"
                  className="min-w-0 flex-1 bg-transparent text-[13px] text-ink/90 outline-none placeholder:text-ink/35 [&::-webkit-search-cancel-button]:hidden"
                />
              </label>
              <IconButton label="Export as text" onClick={handleExport}>
                <Icon name="export" size={16} />
              </IconButton>
              <IconButton label="Delete all history" tone="danger" onClick={() => setConfirmClear(true)}>
                <Icon name="delete" size={16} />
              </IconButton>
            </div>
          )
        )}
      </PageHeader>

      {error && <Notice>{error}</Notice>}
      {exportNote && <Notice tone={exportNote.ok ? "info" : "danger"}>{exportNote.text}</Notice>}

      {isLoading ? (
        <div className="flex flex-1 items-center justify-center pb-16">
          <Spinner size={22} />
        </div>
      ) : noHistory ? (
        <Empty title="Nothing here yet">
          <p className="mt-2 max-w-[340px] text-[13px] leading-relaxed text-ink/50">
            What you dictate is kept here, on this computer only.
          </p>
          <Button className="mt-5" onClick={() => setActivePage("dictation")}>
            Set up your hotkey
          </Button>
        </Empty>
      ) : isEmpty ? (
        <Empty title="No matches">
          <p className="mt-2 max-w-[300px] break-words text-[13px] leading-relaxed text-ink/50">
            Nothing you dictated contains “{search}”.
          </p>
        </Empty>
      ) : (
        <>
          {groups.map((group) => (
            <section key={group.day}>
              <h2 className="mb-2 px-4 text-[12px] font-medium text-ink/45">{group.day}</h2>
              <Panel>
                <ul className="divide-y divide-ink/[0.06]">
                  {group.items.map((entry) => (
                    <Entry key={entry.id} entry={entry} onDelete={(id) => void deleteEntry(id)} />
                  ))}
                </ul>
              </Panel>
            </section>
          ))}
          {hasMore && (
            <div className="flex justify-center">
              <Button onClick={loadMore}>Show more</Button>
            </div>
          )}
        </>
      )}
    </Page>
  );
}
