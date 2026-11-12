// Audio page -- input device, sound cues and a test dictation (plan P5.5)
import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { listInputDevices } from "@/lib/commands/system";
import { testDictation } from "@/lib/commands/recording";
import {
  errorText,
  PageHeader,
  Row,
  Section,
  ToggleSetting,
  useSetting,
} from "@/components/settings/controls";

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

function DeviceSetting() {
  const { value, error, save } = useSetting("input_device");
  const [devices, setDevices] = useState<string[]>([]);
  const [listError, setListError] = useState<string | null>(null);
  useEffect(() => {
    listInputDevices()
      .then(setDevices)
      .catch((err: unknown) => setListError(errorText(err)));
  }, []);
  const missing = value !== null && value !== "default" && !devices.includes(value);
  return (
    <Row
      label="Microphone"
      hint={missing ? "The chosen microphone is not connected; the system default is used." : undefined}
      error={error ?? listError}
    >
      <select
        aria-label="Microphone"
        className="h-8 max-w-[260px] rounded-md border border-black/[0.12] bg-white px-2 text-[13px] text-black/85"
        value={value ?? "default"}
        disabled={value === null}
        onChange={(e) => save(e.target.value)}
      >
        <option value="default">System default</option>
        {missing && value && <option value={value}>{value}</option>}
        {devices.map((d) => (
          <option key={d} value={d}>
            {d}
          </option>
        ))}
      </select>
    </Row>
  );
}

function Meter({ level }: { level: number }) {
  const pct = Math.min(100, Math.round(Math.sqrt(Math.max(0, level)) * 100));
  return (
    <div className="h-2 w-full overflow-hidden rounded-full bg-black/[0.08]" aria-label="Input level">
      <div className="h-full rounded-full bg-[#0058bc] transition-[width] duration-75" style={{ width: `${pct}%` }} />
    </div>
  );
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

function TestDictation() {
  const [recording, setRecording] = useState(false);
  const [waiting, setWaiting] = useState(false);
  const [level, setLevel] = useState(0);
  const [result, setResult] = useState<TestResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const unLevel = listen<number>("audio-level", (e) => setLevel(e.payload));
    const unResult = listen<TestResult>("test-dictation-result", (e) => {
      setRecording(false);
      setWaiting(false);
      setLevel(0);
      setResult(e.payload);
    });
    return () => {
      void unLevel.then((fn) => fn());
      void unResult.then((fn) => fn());
    };
  }, []);

  // Leaving the page ends a test that is still recording.
  const recordingRef = useRef(false);
  recordingRef.current = recording;
  useEffect(() => {
    return () => {
      if (recordingRef.current) void testDictation(false);
    };
  }, []);

  const toggle = () => {
    setError(null);
    if (recording) {
      setRecording(false);
      setWaiting(true);
      testDictation(false).catch((err: unknown) => setError(errorText(err)));
    } else {
      setResult(null);
      setRecording(true);
      testDictation(true).catch((err: unknown) => {
        setRecording(false);
        setError(errorText(err));
      });
    }
  };

  return (
    <div className="px-5 py-4">
      <div className="flex items-center justify-between gap-4">
        <div>
          <div className="text-[13px] font-medium text-black/85">Test dictation</div>
          <div className="text-[12px] text-black/50">
            Runs the real pipeline. The text shows here and is not pasted or saved.
          </div>
        </div>
        <button
          onClick={toggle}
          disabled={waiting}
          className="h-8 shrink-0 rounded-lg bg-[#0058bc] px-4 text-[13px] font-medium text-white disabled:opacity-50"
        >
          {recording ? "Stop" : waiting ? "Working…" : "Start"}
        </button>
      </div>
      <div className="mt-3">
        <Meter level={recording ? level : 0} />
      </div>
      {error && (
        <p role="alert" className="mt-2 text-[12px] font-medium text-[#ba1a1a]">
          {error}
        </p>
      )}
      {result && (
        <div className="mt-3">
          {result.text === null ? (
            <p className="text-[13px] text-black/50">Nothing was recognised. Check the overlay for the reason.</p>
          ) : (
            <p className="rounded-lg bg-black/[0.04] px-3 py-2 text-[13px] text-black/85">
              {result.text || "(no words)"}
            </p>
          )}
          {result.timings && (
            <dl className="mt-3 grid grid-cols-2 gap-x-6 gap-y-1 text-[12px] sm:grid-cols-3">
              {STAGES.map((s) => (
                <div key={s.key} className="flex justify-between gap-2">
                  <dt className="text-black/50">{s.label}</dt>
                  <dd className="tabular-nums text-black/85">
                    {result.timings?.[s.key]}
                    {s.unit && ` ${s.unit}`}
                  </dd>
                </div>
              ))}
            </dl>
          )}
        </div>
      )}
    </div>
  );
}

export function AudioPage() {
  return (
    <div className="px-6 py-8 sm:px-12 sm:py-10">
      <PageHeader title="Audio" subtitle="Audio stays in memory and is never written to disk." />
      <Section title="Input">
        <DeviceSetting />
        <ToggleSetting k="sound_cues" label="Sound cues" hint="A short sound when recording starts and stops." />
      </Section>
      <Section title="Try it">
        <TestDictation />
      </Section>
    </div>
  );
}
