// The speech model catalog: size, languages, state and actions
import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  cancelModelDownload,
  deleteModel,
  downloadModel,
  listModels,
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

export function Models() {
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [progress, setProgress] = useState<Progress | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      setModels(await listModels());
    } catch (e) {
      setError(errorText(e));
    }
  }, []);

  useEffect(() => {
    void refresh();
    const unlisten = [
      listen<Progress>("model-download-progress", (e) => setProgress(e.payload)),
      listen("models-changed", () => void refresh()),
      listen("model-state", () => void refresh()),
    ];
    return () => {
      for (const u of unlisten) void u.then((f) => f());
    };
  }, [refresh]);

  const run = useCallback(
    async (id: string, action: () => Promise<void>) => {
      setError(null);
      setBusy(id);
      try {
        await action();
      } catch (e) {
        setError(errorText(e));
      } finally {
        setBusy(null);
        setProgress(null);
        void refresh();
      }
    },
    [refresh],
  );

  return (
    <Section
      title="Model"
      action={
        <Button
          variant="ghost"
          className="h-6"
          title="Choose the best model for this computer"
          disabled={busy !== null}
          onClick={() => void run("auto", () => selectModel("auto"))}
        >
          Auto
        </Button>
      }
    >
      {models.map((m) => {
        const working = busy === m.id;
        const percent = working && progress?.id === m.id ? progress.percent : null;
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
              {working && m.state !== "installed" ? (
                <Button onClick={() => void cancelModelDownload()}>Cancel</Button>
              ) : working ? (
                <span className="flex h-7 items-center gap-2 px-1 text-[12.5px] text-ink/50">
                  <Spinner size={12} />
                  Working
                </span>
              ) : m.state === "installed" ? (
                <>
                  {!m.active && (
                    <Button disabled={busy !== null} onClick={() => void run(m.id, () => selectModel(m.id))}>
                      Use
                    </Button>
                  )}
                  {/* Checks every file against its checksum and fetches
                      again only what is missing or damaged. */}
                  <Button
                    variant="ghost"
                    title="Check the downloaded files and repair them if needed"
                    disabled={busy !== null}
                    onClick={() => void run(m.id, () => downloadModel(m.id))}
                  >
                    Check
                  </Button>
                  <IconButton
                    label={`Delete ${m.name}`}
                    tone="danger"
                    className="-mr-1.5"
                    disabled={busy !== null}
                    onClick={() => void run(m.id, () => deleteModel(m.id))}
                  >
                    <Icon name="delete" size={14} />
                  </IconButton>
                </>
              ) : (
                <Button
                  disabled={busy !== null}
                  onClick={() =>
                    void run(m.id, async () => {
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
