// The speech model catalog: size, languages, state and actions
import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  cancelModelDownload,
  deleteModel,
  downloadModel,
  listModels,
  modelDownloadCurrent,
  selectModel,
  type ModelInfo,
} from "@/lib/commands/models";
import { Button, IconButton } from "@/components/ui/Button";
import { Icon } from "@/components/ui/Icon";
import { errorText, Notice, Row, Section, Spinner } from "@/components/settings/controls";
import { cn } from "@/lib/utils";

interface Progress {
  id: string;
  downloaded: number;
  total: number;
  percent: number;
}

function formatSize(bytes: number): string {
  return bytes >= 1e9 ? `${(bytes / 1e9).toFixed(1)} GB` : `${Math.round(bytes / 1e6)} MB`;
}

function languagesLabel(languages: string[]): string {
  if (languages.length === 1 && languages[0] === "en") return "English";
  return `${languages.length} languages`;
}

/** An action this page started on one model. */
interface Busy {
  id: string;
  /** A download or repair, which Cancel can stop. */
  download: boolean;
}

export function Models() {
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [progress, setProgress] = useState<Progress | null>(null);
  const [busy, setBusy] = useState<Busy | null>(null);
  // A download that was already running when the page opened (started
  // here before switching pages, or by the wizard).
  const [background, setBackground] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const cancelRequested = useRef(false);

  const refresh = useCallback(async () => {
    try {
      setModels(await listModels());
    } catch (e) {
      setError(errorText(e));
    }
  }, []);

  const syncBackground = useCallback(async () => {
    try {
      const id = await modelDownloadCurrent();
      setBackground(id);
      if (id === null) setProgress(null);
    } catch {
      // Leave the current view; the next event corrects it.
    }
  }, []);

  useEffect(() => {
    void refresh();
    void syncBackground();
    const unlisten = [
      listen<Progress>("model-download-progress", (e) => {
        setProgress(e.payload);
        if (e.payload.percent < 100) setBackground((b) => b ?? e.payload.id);
      }),
      listen("models-changed", () => {
        void refresh();
        void syncBackground();
      }),
      listen("model-state", () => void refresh()),
    ];
    return () => {
      for (const u of unlisten) void u.then((f) => f());
    };
  }, [refresh, syncBackground]);

  const run = useCallback(
    async (id: string, download: boolean, action: () => Promise<void>) => {
      setError(null);
      cancelRequested.current = false;
      setBusy({ id, download });
      try {
        await action();
      } catch (e) {
        // A download the user cancelled is not an error.
        if (!cancelRequested.current) setError(errorText(e));
      } finally {
        cancelRequested.current = false;
        setBusy(null);
        setProgress(null);
        void refresh();
        void syncBackground();
      }
    },
    [refresh, syncBackground],
  );

  const cancel = useCallback(() => {
    cancelRequested.current = true;
    void cancelModelDownload();
  }, []);

  const anyBusy = busy !== null || background !== null;

  return (
    <Section
      title="Model"
      action={
        <Button
          variant="ghost"
          className="h-6"
          title="Choose the best model for this computer"
          disabled={anyBusy}
          onClick={() => void run("auto", false, () => selectModel("auto"))}
        >
          Auto
        </Button>
      }
    >
      {models.map((m) => {
        const downloading = (busy?.id === m.id && busy.download) || background === m.id;
        const working = downloading || busy?.id === m.id;
        const percent = downloading && progress?.id === m.id ? progress.percent : null;
        return (
          <Row
            key={m.id}
            label={
              <span className="flex items-center gap-2">
                {/* Solid dot: the model in use. */}
                <span
                  title={m.active ? "In use" : undefined}
                  className={cn("h-1.5 w-1.5 shrink-0 rounded-full", m.active ? "bg-ink" : "border border-ink/30")}
                />
                {m.name}
                {m.active && <span className="sr-only">, in use</span>}
              </span>
            }
            hint={`${m.license}${m.recommended ? ". Recommended for this computer." : ""}`}
            detail={
              <span className="font-mono text-[11.5px]">
                {formatSize(m.size_bytes)} · {languagesLabel(m.languages)}
                {m.state === "partial" && " · partial"}
              </span>
            }
            below={
              percent !== null && (
                <div className="flex animate-fade-in items-center gap-3 pb-1 pt-1.5">
                  <div className="h-px flex-1 bg-ink/15" aria-label="Download progress">
                    <div
                      className="-mt-px h-[3px] bg-ink transition-[width] duration-300 ease-out"
                      style={{ width: `${percent.toFixed(1)}%` }}
                    />
                  </div>
                  <span className="w-9 text-right font-mono text-[11.5px] text-ink/60">{Math.floor(percent)}%</span>
                </div>
              )
            }
          >
            <span className="flex items-center gap-1">
              {downloading ? (
                <Button onClick={cancel}>Cancel</Button>
              ) : working ? (
                <span className="flex h-7 items-center gap-2 px-1 text-[12.5px] text-ink/50">
                  <Spinner size={12} />
                  Working
                </span>
              ) : m.state === "installed" ? (
                <>
                  {!m.active && (
                    <Button disabled={anyBusy} onClick={() => void run(m.id, false, () => selectModel(m.id))}>
                      Use
                    </Button>
                  )}
                  {/* Checks every file against its checksum and fetches
                      again only what is missing or damaged. */}
                  <Button
                    variant="ghost"
                    title="Check the downloaded files and repair them if needed"
                    disabled={anyBusy}
                    onClick={() => void run(m.id, true, () => downloadModel(m.id))}
                  >
                    Check
                  </Button>
                  <IconButton
                    label={`Delete ${m.name}`}
                    tone="danger"
                    className="-mr-1.5"
                    disabled={anyBusy}
                    onClick={() => void run(m.id, false, () => deleteModel(m.id))}
                  >
                    <Icon name="delete" size={14} />
                  </IconButton>
                </>
              ) : (
                <Button
                  disabled={anyBusy}
                  onClick={() =>
                    void run(m.id, true, async () => {
                      await downloadModel(m.id);
                      if (!models.some((x) => x.state === "installed")) await selectModel(m.id);
                    })
                  }
                >
                  {m.state === "partial" ? "Resume" : "Download"}
                </Button>
              )}
            </span>
          </Row>
        );
      })}
      {error && (
        <div className="py-2">
          <Notice>{error}</Notice>
        </div>
      )}
    </Section>
  );
}
