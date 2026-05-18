// Models page -- speech recognition configuration only
import type { ReactNode } from "react";
import { useModels } from "@/hooks/useModels";
import { Icon } from "@/components/ui/Icon";

function formatSpeechModelName(modelName: string): string {
  switch (modelName) {
    case "parakeet-110m":
      return "Parakeet 110M";
    case "parakeet-0.6b":
      return "Parakeet 0.6B";
    default:
      return "Local Speech Engine";
  }
}

function StatusBadge({
  tone,
  children,
}: {
  tone: "neutral" | "accent" | "success" | "warning";
  children: ReactNode;
}) {
  const toneClass =
    tone === "success"
      ? "bg-[#28CD41]/[0.10] text-[#006b19]"
      : tone === "warning"
        ? "bg-[#FF9500]/[0.12] text-[#9a5a00]"
        : tone === "accent"
          ? "bg-[#0058bc]/[0.10] text-[#0058bc]"
          : "bg-black/[0.05] text-black/50";

  return (
    <span
      className={`inline-flex h-7 items-center rounded-full px-3 text-[12px] font-semibold ${toneClass}`}
    >
      {children}
    </span>
  );
}

function Row({
  label,
  children,
  isLast = false,
}: {
  label: string;
  children: ReactNode;
  isLast?: boolean;
}) {
  return (
    <div
      className={`min-h-[52px] px-5 py-3 flex items-center justify-between gap-4 ${
        !isLast ? "border-b border-black/[0.07]" : ""
      }`}
    >
      <span className="text-[13px] font-semibold text-black/85">{label}</span>
      <div className="min-w-0 text-right">{children}</div>
    </div>
  );
}

function StatusLine({
  tone,
  label,
}: {
  tone: "success" | "warning" | "neutral";
  label: string;
}) {
  const dotClass =
    tone === "success"
      ? "bg-[#28CD41]"
      : tone === "warning"
        ? "bg-[#FF9500]"
        : "bg-black/[0.22]";
  const textClass =
    tone === "success" ? "text-[#28CD41]" : "text-black/55";

  return (
    <div className="flex items-center justify-end gap-1.5">
      <div className={`w-2 h-2 rounded-full ${dotClass}`} />
      <span className={`text-[13px] font-medium ${textClass}`}>{label}</span>
    </div>
  );
}

function InlineAction({
  children,
  tone = "primary",
  onClick,
}: {
  children: ReactNode;
  tone?: "primary" | "danger";
  onClick: () => void;
}) {
  const className =
    tone === "danger"
      ? "text-[12px] text-[#ba1a1a] hover:underline"
      : "h-7 px-3 bg-[#0058bc] text-white text-[12px] font-medium rounded-md hover:bg-[#004ea8] transition-colors";

  return (
    <button onClick={onClick} className={className}>
      {children}
    </button>
  );
}

