// Setting controls -- each one loads its key, saves on change and shows the
// backend's error inline. The backend validates every value.
import { useCallback, useEffect, useState, type ReactNode } from "react";
import { listen } from "@tauri-apps/api/event";
import { getSetting, setSetting } from "@/lib/commands/settings";
import { Switch } from "@/components/ui/Switch";
import { Icon } from "@/components/ui/Icon";
import { cn } from "@/lib/utils";

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

/** A page: a centred column whose blocks rise in one after another. */
export function Page({ children, className }: { children: ReactNode; className?: string }) {
  return (
    <div className={cn("stagger mx-auto flex w-full max-w-[680px] flex-col gap-6 px-6 pb-10 pt-5", className)}>
      {children}
    </div>
  );
}

export function PageHeader({ title, subtitle, children }: { title: string; subtitle?: string; children?: ReactNode }) {
  return (
    <header className="flex flex-wrap items-start justify-between gap-x-4 gap-y-2">
      <div className="min-w-0">
        <h1 className="text-[22px] font-semibold leading-9 tracking-[-0.02em] text-ink">{title}</h1>
        {subtitle && <p className="text-[13px] text-ink/50">{subtitle}</p>}
      </div>
      {children}
    </header>
  );
}

/** A glass panel. */
export function Panel({ children, className }: { children: ReactNode; className?: string }) {
  return <div className={cn("glass rounded-[20px]", className)}>{children}</div>;
}

export function Section({ title, action, children }: { title: string; action?: ReactNode; children: ReactNode }) {
  return (
    <section>
      <div className="mb-2 flex min-h-7 items-center justify-between gap-3 px-4">
        <h2 className="text-[12px] font-medium text-ink/45">{title}</h2>
        {action}
      </div>
      <Panel className="divide-y divide-ink/[0.06]">{children}</Panel>
    </section>
  );
}

export function Row({
  label,
  hint,
  error,
  children,
  below,
}: {
  label: ReactNode;
  hint?: ReactNode;
  error?: string | null;
  children?: ReactNode;
  /** Full-width content under the label line. */
  below?: ReactNode;
}) {
  return (
    <div className="px-4 py-3">
      <div className="flex min-h-8 items-center justify-between gap-4">
        <div className="min-w-0">
          <div className="text-[13px] font-medium text-ink/90">{label}</div>
          {hint && <div className="mt-0.5 break-words text-[12px] leading-snug text-ink/45">{hint}</div>}
        </div>
        {children && <div className="shrink-0">{children}</div>}
      </div>
      {below}
      {error && (
        <p role="alert" className="mt-1.5 text-[12px] font-medium text-danger">
          {error}
        </p>
      )}
    </div>
  );
}

