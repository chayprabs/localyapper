// Speech page -- the model catalog: size, languages, state and actions
import { useCallback, useEffect, useState } from "react";
import { getSetting, setSetting } from "@/lib/commands/settings";
import { listen } from "@tauri-apps/api/event";
import {
  cancelModelDownload,
  deleteModel,
  downloadModel,
  listModels,
  selectModel,
  type ModelInfo,
} from "@/lib/commands/models";

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

function errorText(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  return "Something went wrong";
}

export function SpeechPage() {
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [progress, setProgress] = useState<Progress | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [idleUnload, setIdleUnload] = useState("120");

  useEffect(() => {
    void getSetting("idle_unload_seconds")
      .then((v) => setIdleUnload(v))
      .catch(() => undefined);
  }, []);

  const refresh = useCallback(async () => {
    try {
      setModels(await listModels());
    } catch (e) {
      setError(errorText(e));
    }
  }, []);

  useEffect(() => {
    void refresh();
    const unlistenProgress = listen<Progress>("model-download-progress", (e) =>
      setProgress(e.payload),
    );
    const unlistenChanged = listen("models-changed", () => void refresh());
    const unlistenState = listen("model-state", () => void refresh());
    return () => {
      void unlistenProgress.then((f) => f());
      void unlistenChanged.then((f) => f());
      void unlistenState.then((f) => f());
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
    <div className="p-6 max-w-[720px]">
      <h1 className="text-[20px] font-semibold text-black/85 mb-1">Speech</h1>
      <p className="text-[13px] text-black/50 mb-5">
        Speech is recognised on this computer. Models download once and are
        checked against a pinned checksum.
      </p>
      {error && (
        <p role="alert" className="mb-4 text-[13px] text-[#ba1a1a]">
          {error}
        </p>
      )}
      <ul className="bg-white rounded-xl border border-black/[0.08] divide-y divide-black/[0.07]">
        {models.map((m) => {
          const downloading = busy === m.id && progress?.id === m.id;
          return (
            <li key={m.id} className="px-5 py-4 flex items-center gap-4">
              <div className="min-w-0 flex-1">
                <div className="flex items-center gap-2">
                  <span className="text-[14px] font-semibold text-black/85">{m.name}</span>
                  {m.active && (
                    <span className="text-[11px] font-semibold text-[#0058bc] bg-[#0058bc]/10 rounded-full px-2 py-0.5">
                      In use
                    </span>
                  )}
                  {m.recommended && (
                    <span className="text-[11px] text-black/50">Recommended for this computer</span>
                  )}
                </div>
                <div className="text-[12px] text-black/50">
                  {formatSize(m.size_bytes)} · {languagesLabel(m.languages)} · {m.license}
                  {m.state === "installed" ? " · Downloaded" : m.state === "partial" ? " · Partly downloaded" : ""}
                </div>
                {downloading && (
                  <div className="mt-2 h-1.5 rounded-full bg-black/[0.06] overflow-hidden" aria-label="Download progress">
                    <div className="h-full bg-[#0058bc]" style={{ width: `${progress.percent.toFixed(1)}%` }} />
                  </div>
                )}
              </div>
              <div className="flex items-center gap-2 shrink-0">
                {busy === m.id ? (
                  <button
                    className="h-7 px-3 text-[12px] rounded-md border border-black/[0.12]"
                    onClick={() => void cancelModelDownload()}
                  >
                    Cancel
                  </button>
                ) : m.state === "installed" ? (
                  <>
                    {!m.active && (
                      <button
                        className="h-7 px-3 bg-[#0058bc] text-white text-[12px] font-medium rounded-md"
                        onClick={() => void run(m.id, () => selectModel(m.id))}
                      >
                        Use
                      </button>
                    )}
                    <button
                      className="text-[12px] text-[#ba1a1a] hover:underline"
                      onClick={() => void run(m.id, () => deleteModel(m.id))}
                    >
                      Delete
                    </button>
                  </>
                ) : (
                  <button
                    className="h-7 px-3 bg-[#0058bc] text-white text-[12px] font-medium rounded-md disabled:opacity-40"
                    disabled={busy !== null}
                    onClick={() =>
                      void run(m.id, async () => {
                        await downloadModel(m.id);
                        if (!models.some((x) => x.state === "installed")) await selectModel(m.id);
                      })
                    }
                  >
                    {m.state === "partial" ? "Resume download" : "Download"}
                  </button>
                )}
              </div>
            </li>
          );
        })}
      </ul>
      <label className="mt-5 flex items-center justify-between gap-4 bg-white rounded-xl border border-black/[0.08] px-5 py-3">
        <span className="text-[13px] font-semibold text-black/85">
          Free memory when idle
          <span className="block text-[12px] font-normal text-black/50">
            The model is unloaded after this long without dictation.
          </span>
        </span>
        <select
          className="text-[13px] border border-black/[0.12] rounded-md px-2 h-8"
          value={idleUnload}
          onChange={(e) => {
            const v = e.target.value;
            setIdleUnload(v);
            void setSetting("idle_unload_seconds", v).catch((err: unknown) => setError(errorText(err)));
          }}
        >
          <option value="30">After 30 seconds</option>
          <option value="120">After 2 minutes</option>
          <option value="300">After 5 minutes</option>
          <option value="900">After 15 minutes</option>
          <option value="0">Never</option>
        </select>
      </label>
      <button
        className="mt-4 text-[12px] text-[#0058bc] hover:underline"
        onClick={() => void run("auto", () => selectModel("auto"))}
      >
        Choose automatically for this computer
      </button>
    </div>
  );
}
