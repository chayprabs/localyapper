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

/** A page: one centred column. */
export function Page({ children, className }: { children: ReactNode; className?: string }) {
  return (
    <div className={cn("mx-auto flex w-full max-w-[620px] animate-fade-in flex-col gap-9 px-6 pb-12 pt-7", className)}>
      {children}
    </div>
  );
}

/** A titled list of rows, separated by hairlines. */
export function Section({ title, action, children }: { title: string; action?: ReactNode; children: ReactNode }) {
  return (
    <section>
      <div className="flex h-7 items-center justify-between gap-3 border-b border-ink/15">
        <h2 className="text-[12px] text-ink/45">{title}</h2>
        {action}
      </div>
      <div className="divide-y divide-ink/[0.08]">{children}</div>
    </section>
  );
}

export function Row({
  label,
  hint,
  detail,
  error,
  children,
  below,
}: {
  label: ReactNode;
  /** Shown as a tooltip only. */
  hint?: string;
  /** Secondary text beside the label. */
  detail?: ReactNode;
  error?: string | null;
  children?: ReactNode;
  /** Full-width content under the label line. */
  below?: ReactNode;
}) {
  return (
    <div className="py-2" title={hint}>
      <div className="flex min-h-7 items-center justify-between gap-4">
        <div className="flex min-w-0 items-baseline gap-2.5">
          <span className="shrink-0 text-[13px] text-ink/90">{label}</span>
          {detail && <span className="min-w-0 truncate text-[12px] text-ink/40">{detail}</span>}
        </div>
        {children && <div className="shrink-0">{children}</div>}
      </div>
      {below}
      {error && <Notice>{error}</Notice>}
    </div>
  );
}

/** An inline message. Errors are in the signal colour; notes are plain. */
export function Notice({ tone = "error", children }: { tone?: "error" | "note"; children: ReactNode }) {
  return (
    <p
      role={tone === "error" ? "alert" : "status"}
      className={cn(
        "selectable animate-fade-in break-words text-[12px] leading-snug",
        tone === "error" ? "text-signal" : "text-ink/55",
      )}
    >
      {children}
    </p>
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

/** A native select with a plain outline. */
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
          "h-7 max-w-[240px] cursor-pointer appearance-none truncate rounded-md border border-ink/20 bg-canvas pl-2.5 pr-7 " +
          "text-[12.5px] text-ink/90 transition-colors hover:border-ink/50 disabled:opacity-40"
        }
        value={value}
        disabled={disabled}
        onChange={(e) => onChange(e.target.value)}
      >
        {children}
      </select>
      <Icon
        name="expand_more"
        size={13}
        className="pointer-events-none absolute right-2 top-1/2 -translate-y-1/2 text-ink/45"
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

/** A few options side by side; the chosen one is filled. */
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
    <div role="radiogroup" aria-label={label} className="inline-flex divide-x divide-ink/20 overflow-hidden rounded-md border border-ink/20">
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
              "h-[26px] px-2.5 text-[12.5px] transition-colors duration-100 focus-visible:-outline-offset-2",
              on ? "bg-ink text-canvas" : "text-ink/55 hover:text-ink",
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
      <label className="flex h-7 cursor-text items-center gap-1.5 rounded-md border border-ink/20 px-2.5 font-mono text-[12px] text-ink/45 transition-colors focus-within:border-ink/60 hover:border-ink/50">
        <input
          aria-label={label}
          type="number"
          min={min}
          max={max}
          className="w-12 appearance-none bg-transparent text-right text-ink/90 outline-none [-moz-appearance:textfield] [&::-webkit-inner-spin-button]:appearance-none"
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

export function Spinner({ size = 16 }: { size?: number }) {
  return <Icon name="progress_activity" size={size} className="animate-spin text-ink/35" />;
}
