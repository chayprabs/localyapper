// Models page -- speech recognition configuration only
import type { ReactNode } from "react";
import { useModels } from "@/hooks/useModels";

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
      className={`min-h-[44px] px-4 py-3 flex items-center justify-between gap-4 ${
        !isLast ? "border-b border-black/[0.07]" : ""
      }`}
    >
      <span className="text-[13px] font-semibold text-black/85">{label}</span>
      <div className="text-right">{children}</div>
    </div>
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

  const packageLabel =
    speechModel === "parakeet-0.6b"
      ? "Local speech package (high accuracy)"
      : "Built-in local speech package";

  return (
    <div className="px-12 py-10">
      <header className="mb-5">
        <h1 className="text-[26px] font-semibold text-black/85">Speech</h1>
        <p className="mt-2 text-[13px] text-black/50 max-w-[520px] leading-relaxed">
          LocalYapper uses one local speech pipeline for dictation. Download the
          speech package once, keep it loaded here, and the rest of the app
          stays fully on-device.
        </p>
      </header>

      <section>
        <h2 className="text-[10px] uppercase font-medium text-black/[0.40] tracking-[0.06em] mb-2 px-1">
          LOCAL SPEECH
        </h2>
        <div className="bg-white rounded-[10px] border border-black/[0.07] shadow-sm overflow-hidden">
          <Row label="Pipeline">
            <span className="text-[13px] text-black/50">
              Speech-to-text + learned corrections
            </span>
          </Row>

          <Row label="Package">
            <span className="text-[13px] text-black/50">{packageLabel}</span>
          </Row>

          {speechModelDownloading ? (
            <div className="px-4 pb-4 pt-2">
              <div className="w-full h-1.5 rounded-full bg-black/[0.06] mb-1.5">
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
              <div className="flex justify-between text-[11px] text-black/[0.40] mb-3">
                <span>
                  {(speechModelDownloadProgress?.percent ?? 0).toFixed(0)}% -{" "}
                  {speechModelDownloadProgress?.downloaded_mb ?? 0} /{" "}
                  {speechModelDownloadProgress?.total_mb ?? 458} MB
                </span>
                <span>
                  {(speechModelDownloadProgress?.speed_mbps ?? 0) > 0
                    ? `${(speechModelDownloadProgress?.speed_mbps ?? 0).toFixed(1)} MB/s`
                    : "Starting..."}
                </span>
              </div>
              <button
                onClick={cancelSpeechModelDownload}
                className="text-[12px] text-[#ba1a1a] hover:underline"
              >
                Cancel download
              </button>
            </div>
          ) : (
            <Row label="Download Status">
              {speechModelFileStatus.exists ? (
                <div className="flex items-center gap-3">
                  <div className="flex items-center gap-1.5">
                    <div className="w-2 h-2 rounded-full bg-[#28CD41]" />
                    <span className="text-[13px] font-medium text-[#28CD41]">
                      Downloaded ({speechModelFileStatus.size_mb} MB)
                    </span>
                  </div>
                  <button
                    onClick={deleteSpeechModelAction}
                    className="text-[12px] text-[#ba1a1a] hover:underline"
                  >
                    Delete
                  </button>
                </div>
              ) : (
                <div className="flex items-center gap-3">
                  <div className="flex items-center gap-1.5">
                    <div className="w-2 h-2 rounded-full bg-[#FF9500]" />
                    <span className="text-[13px] font-medium text-black/50">
                      Not downloaded
                    </span>
                  </div>
                  <button
                    onClick={downloadSpeechModelAction}
                    className="h-6 px-3 bg-[#0058bc] text-white text-[12px] font-medium rounded-md hover:bg-[#004ea8] transition-colors"
                  >
                    Download Package
                  </button>
                </div>
              )}
            </Row>
          )}

          {speechModelFileStatus.exists && !speechModelDownloading && (
            <Row label="Engine Status" isLast={!speechModelError}>
              {speechModelLoaded ? (
                <div className="flex items-center gap-1.5">
                  <div className="w-2 h-2 rounded-full bg-[#28CD41]" />
                  <span className="text-[13px] font-medium text-[#28CD41]">
                    Loaded
                  </span>
                </div>
              ) : speechModelLoading ? (
                <div className="flex items-center gap-1.5">
                  <span className="material-symbols-outlined text-[16px] text-[#0058bc] animate-spin">
                    progress_activity
                  </span>
                  <span className="text-[13px] text-black/50">Loading...</span>
                </div>
              ) : (
                <div className="flex items-center gap-3">
                  <div className="flex items-center gap-1.5">
                    <div className="w-2 h-2 rounded-full bg-black/[0.25]" />
                    <span className="text-[13px] font-medium text-black/50">
                      Not loaded
                    </span>
                  </div>
                  <button
                    onClick={loadSpeechModel}
                    className="h-6 px-3 bg-[#0058bc] text-white text-[12px] font-medium rounded-md hover:bg-[#004ea8] transition-colors"
                  >
                    Load Engine
                  </button>
                </div>
              )}
            </Row>
          )}

          {speechModelError && (
            <div className="px-4 pb-4">
              <p className="text-[11px] text-[#ba1a1a]">{speechModelError}</p>
            </div>
          )}
        </div>
      </section>
    </div>
  );
}
