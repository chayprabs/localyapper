// Number and date formatting utilities for display
const numberFormatter = new Intl.NumberFormat("en-US");

export function formatNumber(n: number): string {
  return numberFormatter.format(n);
}

/** Time unit constants in milliseconds for relative time formatting. */
const MINUTE = 60_000;
const HOUR = 3_600_000;
const DAY = 86_400_000;
const WEEK = 604_800_000;

export function formatRelativeTime(isoDate: string): string {
  const timestamp = new Date(isoDate.endsWith("Z") ? isoDate : isoDate + "Z");
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

const timeFormatter = new Intl.DateTimeFormat("en-US", {
  hour: "numeric",
  minute: "2-digit",
  hour12: true,
});

const dateTimeFormatter = new Intl.DateTimeFormat("en-US", {
  month: "short",
  day: "numeric",
  hour: "numeric",
  minute: "2-digit",
  hour12: true,
});

export function formatHistoryTimestamp(isoDate: string): string {
  const timestamp = new Date(isoDate.endsWith("Z") ? isoDate : isoDate + "Z");
  const now = new Date();

  const isToday =
    timestamp.getFullYear() === now.getFullYear() &&
    timestamp.getMonth() === now.getMonth() &&
    timestamp.getDate() === now.getDate();

  if (isToday) return `Today, ${timeFormatter.format(timestamp)}`;

  const yesterday = new Date(now);
  yesterday.setDate(yesterday.getDate() - 1);
  const isYesterday =
    timestamp.getFullYear() === yesterday.getFullYear() &&
    timestamp.getMonth() === yesterday.getMonth() &&
    timestamp.getDate() === yesterday.getDate();

  if (isYesterday) return `Yesterday, ${timeFormatter.format(timestamp)}`;

  return dateTimeFormatter.format(timestamp);
}
