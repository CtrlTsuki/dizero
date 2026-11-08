const UNITS = ["KiB", "MiB", "GiB", "TiB"];

export function formatBytes(n: number): string {
  if (!Number.isFinite(n) || n < 1024) return `${Math.max(0, Math.round(n || 0))} B`;
  let v = n / 1024;
  let u = 0;
  while (v >= 1024 && u < UNITS.length - 1) {
    v /= 1024;
    u++;
  }
  return `${v.toFixed(1)} ${UNITS[u]}`;
}

export function formatDuration(ms: number): string {
  if (ms < 1000) return `${Math.round(ms)} ms`;
  if (ms < 60_000) return `${(ms / 1000).toFixed(1)} s`;
  const totalSec = Math.floor(ms / 1000);
  const h = Math.floor(totalSec / 3600);
  const min = Math.floor((totalSec % 3600) / 60);
  const s = totalSec % 60;
  return h > 0 ? `${h} h ${min} min` : `${min} min ${s} s`;
}

const pad = (n: number) => String(n).padStart(2, "0");

export function clockTime(ts: number): string {
  const d = new Date(ts);
  return `${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

function startOfDay(ts: number): number {
  const d = new Date(ts);
  d.setHours(0, 0, 0, 0);
  return d.getTime();
}

export function dayLabel(ts: number, now: number = Date.now()): string {
  const diffDays = Math.round((startOfDay(now) - startOfDay(ts)) / 86_400_000);
  if (diffDays === 0) return "Today";
  if (diffDays === 1) return "Yesterday";
  return new Date(ts).toLocaleDateString(undefined, { weekday: "long", month: "short", day: "numeric" });
}

export function fullTimestamp(ts: number): string {
  return new Date(ts).toLocaleString(undefined, {
    weekday: "long",
    year: "numeric",
    month: "long",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

export function relativeTime(ts: number, now: number = Date.now()): string {
  const s = Math.max(0, Math.round((now - ts) / 1000));
  if (s < 60) return "just now";
  if (s < 3600) return `${Math.floor(s / 60)} min ago`;
  if (s < 86_400) return `${Math.floor(s / 3600)} h ago`;
  return `${Math.floor(s / 86_400)} d ago`;
}

const GROUP_WINDOW_MS = 300_000;

/** Whether `b` continues `a`'s visual group (same author, within 5 minutes). */
export function sameGroup(
  a: { authorId: string; createdAt: number },
  b: { authorId: string; createdAt: number },
): boolean {
  const gap = b.createdAt - a.createdAt;
  return a.authorId === b.authorId && gap >= 0 && gap < GROUP_WINDOW_MS;
}
