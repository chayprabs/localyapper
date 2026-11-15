// Number and date formatting utilities for display
const numberFormatter = new Intl.NumberFormat("en-US");

export function formatNumber(n: number): string {
  return numberFormatter.format(n);
}

/** "1 word", "12 words". */
export function formatWordCount(count: number | null | undefined): string {
  const n = count ?? 0;
  return `${formatNumber(n)} word${n === 1 ? "" : "s"}`;
}

/**
 * History rows store UTC: ISO-8601 with `Z` now, "YYYY-MM-DD HH:MM:SS"
 * in databases written before version 2.
 */
export function parseStoredDate(stored: string): Date {
  const iso = stored.includes("T") ? stored : stored.replace(" ", "T");
  return new Date(/[zZ]|[+-]\d\d:\d\d$/.test(iso) ? iso : iso + "Z");
}

const dayFormatter = new Intl.DateTimeFormat(undefined, {
  weekday: "long",
  month: "long",
  day: "numeric",
});

/** Local calendar day of a stored timestamp: "Today", "Yesterday" or a date. */
export function formatLocalDay(stored: string, now: Date = new Date()): string {
  const d = parseStoredDate(stored);
  const startOf = (x: Date) => new Date(x.getFullYear(), x.getMonth(), x.getDate()).getTime();
  const days = Math.round((startOf(now) - startOf(d)) / 86_400_000);
  if (days === 0) return "Today";
  if (days === 1) return "Yesterday";
  return dayFormatter.format(d);
}

/** Local time of day of a stored timestamp. */
export function formatLocalTime(stored: string): string {
  return timeOfDayFormatter.format(parseStoredDate(stored));
}

const timeOfDayFormatter = new Intl.DateTimeFormat(undefined, {
  hour: "numeric",
  minute: "2-digit",
});

/** Time unit constants in milliseconds for relative time formatting. */
const MINUTE = 60_000;
const HOUR = 3_600_000;
const DAY = 86_400_000;
const WEEK = 604_800_000;

export function formatRelativeTime(isoDate: string): string {
  const timestamp = parseStoredDate(isoDate);
  const now = Date.now();
  const diff = now - timestamp.getTime();

  if (diff < MINUTE) return "just now";
  if (diff < HOUR) {
    const mins = Math.floor(diff / MINUTE);
    return `${mins} minute${mins === 1 ? "" : "s"} ago`;
  }
  if (diff < DAY) {
    const hours = Math.floor(diff / HOUR);
    return `${hours} hour${hours === 1 ? "" : "s"} ago`;
  }
  if (diff < DAY * 2) return "yesterday";
  if (diff < WEEK) {
    const days = Math.floor(diff / DAY);
    return `${days} days ago`;
  }
  const weeks = Math.floor(diff / WEEK);
  return `${weeks} week${weeks === 1 ? "" : "s"} ago`;
}

