// Dictation page -- hotkeys, recording behaviour, output and the overlay
import { useEffect, useState } from "react";
import { HotkeysSection } from "@/components/hotkeys/HotkeysSection";
import {
  NumberSetting,
  PageHeader,
  Row,
  Section,
  SelectSetting,
  ToggleSetting,
  useSetting,
} from "@/components/settings/controls";

const isMac = typeof navigator !== "undefined" && /mac/i.test(navigator.userAgent);
const paste = isMac ? "Cmd" : "Ctrl";

const PASTE_METHODS = [
  { value: "auto", label: "Automatic" },
  { value: "ctrl_v", label: `${paste}+V` },
  { value: "ctrl_shift_v", label: `${paste}+Shift+V (terminals)` },
  { value: "shift_insert", label: "Shift+Insert" },
  { value: "type", label: "Type the text" },
  { value: "clipboard_only", label: "Copy only, do not paste" },
];

/** Where the pill goes when the position is "custom": two percentages. */
function CustomPosition() {
  const position = useSetting("overlay_position");
  const { value, error, save } = useSetting("overlay_custom");
  const [x, setX] = useState(50);
  const [y, setY] = useState(85);
  useEffect(() => {
    if (!value) return;
    const [a, b] = value.split(",").map((v) => Math.round(Number(v) * 100));
    if (a !== undefined && Number.isFinite(a)) setX(a);
    if (b !== undefined && Number.isFinite(b)) setY(b);
  }, [value]);
  if (position.value !== "custom") return null;
  const commit = (nx: number, ny: number) => save(`${(nx / 100).toFixed(2)},${(ny / 100).toFixed(2)}`);
  const slider = (label: string, v: number, set: (n: number) => void, done: (n: number) => void) => (
    <label className="flex items-center gap-3 text-[12px] text-black/50">
      <span className="w-20">{label}</span>
      <input
        type="range"
        min={0}
        max={100}
        value={v}
        aria-label={label}
        onChange={(e) => set(Number(e.target.value))}
        onPointerUp={(e) => done(Number((e.target as HTMLInputElement).value))}
        onKeyUp={(e) => done(Number((e.target as HTMLInputElement).value))}
      />
      <span className="w-10 text-right tabular-nums">{v}%</span>
    </label>
  );
  return (
    <Row label="Custom position" hint="Where the centre of the pill sits on the screen." error={error}>
      <div className="flex flex-col gap-1">
        {slider("From left", x, setX, (n) => commit(n, y))}
        {slider("From top", y, setY, (n) => commit(x, n))}
      </div>
    </Row>
  );
}

export function DictationPage() {
  return (
    <div className="px-6 py-8 sm:px-12 sm:py-10">
      <PageHeader title="Dictation" subtitle="How you start and stop dictation, and where the text goes." />

      <HotkeysSection />

      <Section title="Recording">
        <SelectSetting
          k="activation_mode"
          label="Record key"
          hint="Hold to talk, press to start and stop, or both."
          options={[
            { value: "hybrid", label: "Hold, or tap for hands-free" },
            { value: "hold", label: "Hold to talk" },
            { value: "toggle", label: "Press to start and stop" },
          ]}
        />
        <ToggleSetting k="cancel_key_enabled" label="Cancel key" hint="The cancel hotkey discards a recording." />
        <ToggleSetting
          k="cancel_key_in_hands_free"
          label="Cancel key in hands-free mode"
          hint="Also listen for the cancel key while recording hands-free."
        />
        <NumberSetting
          k="max_recording_seconds"
          label="Longest recording"
          hint="Recording stops on its own after this long (30 to 3600)."
          min={30}
          max={3600}
          unit="seconds"
        />
        <ToggleSetting k="trailing_space" label="Add a space after the text" />
      </Section>

      <Section title="Output">
        <SelectSetting k="paste_method" label="Paste with" options={PASTE_METHODS} />
        <ToggleSetting
          k="restore_clipboard"
          label="Restore the clipboard"
          hint="Put back what was on the clipboard after pasting."
        />
        <NumberSetting
          k="clipboard_restore_delay_ms"
          label="Restore after"
          hint="Some apps read the clipboard late (100 to 2000)."
          min={100}
          max={2000}
          unit="ms"
        />
      </Section>

      <Section title="Overlay">
        <ToggleSetting k="overlay_enabled" label="Show the overlay" hint="A small pill while you dictate." />
        <SelectSetting
          k="overlay_position"
          label="Position"
          options={[
            { value: "bottom_center", label: "Bottom centre" },
            { value: "top_center", label: "Top centre" },
            { value: "custom", label: "Custom" },
          ]}
        />
        <CustomPosition />
        <ToggleSetting k="overlay_preview" label="Show text as it is recognised" />
      </Section>
    </div>
  );
}
