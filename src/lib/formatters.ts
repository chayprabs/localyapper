const numberFormatter = new Intl.NumberFormat("en-US");

export function formatNumber(n: number): string {
  return numberFormatter.format(n);
}

const MINUTE = 60_000;
const HOUR = 3_600_000;
const DAY = 86_400_000;

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
  const days = Math.floor(diff / DAY);
  return `${days} days ago`;
}
