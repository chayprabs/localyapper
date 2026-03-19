import { useState, useCallback } from "react";
import { useOverlayState } from "@/hooks/useOverlayState";
import { Waveform } from "./Waveform";
import { CountdownTimer } from "./CountdownTimer";
import { YappingEmoji } from "./YappingEmoji";

const MAX_RECORDING_SECONDS = 120;

function Spinner({ slow }: { slow?: boolean }) {
  const speed = slow ? "animate-spin-slow" : "animate-spin";
  return (
    <div
      className={`w-[16px] h-[16px] border-[2px] border-black/[0.10] border-t-black/30 rounded-full ${speed}`}
    />
  );
}

function CopyButton({ text }: { text: string }) {
  const [copied, setCopied] = useState(false);

  const handleCopy = useCallback(async () => {
    await navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  }, [text]);

  return (
    <button
      onClick={handleCopy}
      className="text-[11px] font-semibold text-primary hover:text-[#004ea8] transition-colors"
    >
      {copied ? "\u2713" : "Copy"}
    </button>
  );
}

export function Overlay() {
  const { overlayData, elapsedSeconds, remainingSeconds, autoInjectProgress } =
    useOverlayState();
  const { visualState, text, durationMs } = overlayData;

  if (visualState === "hidden") {
    return <div className="h-screen w-screen bg-transparent" />;
  }

  const pillBase =
    "bg-white/95 border border-black/[0.10] shadow-[0_8px_32px_rgba(0,0,0,0.16)] backdrop-blur-md px-8";

  const isTranscribed = visualState === "transcribed";
  const isLong = text != null && text.length > 40;
  const pillHeight = isTranscribed || visualState === "long-recording" ? "h-[72px]" : "h-[64px]";
  const pillRadius = isTranscribed ? "rounded-[36px]" : "rounded-full";

  return (
    <div className="h-screen w-screen flex items-center justify-center bg-transparent">
      <div
        className={`w-[320px] ${pillHeight} ${pillRadius} ${pillBase} relative overflow-hidden`}
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
            <CountdownTimer mode="elapsed" seconds={elapsedSeconds} />
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

        {isTranscribed && text != null && (
          <>
            {!isLong ? (
              <>
                <div className="absolute inset-y-0 right-5 flex items-center z-10">
                  <CopyButton text={text} />
                </div>
                <div className="flex items-center h-full px-6">
                  <span className="text-[13px] font-medium text-black/85 truncate pr-12">
                    {text}
                  </span>
                </div>
              </>
            ) : (
              <>
                <div className="absolute inset-y-0 right-6 flex items-center z-10">
                  <CopyButton text={text} />
                </div>
                <div className="flex items-center h-full px-6">
                  <span className="text-[13px] font-medium text-black/85 line-clamp-2 leading-tight pr-8">
                    {text}
                  </span>
                </div>
              </>
            )}
            <div className="absolute bottom-0 left-0 w-full h-[2px] bg-black/[0.05]">
              <div
                className="h-full bg-primary transition-none"
                style={{ width: `${autoInjectProgress * 100}%` }}
              />
            </div>
          </>
        )}
      </div>
    </div>
  );
}
