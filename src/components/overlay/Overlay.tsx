// Floating overlay pill -- displays recording, processing, and transcription states
import type { PointerEvent as ReactPointerEvent } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useOverlayState } from "@/hooks/useOverlayState";
import { CopyButton } from "@/components/ui/CopyButton";
import { Waveform } from "./Waveform";
import { CountdownTimer } from "./CountdownTimer";
import { YappingEmoji } from "./YappingEmoji";

/** Hard cap on single recording duration — matches backend MAX_RECORDING_SAMPLES. */
const MAX_RECORDING_SECONDS = 120;

/** CSS spinner used during processing and long-recording states. */
function Spinner({ slow }: { slow?: boolean }) {
  const speed = slow ? "animate-spin-slow" : "animate-spin";
  return (
    <div
      className={`w-[16px] h-[16px] border-[2px] border-black/[0.10] border-t-black/30 rounded-full ${speed}`}
    />
  );
}

function isInteractiveTarget(target: EventTarget | null) {
  return target instanceof Element
    && target.closest("button, a, input, textarea, select, [role='button'], [data-overlay-no-drag='true']") !== null;
}

export function Overlay() {
  const {
    overlayData,
    elapsedSeconds,
    remainingSeconds,
    transcribedDisplayProgress,
    processingCountdown,
    dismissOverlay,
  } = useOverlayState();
  const { visualState, text, durationMs, error } = overlayData;

  if (visualState === "hidden") {
    return <div className="h-screen w-screen bg-transparent" />;
  }

  const pillBase =
    "bg-white/95 border border-black/[0.10] shadow-[0_8px_32px_rgba(0,0,0,0.16)] backdrop-blur-md px-8";

  const isTranscribed = visualState === "transcribed";
  const isLong = text != null && text.length > 40;
  const pillHeight =
    isTranscribed || visualState === "long-recording" || visualState === "error"
      ? "h-[72px]"
      : "h-[64px]";
  const pillRadius = isTranscribed ? "rounded-[36px]" : "rounded-full";

  const handlePointerDown = async (event: ReactPointerEvent<HTMLDivElement>) => {
    if (event.button !== 0 || isInteractiveTarget(event.target)) {
      return;
    }

    try {
      await getCurrentWindow().startDragging();
    } catch (error) {
      console.error("[overlay] Failed to start dragging:", error);
    }
  };

  return (
    <div className="h-screen w-screen flex items-center justify-center bg-transparent">
      <div
        onPointerDown={handlePointerDown}
        className={`w-[320px] ${pillHeight} ${pillRadius} ${pillBase} relative overflow-hidden cursor-grab active:cursor-grabbing select-none`}
      >
        {visualState === "listening" && (
          <div className="flex items-center justify-between h-full">
            <YappingEmoji />
            <Waveform color="blue" />
            <span className="text-[12px] font-medium text-black/35 tracking-tight">
              Listening...
            </span>
          </div>
        )}

        {visualState === "stopping-soon" && (
          <div className="flex items-center justify-between h-full">
            <YappingEmoji />
            <Waveform color="red" />
            <CountdownTimer mode="countdown" seconds={remainingSeconds} />
            <div
              className="absolute bottom-0 left-0 h-[2px] bg-destructive"
              style={{
                width: `${(remainingSeconds / (MAX_RECORDING_SECONDS - 105)) * 100}%`,
              }}
            />
          </div>
        )}

        {visualState === "processing" && (
          <div className="flex items-center justify-between h-full">
            <Spinner />
            {processingCountdown !== null && processingCountdown > 0 ? (
              <span className="text-[14px] font-semibold text-black/60 tabular-nums">
                {processingCountdown.toFixed(1)}s
              </span>
            ) : processingCountdown === 0 ? (
              <span className="text-[12px] font-medium text-black/35 tracking-tight">
                Almost done...
              </span>
            ) : (
              <CountdownTimer mode="elapsed" seconds={elapsedSeconds} />
            )}
            <span className="text-[12px] font-medium text-black/35 tracking-tight">
              Processing...
            </span>
          </div>
        )}

        {visualState === "long-recording" && (
          <div className="flex items-center justify-between h-full">
            <Spinner slow />
            <div className="flex flex-col items-center">
              <span className="text-[17px] font-semibold text-black/85">
                {elapsedSeconds.toFixed(1)}s
              </span>
              <span className="text-[11px] text-black/40">
                {durationMs != null
                  ? `${Math.round(durationMs / 60000)} min recording`
                  : `${Math.floor(elapsedSeconds / 60)} min recording`}
              </span>
            </div>
            <span className="text-[12px] font-medium text-black/35 tracking-tight">
              Processing...
            </span>
          </div>
        )}

        {visualState === "no-speech" && (
          <div className="flex items-center justify-center h-full">
            <span className="text-[13px] font-medium text-black/40 tracking-tight">
              No speech detected
            </span>
          </div>
        )}

        {visualState === "error" && (
          <div className="flex items-center gap-3 h-full">
            <span className="material-symbols-outlined text-[20px] text-destructive shrink-0">
              error
            </span>
            <div className="min-w-0">
              <p className="text-[11px] font-semibold text-destructive">
                Dictation failed
              </p>
              <p className="text-[12px] font-medium text-black/65 line-clamp-2 leading-tight">
                {error ?? "Something went wrong"}
              </p>
            </div>
          </div>
        )}

        {isTranscribed && text != null && (
          <>
            {!isLong ? (
              <>
                <div className="absolute inset-y-0 right-5 flex items-center z-10">
                  <CopyButton
                    text={text}
                    variant="text"
                    onAfterCopy={dismissOverlay}
                    className="text-[11px] font-semibold text-primary hover:text-[#004ea8] transition-colors"
                  />
                </div>
                <div className="flex items-center h-full px-6">
                  {error && (
                    <span className="absolute top-2 left-6 text-[10px] font-semibold text-destructive">
                      Paste failed
                    </span>
                  )}
                  <span className={`text-[13px] font-medium text-black/85 truncate pr-12 ${error ? "pt-3" : ""}`}>
                    {text}
                  </span>
                </div>
              </>
            ) : (
              <>
                <div className="absolute inset-y-0 right-6 flex items-center z-10">
                  <CopyButton
                    text={text}
                    variant="text"
                    onAfterCopy={dismissOverlay}
                    className="text-[11px] font-semibold text-primary hover:text-[#004ea8] transition-colors"
                  />
                </div>
                <div className="flex items-center h-full px-6">
                  {error && (
                    <span className="absolute top-2 left-6 text-[10px] font-semibold text-destructive">
                      Paste failed
                    </span>
                  )}
                  <span className={`text-[13px] font-medium text-black/85 line-clamp-2 leading-tight pr-8 ${error ? "pt-3" : ""}`}>
                    {text}
                  </span>
                </div>
              </>
            )}
            {!error && (
              <div className="absolute bottom-0 left-0 w-full h-[2px] bg-black/[0.05]">
                <div
                  className="h-full bg-primary transition-none"
                  style={{ width: `${transcribedDisplayProgress * 100}%` }}
                />
              </div>
            )}
          </>
        )}
      </div>
    </div>
  );
}
