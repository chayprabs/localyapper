// Test dictation -- runs the real pipeline and shows the text and timings
// here. Nothing is pasted or saved.
import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { testDictation } from "@/lib/commands/recording";
import { Button } from "@/components/ui/Button";
import { errorText, Row } from "@/components/settings/controls";
import { cn } from "@/lib/utils";

interface Timings {
  capture_start_ms: number;
  model_ready_ms: number;
  segments: number;
  audio_ms: number;
  decode_ms: number;
  tail_decode_ms: number;
  inject_ms: number;
  release_to_text_ms: number;
}

interface TestResult {
  text: string | null;
  timings: Timings | null;
}

const STAGES: { key: keyof Timings; label: string; unit: string }[] = [
  { key: "capture_start_ms", label: "Microphone start", unit: "ms" },
  { key: "model_ready_ms", label: "Model ready", unit: "ms" },
  { key: "audio_ms", label: "Audio", unit: "ms" },
  { key: "segments", label: "Segments", unit: "" },
  { key: "decode_ms", label: "Decoding", unit: "ms" },
  { key: "tail_decode_ms", label: "Decoding after release", unit: "ms" },
  { key: "release_to_text_ms", label: "Release to text", unit: "ms" },
];

const BARS = 40;
const SILENCE: number[] = Array.from({ length: BARS }, () => 0);

/** The last few input levels as a scrolling row of bars. */
function Waveform({ levels, live }: { levels: number[]; live: boolean }) {
  return (
    <div
      role="img"
      aria-label="Input level"
      className="mt-3 flex h-12 items-center justify-between rounded-2xl bg-ink/[0.04] px-4"
    >
      {levels.map((level, i) => (
        <span
          key={i}
          className={cn(
            "w-[3px] rounded-full transition-[height,background-color] duration-100 ease-out",
            live ? "bg-accent" : "bg-ink/15",
          )}
          style={{ height: `${Math.max(3, Math.min(1, Math.sqrt(Math.max(0, level))) * 32)}px` }}
        />
      ))}
    </div>
  );
}

export function TestDictation() {
  const [recording, setRecording] = useState(false);
  const [waiting, setWaiting] = useState(false);
  const [levels, setLevels] = useState(SILENCE);
  const [result, setResult] = useState<TestResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Leaving the page ends a test that is still recording.
  const recordingRef = useRef(false);
  recordingRef.current = recording;
  useEffect(() => {
    return () => {
      if (recordingRef.current) void testDictation(false);
    };
  }, []);

  useEffect(() => {
    const unLevel = listen<number>("audio-level", (e) => {
      if (recordingRef.current) setLevels((prev) => [...prev.slice(1), e.payload]);
    });
    const unResult = listen<TestResult>("test-dictation-result", (e) => {
      setRecording(false);
      setWaiting(false);
      setLevels(SILENCE);
      setResult(e.payload);
    });
    return () => {
      void unLevel.then((fn) => fn());
      void unResult.then((fn) => fn());
    };
  }, []);

  const toggle = () => {
    setError(null);
    if (recording) {
      setRecording(false);
      setWaiting(true);
      testDictation(false).catch((err: unknown) => {
        setWaiting(false);
        setError(errorText(err));
      });
    } else {
      setResult(null);
      setLevels(SILENCE);
      setRecording(true);
      testDictation(true).catch((err: unknown) => {
        setRecording(false);
        setError(errorText(err));
      });
    }
  };

  return (
    <Row
      label="Test dictation"
      hint="Runs the real pipeline. The text shows here and is not pasted or saved."
      error={error}
      below={
        <>
          <Waveform levels={levels} live={recording} />
          {result && (
            <div className="mt-3 animate-rise-in">
              {result.text === null ? (
                <p className="text-[13px] text-ink/50">Nothing was recognised. The overlay shows the reason.</p>
              ) : (
                <p className="selectable text-[15px] leading-relaxed text-ink/90">{result.text || "(no words)"}</p>
              )}
              {result.timings && (
                <dl className="mt-3 grid grid-cols-2 gap-x-6 gap-y-1 border-t border-ink/[0.06] pt-3 text-[12px] sm:grid-cols-3">
                  {STAGES.map((s) => (
                    <div key={s.key} className="flex justify-between gap-2">
                      <dt className="text-ink/45">{s.label}</dt>
                      <dd className="tabular-nums text-ink/80">
                        {result.timings?.[s.key]}
                        {s.unit && ` ${s.unit}`}
                      </dd>
                    </div>
                  ))}
                </dl>
              )}
            </div>
          )}
        </>
      }
    >
      <Button variant={recording ? "danger" : "primary"} onClick={toggle} disabled={waiting} className="min-w-[72px]">
        {recording ? "Stop" : waiting ? "Working…" : "Start"}
      </Button>
    </Row>
  );
}
