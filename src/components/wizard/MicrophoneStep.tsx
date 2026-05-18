// Wizard microphone step -- request permission and verify a default input device
import { useEffect } from "react";
import type { PermissionsStatus } from "@/types/commands";
import { Icon, type IconName } from "@/components/ui/Icon";

interface MicrophoneStepProps {
  permissions: PermissionsStatus | null;
  loading: boolean;
  refresh: () => void;
  openSettings: () => void;
  onContinue: () => void;
}

export function MicrophoneStep({
  permissions,
  loading,
  refresh,
  openSettings,
  onContinue,
}: MicrophoneStepProps) {
  useEffect(() => {
    refresh();
  }, [refresh]);

  const micGranted = permissions?.microphone === true;
  const micDenied = permissions != null && permissions.microphone === false;

  return (
    <div className="flex flex-col items-center text-center">
      <div className="mb-5 flex h-16 w-16 items-center justify-center rounded-2xl bg-[#0058bc]/[0.10] shadow-[0_0_24px_rgba(0,88,188,0.10)]">
        <Icon name="mic" size={32} className="text-[#0058bc]" />
      </div>

      <h2 className="mb-2 text-[22px] font-semibold text-black/85">
        Allow microphone access
      </h2>
      <p className="mb-6 max-w-[360px] text-[13px] leading-relaxed text-black/50">
        LocalYapper needs your microphone to hear you when you hold the
        dictation hotkey. Audio stays on your device the entire time.
      </p>

      <div
        className={`mb-6 w-full rounded-[10px] border px-4 py-3 text-left transition-colors ${
          micGranted
            ? "border-[#006b19]/15 bg-[#006b19]/[0.05]"
            : micDenied
              ? "border-[#ba1a1a]/20 bg-[#ba1a1a]/[0.05]"
              : "border-black/[0.08] bg-black/[0.02]"
        }`}
      >
        <div className="flex items-center gap-3">
          <Icon
            name={
              (loading
                ? "sync"
                : micGranted
                  ? "check_circle"
                  : micDenied
                    ? "error"
                    : "help") as IconName
            }
            size={20}
            className={
              micGranted
                ? "text-[#006b19]"
                : micDenied
                  ? "text-[#ba1a1a]"
                  : "text-black/40"
            }
          />
          <div className="flex-1">
            <p
              className={`text-[13px] font-semibold ${
                micGranted
                  ? "text-[#006b19]"
                  : micDenied
                    ? "text-[#ba1a1a]"
                    : "text-black/85"
              }`}
            >
              {loading
                ? "Checking microphone..."
                : micGranted
                  ? "Microphone detected"
                  : micDenied
                    ? "No microphone available"
                    : "Microphone status unknown"}
            </p>
            <p className="mt-0.5 text-[12px] text-black/50">
              {micGranted
                ? "Your default input device is ready for dictation."
                : "Open system settings to allow LocalYapper to use the microphone, or plug in a recording device."}
            </p>
          </div>
        </div>
      </div>

      {micDenied && (
        <button
          type="button"
          onClick={openSettings}
          className="mb-3 inline-flex h-9 items-center justify-center gap-1.5 rounded-[8px] border border-black/[0.10] bg-white px-4 text-[13px] font-medium text-black/85 shadow-sm transition-colors hover:bg-black/[0.02]"
        >
          <Icon name="open_in_new" size={16} />
          Open microphone settings
        </button>
      )}

      <button
        type="button"
        onClick={onContinue}
        className="h-9 w-full rounded-[8px] bg-gradient-to-b from-[#0062d0] to-[#0058bc] text-[13px] font-medium text-white transition-all hover:brightness-110 active:brightness-95 disabled:cursor-not-allowed disabled:opacity-60"
        disabled={loading}
      >
        {micGranted ? "Continue" : micDenied ? "Continue anyway" : "Continue"}
      </button>

      <button
        type="button"
        onClick={refresh}
        className="mt-3 text-[12px] text-black/[0.45] hover:text-black/65"
      >
        Re-check microphone
      </button>
    </div>
  );
}
