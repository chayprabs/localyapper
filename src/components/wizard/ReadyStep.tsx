// Wizard ready step -- summary of choices made before completing setup
import { useState } from "react";
import type { PermissionsStatus } from "@/types/commands";
import { Icon, type IconName } from "@/components/ui/Icon";

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
  icon: IconName;
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
    <div className="flex items-start gap-3 border-b border-black/[0.06] py-2 last:border-b-0">
      <Icon name={icon} size={18} className={`${toneClass} mt-[2px] shrink-0`} />
      <div className="min-w-0 flex-1 text-left">
        <p className="text-[13px] text-black/85">
          <span className="text-black/50">{label}: </span>
          <span className="font-medium">{value}</span>
        </p>
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
  onFinish: (launchAtLogin: boolean) => void;
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
  const [launchAtLogin, setLaunchAtLogin] = useState(false);
  const micGranted = permissions?.microphone === true;

  return (
    <div className="flex flex-col items-center text-center">
      <div className="mb-3 flex h-12 w-12 items-center justify-center rounded-full bg-[#006b19]/[0.08] shadow-[0_0_24px_rgba(40,205,65,0.20)]">
        <Icon name="check_circle" size={26} className="text-[#006b19]" />
      </div>

      <h2 className="mb-1.5 text-[22px] font-semibold text-black/85">
        You're ready to dictate
      </h2>
      <p className="mb-4 max-w-[380px] text-[13px] leading-relaxed text-black/50">
        Hold your hotkey in any app and speak. The words appear where your
        cursor is when you let go.
      </p>

      <div className="mb-4 w-full rounded-[10px] border border-black/[0.07] bg-surface px-4 py-1 text-left">
        <div className="flex items-start gap-3 border-b border-black/[0.06] py-2">
          <Icon name="keyboard" size={18} className="mt-[2px] shrink-0 text-[#006b19]" />
          <div className="min-w-0 flex-1 text-left">
            <p className="flex flex-wrap items-center gap-1.5 text-[13px] text-black/50">
              Hotkey:
              {hotkeyParts.map((part, i) => (
                <span
                  key={i}
                  className="flex h-6 items-center rounded-[6px] border border-black/[0.08] bg-black/[0.03] px-2 font-mono text-[12px] font-medium text-black/85"
                >
                  {part}
                </span>
              ))}
            </p>
            <p className="mt-0.5 text-[12px] text-black/50">
              Hold to dictate. Double-tap to keep recording hands-free.
            </p>
          </div>
        </div>
        <SummaryRow
          icon={micGranted ? "mic" : "mic_off"}
          label="Microphone"
          value={micGranted ? "Ready" : "Not found"}
          tone={micGranted ? "ok" : "warn"}
          detail={
            micGranted
              ? undefined
              : "Connect or allow a microphone before dictating."
          }
        />
        <SummaryRow
          icon={filesInstalled ? "check_circle" : "cloud_off"}
          label="Speech model"
          value={filesInstalled ? "Installed" : "Not installed yet"}
          tone={filesInstalled ? "ok" : "warn"}
          detail={
            filesInstalled
              ? undefined
              : "Dictation will not work until it is downloaded from the Speech page."
          }
        />
      </div>

      <label className="mb-4 flex w-full items-center gap-2 text-left text-[13px] text-black/70">
        <input
          type="checkbox"
          checked={launchAtLogin}
          onChange={(e) => setLaunchAtLogin(e.target.checked)}
        />
        Launch LocalYapper when I sign in
      </label>

      <button
        type="button"
        onClick={() => onFinish(launchAtLogin)}
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
