// General page -- launch at login, theme, privacy and About (plan P5.6)
import { useEffect, useState } from "react";
import { aboutInfo, diagnostics, type AboutInfo } from "@/lib/commands/system";
import { resetAllSettings } from "@/lib/commands/settings";
import { CopyButton } from "@/components/ui/CopyButton";
import {
  errorText,
  PageHeader,
  Row,
  Section,
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
    <Row label={label} hint={path}>
      <CopyButton
        text={path}
        variant="icon"
        iconSize={16}
        className="flex h-8 w-8 items-center justify-center rounded-md text-black/40 hover:bg-black/[0.04] hover:text-black/85"
      />
    </Row>
  );
}

function About() {
  const [info, setInfo] = useState<AboutInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const [confirmReset, setConfirmReset] = useState(false);
  const [resetNote, setResetNote] = useState<string | null>(null);

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

  const reset = () => {
    setConfirmReset(false);
    setResetNote(null);
    resetAllSettings()
      .then(() => setResetNote("Settings restored to their defaults."))
      .catch((err: unknown) => setResetNote(errorText(err)));
  };

  return (
    <Section title="About">
      <Row label="LocalYapper" hint={info ? `Version ${info.version}` : undefined} error={error}>
        <button
          onClick={copyDiagnostics}
          className="h-8 rounded-lg border border-black/[0.15] bg-surface px-3 text-[13px] font-medium text-black/85 hover:bg-black/[0.02]"
        >
          {copied ? "Copied" : "Copy diagnostics"}
        </button>
      </Row>
      {info && <PathRow label="Data folder" path={info.data_dir} />}
      {info && <PathRow label="Log folder" path={info.log_dir} />}
      <div className="px-5 py-3">
        <div className="text-[13px] font-medium text-black/85">Notices</div>
        <ul className="mt-1 list-disc pl-5 text-[12px] text-black/50">
          {NOTICES.map((n) => (
            <li key={n}>{n}</li>
          ))}
        </ul>
      </div>
      <Row label="Reset all settings" hint="Hotkeys, audio, output and overlay go back to their defaults." error={resetNote}>
        {confirmReset ? (
          <span className="flex gap-2">
            <button
              onClick={() => setConfirmReset(false)}
              className="h-8 rounded-lg border border-black/[0.15] bg-surface px-3 text-[13px] font-medium text-black/85"
            >
              Cancel
            </button>
            <button onClick={reset} className="h-8 rounded-lg bg-[#ba1a1a] px-3 text-[13px] font-medium text-white">
              Reset
            </button>
          </span>
        ) : (
          <button
            onClick={() => setConfirmReset(true)}
            className="h-8 rounded-lg border border-black/[0.15] bg-surface px-3 text-[13px] font-medium text-[#ba1a1a]"
          >
            Reset…
          </button>
        )}
      </Row>
    </Section>
  );
}

export function GeneralPage() {
  return (
    <div className="px-6 py-8 sm:px-12 sm:py-10">
      <PageHeader title="General" />
      <Section title="App">
        <ToggleSetting k="auto_start" label="Launch at login" hint="Start in the tray when you sign in." />
        <SelectSetting
          k="theme"
          label="Appearance"
          options={[
            { value: "system", label: "Match the system" },
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
    </div>
  );
}
