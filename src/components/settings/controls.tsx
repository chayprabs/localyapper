// Setting controls -- each one loads its key, saves on change and shows the
// backend's error inline (plan P5.4). The backend validates every value.
import { useCallback, useEffect, useState, type ReactNode } from "react";
import { listen } from "@tauri-apps/api/event";
import { getSetting, setSetting } from "@/lib/commands/settings";
import { Switch } from "@/components/ui/Switch";

export function errorText(err: unknown): string {
  return err instanceof Error ? err.message : typeof err === "string" ? err : "Something went wrong";
}

interface SettingState {
  value: string | null;
  error: string | null;
  save: (next: string) => void;
}

/** One setting's value, kept in step with `settings-changed` events. */
export function useSetting(key: string): SettingState {
  const [value, setValue] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let live = true;
    getSetting(key)
      .then((v) => live && setValue(v))
      .catch((err: unknown) => live && setError(errorText(err)));
    const unlisten = listen<{ key: string; value: string }>("settings-changed", (e) => {
      if (e.payload.key === key) setValue(e.payload.value);
    });
    return () => {
      live = false;
      void unlisten.then((fn) => fn());
    };
  }, [key]);

  const save = useCallback(
    (next: string) => {
      const previous = value;
      setValue(next);
      setError(null);
      setSetting(key, next).catch((err: unknown) => {
        setValue(previous);
        setError(errorText(err));
      });
    },
    [key, value],
  );

  return { value, error, save };
}

export function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="mb-8">
      <h2 className="mb-2 text-[12px] font-semibold uppercase tracking-wide text-black/40">{title}</h2>
      <div className="divide-y divide-black/[0.06] rounded-xl border border-black/[0.08] bg-white">
        {children}
      </div>
    </section>
  );
}

export function Row({
  label,
  hint,
  error,
  children,
}: {
  label: string;
  hint?: string;
  error?: string | null;
  children: ReactNode;
}) {
  return (
    <div className="px-5 py-3">
      <div className="flex items-center justify-between gap-4">
        <div className="min-w-0">
          <div className="text-[13px] font-medium text-black/85">{label}</div>
          {hint && <div className="text-[12px] text-black/50">{hint}</div>}
        </div>
        <div className="shrink-0">{children}</div>
      </div>
      {error && (
        <p role="alert" className="mt-2 text-[12px] font-medium text-[#ba1a1a]">
          {error}
        </p>
      )}
    </div>
  );
}

export function ToggleSetting({ k, label, hint }: { k: string; label: string; hint?: string }) {
  const { value, error, save } = useSetting(k);
  return (
    <Row label={label} hint={hint} error={error}>
      <Switch
        ariaLabel={label}
        checked={value === "true"}
        disabled={value === null}
        onChange={(next) => save(next ? "true" : "false")}
      />
    </Row>
  );
}

export function SelectSetting({
  k,
  label,
  hint,
  options,
}: {
  k: string;
  label: string;
  hint?: string;
  options: { value: string; label: string }[];
}) {
  const { value, error, save } = useSetting(k);
  const known = value === null || options.some((o) => o.value === value);
  return (
    <Row label={label} hint={hint} error={error}>
      <select
        aria-label={label}
        className="h-8 rounded-md border border-black/[0.12] bg-white px-2 text-[13px] text-black/85"
        value={value ?? ""}
        disabled={value === null}
        onChange={(e) => save(e.target.value)}
      >
        {!known && value !== null && <option value={value}>{value}</option>}
        {options.map((o) => (
          <option key={o.value} value={o.value}>
            {o.label}
          </option>
        ))}
      </select>
    </Row>
  );
}

/** A whole number saved when the field loses focus or Enter is pressed. */
export function NumberSetting({
  k,
  label,
  hint,
  min,
  max,
  unit,
}: {
  k: string;
  label: string;
  hint?: string;
  min: number;
  max: number;
  unit: string;
}) {
  const { value, error, save } = useSetting(k);
  const [draft, setDraft] = useState("");
  useEffect(() => {
    if (value !== null) setDraft(value);
  }, [value]);
  const commit = () => {
    if (draft !== value) save(draft);
  };
  return (
    <Row label={label} hint={hint} error={error}>
      <span className="flex items-center gap-2 text-[13px] text-black/50">
        <input
          aria-label={label}
          type="number"
          min={min}
          max={max}
          className="h-8 w-24 rounded-md border border-black/[0.12] bg-white px-2 text-right text-[13px] text-black/85"
          value={draft}
          disabled={value === null}
          onChange={(e) => setDraft(e.target.value)}
          onBlur={commit}
          onKeyDown={(e) => {
            if (e.key === "Enter") commit();
          }}
        />
        {unit}
      </span>
    </Row>
  );
}

export function PageHeader({ title, subtitle }: { title: string; subtitle?: string }) {
  return (
    <header className="mb-8">
      <h1 className="text-[26px] font-semibold text-black/90">{title}</h1>
      {subtitle && <p className="mt-1 text-[13px] text-black/50">{subtitle}</p>}
    </header>
  );
}