/** An inline message: an error, a warning or a neutral note. */
export function Notice({
  tone = "danger",
  children,
  action,
}: {
  tone?: "danger" | "warn" | "info";
  children: ReactNode;
  action?: ReactNode;
}) {
  return (
    <div
      role={tone === "danger" ? "alert" : "status"}
      className={cn(
        "flex animate-rise-in items-center gap-2.5 rounded-2xl px-3.5 py-2.5 text-[12px] font-medium",
        tone === "danger" && "bg-danger/[0.08] text-danger",
        tone === "warn" && "bg-warn/10 text-warn",
        tone === "info" && "bg-ink/[0.05] text-ink/60",
      )}
    >
      {tone !== "info" && <Icon name={tone === "danger" ? "error" : "warning"} size={15} className="shrink-0" />}
      <span className="selectable min-w-0 flex-1 break-words text-left">{children}</span>
      {action}
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

/** A native select drawn as a pill. */
export function Select({
  label,
  value,
  disabled,
  onChange,
  children,
}: {
  label: string;
  value: string;
  disabled?: boolean;
  onChange: (value: string) => void;
  children: ReactNode;
}) {
  return (
    <span className="relative inline-flex">
      <select
        aria-label={label}
        className={
          "h-8 max-w-[250px] cursor-pointer appearance-none truncate rounded-full bg-ink/[0.06] pl-3.5 pr-8 " +
          "text-[13px] text-ink/85 transition-colors hover:bg-ink/[0.1] disabled:opacity-50"
        }
        value={value}
        disabled={disabled}
        onChange={(e) => onChange(e.target.value)}
      >
        {children}
      </select>
      <Icon
        name="expand_more"
        size={14}
        className="pointer-events-none absolute right-3 top-1/2 -translate-y-1/2 text-ink/45"
      />
    </span>
  );
}

interface Option {
  value: string;
  label: string;
}

export function SelectSetting({ k, label, hint, options }: { k: string; label: string; hint?: string; options: Option[] }) {
  const { value, error, save } = useSetting(k);
  const known = value === null || options.some((o) => o.value === value);
  return (
    <Row label={label} hint={hint} error={error}>
      <Select label={label} value={value ?? ""} disabled={value === null} onChange={save}>
        {!known && value !== null && <option value={value}>{value}</option>}
        {options.map((o) => (
          <option key={o.value} value={o.value}>
            {o.label}
          </option>
        ))}
      </Select>
    </Row>
  );
}

/** A few options side by side; the chosen one is lifted. */
export function Segmented({
  label,
  value,
  options,
  onChange,
}: {
  label: string;
  value: string | null;
  options: Option[];
  onChange: (value: string) => void;
}) {
  return (
    <div role="radiogroup" aria-label={label} className="inline-flex rounded-full bg-ink/[0.06] p-[3px]">
      {options.map((o) => {
        const on = o.value === value;
        return (
          <button
            key={o.value}
            type="button"
            role="radio"
            aria-checked={on}
            disabled={value === null}
            onClick={() => onChange(o.value)}
            className={cn(
              "h-[26px] rounded-full px-3 text-[12px] font-medium transition-[color,background-color,box-shadow] duration-200",
              on ? "raised text-ink" : "text-ink/50 hover:text-ink/80",
            )}
          >
            {o.label}
          </button>
        );
      })}
    </div>
  );
}

export function SegmentedSetting({ k, label, hint, options }: { k: string; label: string; hint?: string; options: Option[] }) {
  const { value, error, save } = useSetting(k);
  return (
    <Row label={label} hint={hint} error={error}>
      <Segmented label={label} value={value} options={options} onChange={save} />
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
      <label className="flex h-8 cursor-text items-center gap-1.5 rounded-full bg-ink/[0.06] pl-3 pr-3.5 text-[13px] text-ink/45 transition-colors focus-within:bg-ink/[0.1] hover:bg-ink/[0.1]">
        <input
          aria-label={label}
          type="number"
          min={min}
          max={max}
          className="w-14 appearance-none bg-transparent text-right tabular-nums text-ink/90 outline-none [-moz-appearance:textfield] [&::-webkit-inner-spin-button]:appearance-none"
          value={draft}
          disabled={value === null}
          onChange={(e) => setDraft(e.target.value)}
          onBlur={commit}
          onKeyDown={(e) => {
            if (e.key === "Enter") commit();
          }}
        />
        {unit}
      </label>
    </Row>
  );
}

/** A small label with a status dot or plain text. */
export function Chip({ tone = "neutral", children }: { tone?: "neutral" | "accent" | "ok" | "warn"; children: ReactNode }) {
  return (
    <span
      className={cn(
        "inline-flex h-5 max-w-full items-center truncate rounded-full px-2 text-[11px] font-medium",
        tone === "neutral" && "bg-ink/[0.06] text-ink/55",
        tone === "accent" && "bg-accent/[0.12] text-accent",
        tone === "ok" && "bg-ok/[0.12] text-ok",
        tone === "warn" && "bg-warn/[0.12] text-warn",
      )}
    >
      {children}
    </span>
  );
}

export function Spinner({ size = 20 }: { size?: number }) {
  return <Icon name="progress_activity" size={size} className="animate-spin text-ink/30" />;
}
