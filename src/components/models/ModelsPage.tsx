import { useState, useRef, useEffect } from "react";
import { useModels } from "@/hooks/useModels";

const PROVIDER_OPTIONS: { value: string; label: string }[] = [
  { value: "openai", label: "OpenAI" },
  { value: "anthropic", label: "Anthropic" },
  { value: "groq", label: "Groq" },
];

const LLM_TABS = ["local", "ollama", "byok"] as const;
const LLM_TAB_LABELS: Record<string, string> = {
  local: "Local Model",
  ollama: "Ollama",
  byok: "BYOK (API Key)",
};

/* ------------------------------------------------------------------ */
/*  Dropdown component (custom, click-to-open)                         */
/* ------------------------------------------------------------------ */
function Dropdown({
  value,
  options,
  onChange,
  width = "w-[160px]",
  variant = "default",
}: {
  value: string;
  options: { value: string; label: string }[];
  onChange: (value: string) => void;
  width?: string;
  variant?: "default" | "pill";
}) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    function handleClick(e: MouseEvent) {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        setOpen(false);
      }
    }
    document.addEventListener("mousedown", handleClick);
    return () => document.removeEventListener("mousedown", handleClick);
  }, [open]);

  const selected = options.find((o) => o.value === value);

  const isPill = variant === "pill";
  const triggerClass = isPill
    ? `${width} flex items-center justify-between gap-2 bg-[#F5F5F5] border border-black/[0.05] px-3 py-1.5 rounded-lg cursor-pointer`
    : `${width} flex items-center justify-between px-2.5 bg-white border border-black/[0.12] rounded-[6px] h-7 cursor-pointer`;

  return (
    <div ref={ref} className="relative">
      <div onClick={() => setOpen(!open)} className={triggerClass}>
        <span className="text-[13px] text-black/85 truncate">
          {selected?.label ?? value}
        </span>
        <span className="material-symbols-outlined text-[16px] text-black/[0.40] shrink-0">
          expand_more
        </span>
      </div>
      {open && (
        <div className="absolute right-0 top-full mt-1 bg-white rounded-lg border border-black/[0.07] shadow-lg z-10 min-w-full py-1">
          {options.map((opt) => (
            <div
              key={opt.value}
              onClick={() => {
                onChange(opt.value);
                setOpen(false);
              }}
              className={`px-3 py-1.5 text-[13px] cursor-pointer hover:bg-black/[0.04] ${
                opt.value === value
                  ? "text-[#0058bc] font-medium"
                  : "text-black/85"
              }`}
            >
              {opt.label}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

/* ------------------------------------------------------------------ */
/*  Segmented control                                                  */
/* ------------------------------------------------------------------ */
function SegmentedControl({
  value,
  onChange,
}: {
  value: string;
  onChange: (value: string) => void;
}) {
  return (
    <div className="p-1">
      <div className="flex bg-black/[0.06] rounded-lg p-0.5 h-8">
        {LLM_TABS.map((tab) => (
          <button
            key={tab}
            onClick={() => onChange(tab)}
            className={`flex-1 flex items-center justify-center text-[13px] font-medium rounded-[6px] transition-all ${
              value === tab
                ? "bg-white shadow-sm text-black/85"
                : "text-black/[0.40]"
            }`}
          >
            {LLM_TAB_LABELS[tab]}
          </button>
        ))}
      </div>
    </div>
  );
}

/* ------------------------------------------------------------------ */
/*  Row component                                                      */
/* ------------------------------------------------------------------ */
function Row({
  label,
  children,
  isLast = false,
}: {
  label: string;
  children: React.ReactNode;
  isLast?: boolean;
}) {
  return (
    <div
      className={`h-[44px] px-4 flex items-center justify-between ${
        !isLast ? "border-b border-black/[0.07]" : ""
      }`}
    >
      <span className="text-[13px] font-semibold text-black/85">{label}</span>
      <div>{children}</div>
    </div>
  );
}

/* ------------------------------------------------------------------ */
/*  Local tab content                                                  */
/* ------------------------------------------------------------------ */
function LocalContent({
  fileExists,
  sizeMb,
  loaded,
  downloading,
  downloadProgress,
  onDownload,
  onCancelDownload,
  onDelete,
  onLoad,
}: {
  fileExists: boolean;
  sizeMb: number;
  loaded: boolean;
  downloading: boolean;
  downloadProgress: { percent: number; downloaded_mb: number; total_mb: number; speed_mbps: number } | null;
  onDownload: () => void;
  onCancelDownload: () => void;
  onDelete: () => void;
  onLoad: () => void;
}) {
  return (
    <div className="border-t border-black/[0.07]">
      <Row label="Model">
        <span className="text-[13px] text-black/50">Qwen3 0.6B (397 MB)</span>
      </Row>

      {/* Download status / progress */}
      {downloading ? (
        <div className="px-4 pb-3 pt-1">
          <div className="w-full h-1.5 rounded-full bg-black/[0.06] mb-1.5">
            <div
              className="h-full rounded-full bg-[#0058bc] transition-all duration-300"
              style={{ width: `${Math.min(downloadProgress?.percent ?? 0, 100)}%` }}
            />
          </div>
          <div className="flex justify-between text-[11px] text-black/[0.40] mb-2">
            <span>{(downloadProgress?.percent ?? 0).toFixed(0)}% — {downloadProgress?.downloaded_mb ?? 0} / {downloadProgress?.total_mb ?? 397} MB</span>
            <span>{(downloadProgress?.speed_mbps ?? 0) > 0 ? `${(downloadProgress?.speed_mbps ?? 0).toFixed(1)} MB/s` : "Starting..."}</span>
          </div>
          <button
            onClick={onCancelDownload}
            className="text-[12px] text-[#ba1a1a] hover:underline"
          >
            Cancel
          </button>
        </div>
      ) : (
        <Row label="File Status">
          {fileExists ? (
            <div className="flex items-center gap-3">
              <div className="flex items-center gap-1.5">
                <div className="w-2 h-2 rounded-full bg-[#28CD41]" />
                <span className="text-[13px] font-medium text-[#28CD41]">
                  Downloaded ({sizeMb} MB)
                </span>
              </div>
              <button
                onClick={onDelete}
                className="text-[12px] text-[#ba1a1a] hover:underline"
              >
                Delete
              </button>
            </div>
          ) : (
            <div className="flex items-center gap-3">
              <div className="flex items-center gap-1.5">
                <div className="w-2 h-2 rounded-full bg-[#FF9500]" />
                <span className="text-[13px] font-medium text-black/50">Not downloaded</span>
              </div>
              <button
                onClick={onDownload}
                className="h-6 px-3 bg-[#0058bc] text-white text-[12px] font-medium rounded-md hover:bg-[#004ea8] transition-colors"
              >
                Download
              </button>
            </div>
          )}
        </Row>
      )}

      {/* Load status (only if downloaded) */}
      {fileExists && !downloading && (
        <Row label="Engine Status" isLast>
          {loaded ? (
            <div className="flex items-center gap-1.5">
              <div className="w-2 h-2 rounded-full bg-[#28CD41]" />
              <span className="text-[13px] font-medium text-[#28CD41]">Loaded</span>
            </div>
          ) : (
            <div className="flex items-center gap-3">
              <div className="flex items-center gap-1.5">
                <div className="w-2 h-2 rounded-full bg-black/[0.25]" />
                <span className="text-[13px] font-medium text-black/50">Not loaded</span>
              </div>
              <button
                onClick={onLoad}
                className="h-6 px-3 bg-[#0058bc] text-white text-[12px] font-medium rounded-md hover:bg-[#004ea8] transition-colors"
              >
                Load Model
              </button>
            </div>
          )}
        </Row>
      )}
    </div>
  );
}

/* ------------------------------------------------------------------ */
/*  Ollama tab content                                                 */
/* ------------------------------------------------------------------ */
function OllamaContent({
  ollamaStatus,
  ollamaModel,
  onModelChange,
}: {
  ollamaStatus: { running: boolean; models: string[] } | null;
  ollamaModel: string;
  onModelChange: (model: string) => void;
}) {
  const running = ollamaStatus?.running ?? false;
  const models = ollamaStatus?.models ?? [];
  const modelOptions = models.map((m) => ({ value: m, label: m }));

  return (
    <div className="border-t border-black/[0.07]">
      <Row label="Active Model">
        {running && models.length > 0 ? (
          <Dropdown
            value={ollamaModel}
            options={modelOptions}
            onChange={onModelChange}
            width="w-[160px]"
          />
        ) : (
          <span className="text-[13px] text-black/[0.26]">No models</span>
        )}
      </Row>
      <Row label="Service Status">
        <div className="flex items-center gap-2">
          <div
            className={`w-2 h-2 rounded-full ${running ? "bg-[#28CD41]" : "bg-[#ba1a1a]"}`}
          />
          <span
            className={`text-[13px] font-medium ${running ? "text-[#28CD41]" : "text-[#ba1a1a]"}`}
          >
            {running
              ? `Running \u00B7 ${models.length} model${models.length !== 1 ? "s" : ""} available`
              : "Not running"}
          </span>
        </div>
      </Row>
      <Row label="Ollama URL" isLast>
        <span className="text-[13px] text-black/50">localhost:11434</span>
      </Row>
    </div>
  );
}

/* ------------------------------------------------------------------ */
/*  BYOK tab content                                                   */
/* ------------------------------------------------------------------ */
function ByokRows({
  provider,
  apiKey,
  onProviderChange,
  onApiKeyChange,
}: {
  provider: string;
  apiKey: string;
  onProviderChange: (provider: string) => void;
  onApiKeyChange: (key: string) => void;
}) {
  const [showKey, setShowKey] = useState(false);

  return (
    <div className="border-t border-black/[0.07]">
      <Row label="Provider">
        <Dropdown
          value={provider}
          options={PROVIDER_OPTIONS}
          onChange={onProviderChange}
          width="w-[120px]"
        />
      </Row>
      <Row label="API Key" isLast>
        <div className="relative w-[200px]">
          <input
            type={showKey ? "text" : "password"}
            value={apiKey}
            onChange={(e) => onApiKeyChange(e.target.value)}
            placeholder="sk-\u2022\u2022\u2022\u2022\u2022\u2022\u2022\u2022\u2022\u2022\u2022\u2022\u2022\u2022\u2022\u2022\u2022"
            className="w-full h-7 bg-white border border-black/[0.12] rounded-[6px] text-[13px] px-2.5 pr-8 focus:outline-none focus:ring-1 focus:ring-[#0058bc] placeholder:text-black/[0.25]"
          />
          <button
            type="button"
            onClick={() => setShowKey(!showKey)}
            className="absolute right-2 top-1/2 -translate-y-1/2"
          >
            <span className="material-symbols-outlined text-[16px] text-black/[0.35]">
              {showKey ? "visibility_off" : "visibility"}
            </span>
          </button>
        </div>
      </Row>
    </div>
  );
}

/* ------------------------------------------------------------------ */
/*  Main page                                                          */
/* ------------------------------------------------------------------ */
export function ModelsPage() {
  const {
    llmMode,
    ollamaModel,
    byokProvider,
    byokApiKey,
    ollamaStatus,
    connectionResult,
    isLoading,
    isTesting,
    setLlmMode,
    setOllamaModel,
    setByokProvider,
    setByokApiKey,
    testConnection,
    llmFileStatus,
    llmLoaded,
    llmDownloading,
    llmDownloadProgress,
    downloadLocalModel,
    cancelLocalModelDownload,
    deleteLocalModel,
    loadLocalModel,
    whisperFileStatus,
    whisperLoaded,
    whisperDownloading,
    whisperDownloadProgress,
    downloadWhisperModelAction,
    cancelWhisperDownload,
    deleteWhisperModelAction,
    loadWhisperModel,
  } = useModels();

  if (isLoading) {
    return (
      <div className="px-12 py-10">
        <h1 className="text-[26px] font-semibold text-black/85">Models</h1>
      </div>
    );
  }

  return (
    <div className="px-12 py-10">
      {/* Header */}
      <header className="mb-5">
        <h1 className="text-[26px] font-semibold text-black/85">Models</h1>
      </header>

      {/* Speech Recognition */}
      <section className="mb-5">
        <h2 className="text-[10px] uppercase font-medium text-black/[0.40] tracking-[0.06em] mb-2 px-1">
          SPEECH RECOGNITION
        </h2>
        <div className="bg-white rounded-[10px] border border-black/[0.07] shadow-sm overflow-hidden">
          <Row label="Model">
            <span className="text-[13px] text-black/50">Whisper base.en (148 MB)</span>
          </Row>

          {whisperDownloading ? (
            <div className="px-4 pb-3 pt-1">
              <div className="w-full h-1.5 rounded-full bg-black/[0.06] mb-1.5">
                <div
                  className="h-full rounded-full bg-[#0058bc] transition-all duration-300"
                  style={{ width: `${Math.min(whisperDownloadProgress?.percent ?? 0, 100)}%` }}
                />
              </div>
              <div className="flex justify-between text-[11px] text-black/[0.40] mb-2">
                <span>{(whisperDownloadProgress?.percent ?? 0).toFixed(0)}% — {whisperDownloadProgress?.downloaded_mb ?? 0} / {whisperDownloadProgress?.total_mb ?? 148} MB</span>
                <span>{(whisperDownloadProgress?.speed_mbps ?? 0) > 0 ? `${(whisperDownloadProgress?.speed_mbps ?? 0).toFixed(1)} MB/s` : "Starting..."}</span>
              </div>
              <button
                onClick={cancelWhisperDownload}
                className="text-[12px] text-[#ba1a1a] hover:underline"
              >
                Cancel
              </button>
            </div>
          ) : (
            <Row label="File Status">
              {whisperFileStatus.exists ? (
                <div className="flex items-center gap-3">
                  <div className="flex items-center gap-1.5">
                    <div className="w-2 h-2 rounded-full bg-[#28CD41]" />
                    <span className="text-[13px] font-medium text-[#28CD41]">
                      Downloaded ({whisperFileStatus.size_mb} MB)
                    </span>
                  </div>
                  <button
                    onClick={deleteWhisperModelAction}
                    className="text-[12px] text-[#ba1a1a] hover:underline"
                  >
                    Delete
                  </button>
                </div>
              ) : (
                <div className="flex items-center gap-3">
                  <div className="flex items-center gap-1.5">
                    <div className="w-2 h-2 rounded-full bg-[#FF9500]" />
                    <span className="text-[13px] font-medium text-black/50">Not downloaded</span>
                  </div>
                  <button
                    onClick={downloadWhisperModelAction}
                    className="h-6 px-3 bg-[#0058bc] text-white text-[12px] font-medium rounded-md hover:bg-[#004ea8] transition-colors"
                  >
                    Download
                  </button>
                </div>
              )}
            </Row>
          )}

          {whisperFileStatus.exists && !whisperDownloading && (
            <Row label="Engine Status" isLast>
              {whisperLoaded ? (
                <div className="flex items-center gap-1.5">
                  <div className="w-2 h-2 rounded-full bg-[#28CD41]" />
                  <span className="text-[13px] font-medium text-[#28CD41]">Loaded</span>
                </div>
              ) : (
                <div className="flex items-center gap-3">
                  <div className="flex items-center gap-1.5">
                    <div className="w-2 h-2 rounded-full bg-black/[0.25]" />
                    <span className="text-[13px] font-medium text-black/50">Not loaded</span>
                  </div>
                  <button
                    onClick={loadWhisperModel}
                    className="h-6 px-3 bg-[#0058bc] text-white text-[12px] font-medium rounded-md hover:bg-[#004ea8] transition-colors"
                  >
                    Load Model
                  </button>
                </div>
              )}
            </Row>
          )}
        </div>
      </section>

      {/* Language Model */}
      <section>
        <h2 className="text-[10px] uppercase font-medium text-black/[0.40] tracking-[0.06em] mb-2 px-1">
          LANGUAGE MODEL
        </h2>
        <div className={`bg-white rounded-[10px] border border-black/[0.07] shadow-sm overflow-hidden${llmMode === "byok" ? " mb-3" : ""}`}>
          <SegmentedControl value={llmMode} onChange={(v) => setLlmMode(v as "local" | "ollama" | "byok")} />

          {llmMode === "local" && (
            <LocalContent
              fileExists={llmFileStatus.exists}
              sizeMb={llmFileStatus.size_mb}
              loaded={llmLoaded}
              downloading={llmDownloading}
              downloadProgress={llmDownloadProgress}
              onDownload={downloadLocalModel}
              onCancelDownload={cancelLocalModelDownload}
              onDelete={deleteLocalModel}
              onLoad={loadLocalModel}
            />
          )}
          {llmMode === "ollama" && (
            <OllamaContent
              ollamaStatus={ollamaStatus}
              ollamaModel={ollamaModel}
              onModelChange={setOllamaModel}
            />
          )}
          {llmMode === "byok" && (
            <ByokRows
              provider={byokProvider}
              apiKey={byokApiKey}
              onProviderChange={(v) =>
                setByokProvider(v as "openai" | "anthropic" | "groq")
              }
              onApiKeyChange={setByokApiKey}
            />
          )}
        </div>

        {/* BYOK action area — outside the card */}
        {llmMode === "byok" && (
          <div className="flex flex-col items-end px-1">
            <button
              onClick={testConnection}
              disabled={isTesting || !byokApiKey}
              className="w-[130px] h-8 bg-[#0058bc] text-white text-[13px] font-medium rounded-lg hover:bg-[#004ea8] transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {isTesting ? "Testing..." : "Test Connection"}
            </button>
            {connectionResult ? (
              <p
                className={`text-[11px] mt-2 font-medium ${
                  connectionResult.success ? "text-[#28CD41]" : "text-[#ba1a1a]"
                }`}
              >
                {connectionResult.success
                  ? `Connected \u00B7 ${connectionResult.latency_ms}ms`
                  : connectionResult.error ?? "Connection failed"}
              </p>
            ) : (
              <p className="text-[11px] text-black/[0.40] mt-2">
                Your API key is stored locally and never leaves your device.
              </p>
            )}
          </div>
        )}
      </section>
    </div>
  );
}
