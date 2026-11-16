// Number and date formatting utilities for display
const numberFormatter = new Intl.NumberFormat("en-US");

export function formatNumber(n: number): string {
  return numberFormatter.format(n);
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

const dayWithYearFormatter = new Intl.DateTimeFormat(undefined, {
  weekday: "long",
  month: "long",
  day: "numeric",
  year: "numeric",
});

/** Local calendar day of a stored timestamp: "Today", "Yesterday" or a date. */
export function formatLocalDay(stored: string, now: Date = new Date()): string {
  const d = parseStoredDate(stored);
  const startOf = (x: Date) => new Date(x.getFullYear(), x.getMonth(), x.getDate()).getTime();
  const days = Math.round((startOf(now) - startOf(d)) / 86_400_000);
  if (days === 0) return "Today";
  if (days === 1) return "Yesterday";
  // An entry from another year says which.
  return (d.getFullYear() === now.getFullYear() ? dayFormatter : dayWithYearFormatter).format(d);
}

/** Local time of day of a stored timestamp. */
export function formatLocalTime(stored: string): string {
  return timeOfDayFormatter.format(parseStoredDate(stored));
}

const timeOfDayFormatter = new Intl.DateTimeFormat(undefined, {
  hour: "numeric",
  minute: "2-digit",
});
