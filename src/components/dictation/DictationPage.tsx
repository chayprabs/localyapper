// Dictation page -- hotkeys, recording behaviour, output and the overlay
import { useEffect, useState } from "react";
import { HotkeysSection } from "@/components/hotkeys/HotkeysSection";
import { isMacPlatform } from "@/lib/hotkeyDefaults";
import {
  Notice,
  NumberSetting,
  Page,
  Section,
  SegmentedSetting,
  SelectSetting,
  ToggleSetting,
  useSetting,
} from "@/components/settings/controls";

const paste = isMacPlatform ? "Cmd" : "Ctrl";

const PASTE_METHODS = [
  { value: "auto", label: "Automatic" },
  { value: "ctrl_v", label: `${paste}+V` },
  { value: "ctrl_shift_v", label: `${paste}+Shift+V` },
  { value: "shift_insert", label: "Shift+Insert" },
  { value: "type", label: "Typing" },
  { value: "clipboard_only", label: "Copy only" },
];

function Slider({
  label,
  value,
  onInput,
  onCommit,
}: {
  label: string;
  value: number;
  onInput: (n: number) => void;
  onCommit: (n: number) => void;
}) {
  return (
    <label className="flex items-center gap-3 text-[12px] text-ink/50">
      <span className="w-8 shrink-0">{label}</span>
      <input
        type="range"
        min={0}
        max={100}
        value={value}
        aria-label={label}
        className="h-1 min-w-0 flex-1 cursor-pointer accent-[rgb(var(--ink))]"
        onChange={(e) => onInput(Number(e.target.value))}
        onPointerUp={(e) => onCommit(Number((e.target as HTMLInputElement).value))}
        onKeyUp={(e) => onCommit(Number((e.target as HTMLInputElement).value))}
        // A drag released outside the slider may send no pointerup to it.
        onBlur={(e) => onCommit(Number(e.target.value))}
      />
      <span className="w-9 shrink-0 text-right font-mono text-ink/80">{value}%</span>
    </label>
  );
}

/** Where the pill goes when the position is "custom": two percentages,
 *  with a small picture of the screen showing the result. */
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
  const commit = (nx: number, ny: number) => {
    // Nothing to save when a slider was touched but not moved; saving would
    // also round away the exact spot the pill was dragged to.
    const [a, b] = (value ?? "").split(",").map((v) => Math.round(Number(v) * 100));
    if (a === nx && b === ny) return;
    save(`${(nx / 100).toFixed(4)},${(ny / 100).toFixed(4)}`);
  };
  return (
    <div className="flex animate-fade-in items-center gap-5 py-3">
      <div aria-hidden="true" className="relative h-[60px] w-[96px] shrink-0 rounded-[4px] border border-ink/25">
        <span
          className="absolute h-1 w-4 -translate-x-1/2 -translate-y-1/2 rounded-full bg-ink"
          style={{ left: `${10 + x * 0.8}%`, top: `${10 + y * 0.8}%` }}
        />
      </div>
      <div className="flex min-w-0 flex-1 flex-col gap-3">
        <Slider label="Left" value={x} onInput={setX} onCommit={(n) => commit(n, y)} />
        <Slider label="Top" value={y} onInput={setY} onCommit={(n) => commit(x, n)} />
        {error && <Notice>{error}</Notice>}
      </div>
    </div>
  );
}

export function DictationPage() {
  return (
    <Page>
      <HotkeysSection />

      <Section title="Recording">
        <SelectSetting
          k="activation_mode"
          label="Record key"
          hint="Hold to talk, press once to start and again to stop, or hold with a double-tap for hands-free."
          options={[
            { value: "hybrid", label: "Hold or double-tap" },
            { value: "hold", label: "Hold" },
            { value: "toggle", label: "Toggle" },
          ]}
        />
        <ToggleSetting k="cancel_key_enabled" label="Cancel key" hint="The cancel hotkey discards a recording." />
        <ToggleSetting
          k="cancel_key_in_hands_free"
          label="Cancel key in hands-free"
          hint="Also listen for the cancel key while recording hands-free."
        />
        <NumberSetting
          k="max_recording_seconds"
          label="Max length"
          hint="Recording stops on its own after this long (30 to 3600 seconds)."
          min={30}
          max={3600}
          unit="s"
        />
        <ToggleSetting k="trailing_space" label="Trailing space" hint="Add a space after the text." />
      </Section>

      <Section title="Output">
        <SelectSetting k="paste_method" label="Paste with" options={PASTE_METHODS} />
        <ToggleSetting
          k="restore_clipboard"
          label="Restore clipboard"
          hint="Put back what was on the clipboard after pasting."
        />
        <NumberSetting
          k="clipboard_restore_delay_ms"
          label="Restore delay"
          hint="Some apps read the clipboard late (100 to 2000 ms)."
          min={100}
          max={2000}
          unit="ms"
        />
      </Section>

      <Section title="Overlay">
        <ToggleSetting k="overlay_enabled" label="Show" hint="A small pill while you dictate." />
        <SegmentedSetting
          k="overlay_position"
          label="Position"
          hint="You can also drag the pill while it is showing."
          options={[
            { value: "bottom_center", label: "Bottom" },
            { value: "top_center", label: "Top" },
            { value: "custom", label: "Custom" },
          ]}
        />
        <CustomPosition />
        <ToggleSetting k="overlay_preview" label="Live text" hint="Show text as it is recognised." />
      </Section>
    </Page>
  );
}
