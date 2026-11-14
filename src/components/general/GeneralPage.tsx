// General page -- launch at login, theme, privacy and About
import { useEffect, useState } from "react";
import { aboutInfo, diagnostics, openReleases, type AboutInfo } from "@/lib/commands/system";
import { resetAllSettings } from "@/lib/commands/settings";
import { Button } from "@/components/ui/Button";
import { CopyButton } from "@/components/ui/CopyButton";
import {
  errorText,
  Page,
  PageHeader,
  Row,
  Section,
  SegmentedSetting,
  SelectSetting,
  ToggleSetting,
} from "@/components/settings/controls";

const NOTICES = [
  "Parakeet speech models: NVIDIA, CC-BY-4.0",
  "Moonshine speech models: Useful Sensors, MIT",
  "Silero VAD: Silero Team, MIT",
  "sherpa-onnx: Xiaomi / k2-fsa, Apache-2.0",
  "ONNX Runtime: Microsoft, MIT",
  "Test recordings: LibriSpeech, CC-BY-4.0",
];

function PathRow({ label, path }: { label: string; path: string }) {
  return (
    <Row label={label} hint={<span className="selectable break-all font-mono text-[11px]">{path}</span>}>
      <CopyButton text={path} label="Copy path" className="-mr-2" />
    </Row>
  );
}

function About() {
  const [info, setInfo] = useState<AboutInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const [confirmReset, setConfirmReset] = useState(false);
  const [reset, setReset] = useState<{ ok: boolean; text: string } | null>(null);

  useEffect(() => {
    aboutInfo()
      .then(setInfo)
      .catch((err: unknown) => setError(errorText(err)));
  }, []);

  const copyDiagnostics = () => {
    setError(null);
    diagnostics()
      .then((text) => navigator.clipboard.writeText(text))
      .then(() => {
        setCopied(true);
        window.setTimeout(() => setCopied(false), 1500);
      })
      .catch((err: unknown) => setError(errorText(err)));
  };

  const resetAll = () => {
    setConfirmReset(false);
    setReset(null);
    resetAllSettings()
      .then(() => setReset({ ok: true, text: "Settings are back to their defaults." }))
      .catch((err: unknown) => setReset({ ok: false, text: errorText(err) }));
  };

  return (
    <Section title="About">
      <Row label="LocalYapper" hint={info ? `Version ${info.version}` : undefined} error={error}>
        <span className="flex gap-2">
          <Button onClick={() => openReleases().catch((err: unknown) => setError(errorText(err)))}>Releases</Button>
          <Button onClick={copyDiagnostics} className="min-w-[124px]">
            {copied ? "Copied" : "Copy diagnostics"}
          </Button>
        </span>
      </Row>
      {info && <PathRow label="Data folder" path={info.data_dir} />}
      {info && <PathRow label="Log folder" path={info.log_dir} />}
      <Row
        label="Reset all settings"
        hint={reset?.ok ? reset.text : "Hotkeys, audio, output and overlay go back to their defaults."}
        error={reset && !reset.ok ? reset.text : null}
      >
        {confirmReset ? (
          <span className="flex animate-fade-in gap-2">
            <Button onClick={() => setConfirmReset(false)}>Cancel</Button>
            <Button variant="danger" onClick={resetAll}>
              Reset
            </Button>
          </span>
        ) : (
          <Button onClick={() => setConfirmReset(true)}>Reset…</Button>
        )}
      </Row>
      <details className="group px-4 py-3">
        <summary className="flex min-h-8 cursor-pointer list-none items-center justify-between text-[13px] font-medium text-ink/90 [&::-webkit-details-marker]:hidden">
          Notices
          <span className="text-[12px] font-normal text-ink/45 group-open:hidden">Show</span>
          <span className="hidden text-[12px] font-normal text-ink/45 group-open:inline">Hide</span>
        </summary>
        <ul className="mt-1 animate-fade-in space-y-1 pb-1 text-[12px] text-ink/50">
          {NOTICES.map((n) => (
            <li key={n}>{n}</li>
          ))}
        </ul>
      </details>
    </Section>
  );
}

export function GeneralPage() {
  return (
    <Page>
      <PageHeader title="General" />
      <Section title="App">
        <ToggleSetting k="auto_start" label="Launch at login" hint="Start in the tray when you sign in." />
        <SegmentedSetting
          k="theme"
          label="Appearance"
          options={[
            { value: "system", label: "Auto" },
            { value: "light", label: "Light" },
            { value: "dark", label: "Dark" },
          ]}
        />
      </Section>
      <Section title="Privacy">
        <ToggleSetting k="history_enabled" label="Keep a history" hint="Dictated text is stored on this computer only." />
        <SelectSetting
          k="history_retention_days"
          label="Keep history for"
          options={[
            { value: "0", label: "Forever" },
            { value: "90", label: "90 days" },
            { value: "30", label: "30 days" },
            { value: "7", label: "7 days" },
          ]}
        />
      </Section>
      <About />
    </Page>
  );
}
