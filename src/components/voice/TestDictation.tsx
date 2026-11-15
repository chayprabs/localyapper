// Test dictation -- runs the real pipeline and shows the text and timings
// here. Nothing is pasted or saved.
import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { testDictation } from "@/lib/commands/recording";
import { Button } from "@/components/ui/Button";
import { errorText, Row } from "@/components/settings/controls";

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
  { key: "capture_start_ms", label: "Mic start", unit: "ms" },
  { key: "model_ready_ms", label: "Model ready", unit: "ms" },
  { key: "audio_ms", label: "Audio", unit: "ms" },
  { key: "segments", label: "Segments", unit: "" },
  { key: "decode_ms", label: "Decode", unit: "ms" },
  { key: "tail_decode_ms", label: "Tail decode", unit: "ms" },
  { key: "release_to_text_ms", label: "Release→text", unit: "ms" },
];

const BARS = 24;
const SILENCE: number[] = Array.from({ length: BARS }, () => 0);

/** The last few input levels as a scrolling row of bars. */
function Waveform({ levels }: { levels: number[] }) {
  return (
    <span role="img" aria-label="Input level" className="flex h-5 animate-fade-in items-center gap-[2px]">
      {levels.map((level, i) => (
        <span
          key={i}
          className="w-[2px] bg-signal transition-[height] duration-100 ease-out"
          style={{ height: `${Math.max(2, Math.min(1, Math.sqrt(Math.max(0, level))) * 20)}px` }}
        />
      ))}
    </span>
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
      label="Test"
      hint="Runs the real pipeline. The text shows here and is not pasted or saved."
      error={error}
      below={
        result && (
          <div className="animate-fade-in pb-1 pt-2">
            {result.text === null ? (
              <p className="text-[13px] text-ink/50">Nothing recognised.</p>
            ) : (
              <p className="selectable text-[13px] leading-5 text-ink/90">{result.text || "(no words)"}</p>
            )}
            {result.timings && (
              <dl className="mt-3 grid grid-cols-2 gap-x-8 gap-y-1 font-mono text-[11.5px] sm:grid-cols-3">
                {STAGES.map((s) => (
                  <div key={s.key} className="flex justify-between gap-2">
                    <dt className="text-ink/40">{s.label}</dt>
                    <dd className="text-ink/80">
                      {result.timings?.[s.key]}
                      {s.unit && ` ${s.unit}`}
                    </dd>
                  </div>
                ))}
              </dl>
            )}
          </div>
        )
      }
    >
      <span className="flex items-center gap-3">
        {recording && <Waveform levels={levels} />}
        <Button variant={recording ? "danger" : "outline"} onClick={toggle} disabled={waiting} className="min-w-[56px]">
          {recording ? "Stop" : waiting ? "…" : "Start"}
        </Button>
      </span>
    </Row>
  );
}
