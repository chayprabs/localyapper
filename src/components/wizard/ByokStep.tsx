import { useState, useRef, useEffect } from "react";
import type { ConnectionResult } from "@/types/commands";

const PROVIDER_OPTIONS: { value: string; label: string }[] = [
  { value: "openai", label: "OpenAI" },
  { value: "anthropic", label: "Anthropic" },
  { value: "groq", label: "Groq" },
];

export function ByokStep({
  provider,
  apiKey,
  connectionResult,
  isTesting,
  onProviderChange,
  onApiKeyChange,
  onTestConnection,
  onContinue,
  onBack,
}: {
  provider: string;
  apiKey: string;
  connectionResult: ConnectionResult | null;
  isTesting: boolean;
  onProviderChange: (provider: "openai" | "anthropic" | "groq") => void;
  onApiKeyChange: (key: string) => void;
  onTestConnection: () => void;
  onContinue: () => void;
  onBack: () => void;
}) {
  const [showKey, setShowKey] = useState(false);
  const [dropdownOpen, setDropdownOpen] = useState(false);
  const dropdownRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!dropdownOpen) return;
    function handleClick(e: MouseEvent) {
      if (dropdownRef.current && !dropdownRef.current.contains(e.target as Node)) {
        setDropdownOpen(false);
      }
    }
    document.addEventListener("mousedown", handleClick);
    return () => document.removeEventListener("mousedown", handleClick);
  }, [dropdownOpen]);

  const selectedProvider = PROVIDER_OPTIONS.find((p) => p.value === provider);
  const canContinue = connectionResult?.success === true;

  return (
    <div>
      <button
        onClick={onBack}
        className="flex items-center gap-1 text-[13px] text-black/[0.40] hover:text-black/60 transition-colors mb-4"
      >
        <span className="material-symbols-outlined text-[16px]">arrow_back</span>
        Back
      </button>

      <h2 className="text-[20px] font-semibold text-black/85 mb-1">
        API Key Setup
      </h2>
      <p className="text-[13px] text-black/50 mb-5">
        Your key is stored locally and never leaves your device.
      </p>

      {/* Provider */}
      <div className="mb-4" ref={dropdownRef}>
        <label className="block text-[12px] text-black/[0.40] mb-1.5">
          Provider
        </label>
        <div className="relative">
          <div
            onClick={() => setDropdownOpen(!dropdownOpen)}
            className="w-full h-9 bg-white border border-black/[0.12] rounded-[8px] px-3 flex items-center justify-between cursor-pointer hover:border-black/20"
          >
            <span className="text-[13px] text-black/85">
              {selectedProvider?.label ?? provider}
            </span>
            <span className="material-symbols-outlined text-[16px] text-black/[0.40]">
              expand_more
            </span>
          </div>
          {dropdownOpen && (
            <div className="absolute left-0 right-0 top-full mt-1 bg-white rounded-lg border border-black/[0.07] shadow-lg z-10 py-1">
              {PROVIDER_OPTIONS.map((opt) => (
                <div
                  key={opt.value}
                  onClick={() => {
                    onProviderChange(opt.value as "openai" | "anthropic" | "groq");
                    setDropdownOpen(false);
                  }}
                  className={`px-3 py-1.5 text-[13px] cursor-pointer hover:bg-black/[0.04] ${
                    opt.value === provider
                      ? "text-[#0058bc] font-medium"
                      : "text-black/85"
                  }`}
                >
                  {opt.label}
                </div>
              ))}
            </div>
          )}
        </div>
      </div>

      {/* API Key */}
      <div className="mb-5">
        <label className="block text-[12px] text-black/[0.40] mb-1.5">
          API Key
        </label>
        <div className="relative">
          <input
            type={showKey ? "text" : "password"}
            value={apiKey}
            onChange={(e) => onApiKeyChange(e.target.value)}
            placeholder="sk-..."
            className="w-full h-9 bg-white border border-black/[0.12] rounded-[8px] text-[13px] px-3 pr-9 focus:outline-none focus:ring-1 focus:ring-[#0058bc] placeholder:text-black/[0.25]"
          />
          <button
            type="button"
            onClick={() => setShowKey(!showKey)}
            className="absolute right-2.5 top-1/2 -translate-y-1/2"
          >
            <span className="material-symbols-outlined text-[16px] text-black/[0.35]">
              {showKey ? "visibility_off" : "visibility"}
            </span>
          </button>
        </div>
      </div>

      {/* Test Connection */}
      <button
        onClick={onTestConnection}
        disabled={isTesting || !apiKey}
        className="w-full h-9 bg-white border border-black/[0.15] text-[13px] font-medium rounded-[8px] hover:bg-black/[0.02] transition-colors disabled:opacity-50 disabled:cursor-not-allowed mb-3"
      >
        {isTesting ? "Testing..." : "Test Connection"}
      </button>

      {/* Result */}
      {connectionResult && (
        <p
          className={`text-[12px] font-medium text-center mb-3 ${
            connectionResult.success ? "text-[#28CD41]" : "text-[#ba1a1a]"
          }`}
        >
          {connectionResult.success
            ? `Connected \u00B7 ${connectionResult.latency_ms}ms`
            : connectionResult.error ?? "Connection failed"}
        </p>
      )}

      {/* Continue */}
      <button
        onClick={onContinue}
        disabled={!canContinue}
        className="w-full h-9 bg-gradient-to-b from-[#0062d0] to-[#0058bc] text-white text-[13px] font-medium rounded-[8px] hover:brightness-110 transition-all disabled:opacity-40 disabled:cursor-not-allowed mt-1"
      >
        Continue
      </button>
    </div>
  );
}
