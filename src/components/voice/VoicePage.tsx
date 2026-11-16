// Voice page -- microphone, a test dictation and the speech models
import { useEffect, useState } from "react";
import { listInputDevices } from "@/lib/commands/system";
import {
  errorText,
  Page,
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
  // null until the list arrives, so a saved microphone is not called
  // missing while it loads.
  const [devices, setDevices] = useState<string[] | null>(null);
  const [listError, setListError] = useState<string | null>(null);
  useEffect(() => {
    let live = true;
    listInputDevices()
      // ALSA names its default device "default"; "System default" covers it.
      .then((list) => live && setDevices(list.filter((d) => d !== "default")))
      .catch((err: unknown) => live && setListError(errorText(err)));
    return () => {
      live = false;
    };
  }, []);
  const known = devices ?? [];
  const unlisted = value !== null && value !== "default" && !known.includes(value);
  const missing = unlisted && devices !== null;
  return (
    <Row
      label="Input"
      detail={missing ? "Not connected, using the default" : undefined}
      error={error ?? listError}
    >
      <Select label="Microphone" value={value ?? "default"} disabled={value === null} onChange={save}>
        <option value="default">System default</option>
        {unlisted && value && <option value={value}>{value}</option>}
        {known.map((d) => (
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
      <Section title="Microphone">
        <DeviceSetting />
        <ToggleSetting k="sound_cues" label="Sound cues" hint="A short sound when recording starts and stops." />
        <TestDictation />
      </Section>

      <Models />

      <Section title="Memory">
        <SelectSetting
          k="idle_unload_seconds"
          label="Unload model after"
          hint="The model is unloaded after this long without dictation."
          options={[
            { value: "30", label: "30 seconds" },
            { value: "120", label: "2 minutes" },
            { value: "300", label: "5 minutes" },
            { value: "900", label: "15 minutes" },
            { value: "0", label: "Never" },
          ]}
        />
      </Section>
    </Page>
  );
}
