// Wizard ready step -- summary of choices made before completing setup
import type { PermissionsStatus } from "@/types/commands";

const isMac =
  typeof navigator !== "undefined" && /mac/i.test(navigator.userAgent);

function formatKey(key: string): string {
  if (!isMac) return key;
  switch (key.toLowerCase()) {
    case "alt":
      return "\u2325";
    case "shift":
      return "\u21E7";
    case "meta":
    case "cmd":
      return "\u2318";
    case "ctrl":
    case "control":
      return "\u2303";
    default:
      return key;
  }
}

function parseHotkeyParts(hotkey: string): string[] {
  return hotkey.split("+").map(formatKey);
}

interface SummaryRowProps {
  icon: string;
  label: string;
  value: string;
  tone: "ok" | "warn" | "neutral";
  detail?: string;
}

function SummaryRow({ icon, label, value, tone, detail }: SummaryRowProps) {
  const toneClass =
    tone === "ok"
      ? "text-[#006b19]"
      : tone === "warn"
        ? "text-[#9a5a00]"
        : "text-black/55";

  return (
    <div className="flex items-start gap-3 border-b border-black/[0.06] py-3 last:border-b-0">
      <span
        className={`material-symbols-outlined text-[18px] ${toneClass} mt-[2px] shrink-0`}
      >
        {icon}
      </span>
      <div className="min-w-0 flex-1 text-left">
        <p className="text-[12px] font-semibold uppercase tracking-[0.06em] text-black/[0.40]">
          {label}
        </p>
        <p className="mt-0.5 text-[13px] font-medium text-black/85">{value}</p>
        {detail && (
          <p className="mt-0.5 text-[12px] text-black/50">{detail}</p>
        )}
      </div>
    </div>
  );
}

interface ReadyStepProps {
  hotkey: string;
  permissions: PermissionsStatus | null;
  filesInstalled: boolean;
  onFinish: () => void;
  error: string | null;
}

export function ReadyStep({
  hotkey,
  permissions,
  filesInstalled,
  onFinish,
  error,
}: ReadyStepProps) {
  const hotkeyParts = parseHotkeyParts(hotkey);
  const micGranted = permissions?.microphone === true;

  return (
    <div className="flex flex-col items-center text-center">
      <div className="mb-5 flex h-16 w-16 items-center justify-center rounded-full bg-[#006b19]/[0.08] shadow-[0_0_24px_rgba(40,205,65,0.20)]">
        <span className="material-symbols-outlined text-[32px] text-[#006b19]">
          check_circle
        </span>
      </div>

      <h2 className="mb-2 text-[22px] font-semibold text-black/85">
        You're ready to dictate
      </h2>
      <p className="mb-6 max-w-[360px] text-[14px] leading-relaxed text-black/50">
        Hold your hotkey anywhere on this device and speak. Words appear in the
        focused app the moment you let go.
      </p>

      <div className="mb-6 inline-flex items-center gap-1.5 rounded-[10px] border border-black/[0.07] bg-black/[0.03] px-4 py-2">
        {hotkeyParts.map((part, i) => (
          <span
            key={i}
            className="flex h-7 items-center rounded-[6px] border border-black/[0.06] bg-white px-2.5 font-mono text-[13px] font-medium text-black/85 shadow-sm"
          >
            {part}
          </span>
        ))}
      </div>

      <div className="mb-6 w-full rounded-[10px] border border-black/[0.07] bg-white px-4 py-1 text-left">
        <SummaryRow
          icon="keyboard"
          label="Dictation hotkey"
          value={hotkey}
          tone="ok"
          detail="Hold to dictate. Double-tap to toggle hands-free."
        />
        <SummaryRow
          icon={micGranted ? "mic" : "mic_off"}
          label="Microphone"
          value={micGranted ? "Detected" : "No input device found"}
          tone={micGranted ? "ok" : "warn"}
          detail={
            micGranted
              ? "Default input device is ready."
              : "Add or grant a microphone before dictating."
          }
        />
        <SummaryRow
          icon={filesInstalled ? "check_circle" : "cloud_off"}
          label="Speech engine"
          value={filesInstalled ? "Parakeet installed" : "Not installed yet"}
          tone={filesInstalled ? "ok" : "warn"}
          detail={
            filesInstalled
              ? "Local Parakeet model is ready for offline use."
              : "Open Speech in the app to install the model later."
          }
        />
      </div>

      <button
        type="button"
        onClick={onFinish}
        className="h-9 w-full rounded-[8px] bg-gradient-to-b from-[#0062d0] to-[#0058bc] text-[13px] font-medium text-white transition-all hover:brightness-110 active:brightness-95"
      >
        Start yapping
      </button>

      {error && (
        <p className="mt-4 w-full rounded-lg border border-[#ba1a1a]/15 bg-[#ba1a1a]/[0.06] px-3 py-2 text-[12px] font-medium text-[#ba1a1a]">
          {error}
        </p>
      )}
    </div>
  );
}