export function ModelsPage() {
  const {
    speechModel,
    speechModelFileStatus,
    speechModelLoaded,
    speechModelLoading,
    speechModelError,
    speechModelDownloading,
    speechModelDownloadProgress,
    downloadSpeechModelAction,
    cancelSpeechModelDownload,
    deleteSpeechModelAction,
    loadSpeechModel,
    isLoading,
  } = useModels();

  if (isLoading) {
    return (
      <div className="px-12 py-10">
        <h1 className="text-[26px] font-semibold text-black/85">Speech</h1>
      </div>
    );
  }

  const displayModelName = formatSpeechModelName(
    speechModelFileStatus.model_name || speechModel,
  );
  const headerTone = speechModelDownloading
    ? "accent"
    : speechModelLoaded
      ? "success"
      : speechModelFileStatus.exists
        ? "warning"
        : "neutral";
  const headerLabel = speechModelDownloading
    ? "Downloading"
    : speechModelLoaded
      ? "Ready"
      : speechModelFileStatus.exists
        ? "Needs Load"
        : "Not Installed";

  return (
    <div className="px-12 py-10">
      <header className="mb-6">
        <h1 className="text-[26px] font-semibold text-black/85">Speech</h1>
        <p className="mt-2 text-[13px] text-black/50 max-w-[540px] leading-relaxed">
          Everything here runs locally. Download the speech files once, keep
          the engine ready, and dictation stays fully on-device.
        </p>
      </header>

      <section>
        <h2 className="text-[10px] uppercase font-medium text-black/[0.40] tracking-[0.06em] mb-2 px-1">
          VOICE ENGINE
        </h2>

        <div className="bg-white rounded-[10px] border border-black/[0.07] shadow-sm overflow-hidden">
          <div className="px-5 py-4 border-b border-black/[0.07] flex items-start justify-between gap-4">
            <div className="min-w-0">
              <p className="text-[16px] font-semibold text-black/85">
                {displayModelName}
              </p>
              <p className="mt-1 text-[12px] text-black/[0.45]">
                On-device speech recognition for everyday dictation
              </p>
            </div>
            <StatusBadge tone={headerTone}>{headerLabel}</StatusBadge>
          </div>

          {speechModelDownloading ? (
            <div className="px-5 py-4 border-b border-black/[0.07]">
              <div className="flex items-center justify-between gap-4 mb-2">
                <div>
                  <p className="text-[13px] font-semibold text-black/85">
                    Downloading Speech Files
                  </p>
                  <p className="text-[12px] text-black/[0.45]">
                    {speechModelDownloadProgress?.downloaded_mb ?? 0} /{" "}
                    {speechModelDownloadProgress?.total_mb ?? 458} MB
                  </p>
                </div>
                <span className="text-[12px] font-medium text-[#0058bc]">
                  {(speechModelDownloadProgress?.percent ?? 0).toFixed(0)}%
                </span>
              </div>

              <div className="w-full h-1.5 rounded-full bg-black/[0.06] mb-2.5">
                <div
                  className="h-full rounded-full bg-[#0058bc] transition-all duration-300"
                  style={{
                    width: `${Math.min(
                      speechModelDownloadProgress?.percent ?? 0,
                      100,
                    )}%`,
                  }}
                />
              </div>

              <div className="flex items-center justify-between gap-3">
                <span className="text-[11px] text-black/[0.38]">
                  {(speechModelDownloadProgress?.speed_mbps ?? 0) > 0
                    ? `${(speechModelDownloadProgress?.speed_mbps ?? 0).toFixed(1)} MB/s`
                    : "Preparing download..."}
                </span>
                <InlineAction tone="danger" onClick={cancelSpeechModelDownload}>
                  Cancel Download
                </InlineAction>
              </div>
            </div>
          ) : (
            <Row label="Speech Files">
              {speechModelFileStatus.exists ? (
                <div className="flex items-center justify-end gap-3">
                  <StatusLine
                    tone="success"
                    label={`Installed (${speechModelFileStatus.size_mb} MB)`}
                  />
                  <InlineAction tone="danger" onClick={deleteSpeechModelAction}>
                    Remove Files
                  </InlineAction>
                </div>
              ) : (
                <div className="flex items-center justify-end gap-3">
                  <StatusLine tone="warning" label="Not installed" />
                  <InlineAction onClick={downloadSpeechModelAction}>
                    Download Files
                  </InlineAction>
                </div>
              )}
            </Row>
          )}

          {speechModelFileStatus.exists && !speechModelDownloading && (
            <Row label="Engine" isLast={!speechModelError}>
              {speechModelLoaded ? (
                <StatusLine tone="success" label="Ready for dictation" />
              ) : speechModelLoading ? (
                <div className="flex items-center justify-end gap-2">
                  <Icon name="progress_activity" size={16} className="text-[#0058bc] animate-spin" />
                  <span className="text-[13px] text-black/50">Starting...</span>
                </div>
              ) : (
                <div className="flex items-center justify-end gap-3">
                  <StatusLine tone="neutral" label="Not loaded" />
                  <InlineAction onClick={loadSpeechModel}>
                    Start Engine
                  </InlineAction>
                </div>
              )}
            </Row>
          )}

          {speechModelError && (
            <div className="px-5 py-4">
              <p className="text-[11px] text-[#ba1a1a]">{speechModelError}</p>
            </div>
          )}
        </div>
      </section>
    </div>
  );
}
