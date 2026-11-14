// Voice page -- microphone, a test dictation and the speech models
import { useEffect, useState } from "react";
import { listInputDevices } from "@/lib/commands/system";
import {
  errorText,
  Page,
  PageHeader,
  Row,
  Section,
  Select,
  SelectSetting,
  ToggleSetting,
  useSetting,
} from "@/components/settings/controls";
import { Models } from "./Models";
import { TestDictation } from "./TestDictation";

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
      <Select label="Microphone" value={value ?? "default"} disabled={value === null} onChange={save}>
        <option value="default">System default</option>
        {missing && value && <option value={value}>{value}</option>}
        {devices.map((d) => (
          <option key={d} value={d}>
            {d}
          </option>
        ))}
      </Select>
    </Row>
  );
}

export function VoicePage() {
  return (
    <Page>
      <PageHeader title="Voice" subtitle="Speech is recognised on this computer. Audio is never written to disk." />

      <Section title="Input">
        <DeviceSetting />
        <ToggleSetting k="sound_cues" label="Sound cues" hint="A short sound when recording starts and stops." />
        <TestDictation />
      </Section>

      <Models />

      <Section title="Memory">
        <SelectSetting
          k="idle_unload_seconds"
          label="Free memory when idle"
          hint="The model is unloaded after this long without dictation."
          options={[
            { value: "30", label: "After 30 seconds" },
            { value: "120", label: "After 2 minutes" },
            { value: "300", label: "After 5 minutes" },
            { value: "900", label: "After 15 minutes" },
            { value: "0", label: "Never" },
          ]}
        />
      </Section>
    </Page>
  );
}
