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
import { Chip, errorText, Row, Section } from "@/components/settings/controls";

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
      title="Speech model"
      action={
        <Button size="sm" variant="ghost" disabled={busy !== null} onClick={() => void run("auto", () => selectModel("auto"))}>
          Pick for this computer
        </Button>
      }
    >
      {models.map((m) => {
        const working = busy === m.id;
        const percent = working && progress?.id === m.id ? progress.percent : null;
        const state = m.state === "installed" ? "Downloaded" : m.state === "partial" ? "Partly downloaded" : null;
        return (
          <Row
            key={m.id}
            label={
              <span className="flex flex-wrap items-center gap-x-2 gap-y-1">
                {m.name}
                {m.active && <Chip tone="accent">In use</Chip>}
                {m.recommended && <Chip>Recommended</Chip>}
              </span>
            }
            hint={[formatSize(m.size_bytes), languagesLabel(m.languages), m.license, state].filter(Boolean).join(" · ")}
            below={
              percent !== null && (
                <div className="mt-2.5 flex animate-fade-in items-center gap-3">
                  <div className="h-1 flex-1 overflow-hidden rounded-full bg-ink/[0.08]" aria-label="Download progress">
                    <div
                      className="h-full rounded-full bg-accent transition-[width] duration-300 ease-out"
                      style={{ width: `${percent.toFixed(1)}%` }}
                    />
                  </div>
                  <span className="w-9 text-right text-[12px] tabular-nums text-ink/50">{Math.floor(percent)}%</span>
                </div>
              )
            }
          >
            <span className="flex items-center gap-1">
              {working ? (
                <Button size="sm" onClick={() => void cancelModelDownload()}>
                  Cancel
                </Button>
              ) : m.state === "installed" ? (
                <>
                  {!m.active && (
                    <Button size="sm" disabled={busy !== null} onClick={() => void run(m.id, () => selectModel(m.id))}>
                      Use
                    </Button>
                  )}
                  <IconButton
                    label={`Delete ${m.name}`}
                    tone="danger"
                    disabled={busy !== null}
                    onClick={() => void run(m.id, () => deleteModel(m.id))}
                  >
                    <Icon name="delete" size={16} />
                  </IconButton>
                </>
              ) : (
                <Button
                  size="sm"
                  variant={models.some((x) => x.state === "installed") ? "soft" : "primary"}
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
        <p role="alert" className="px-4 py-3 text-[12px] font-medium text-danger">
          {error}
        </p>
      )}
    </Section>
  );
}
