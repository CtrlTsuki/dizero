// The in-memory message window for one conversation is bounded by the
// performance profile. Loading newer messages trims the oldest; paging back
// through history trims the newest (the UI then offers "Jump to present").

export function mergeWindow<T extends { id: string; createdAt: number }>(
  current: T[],
  incoming: T[],
  cap: number,
  direction: "newer" | "older" = "newer",
): { messages: T[]; trimmed: boolean } {
  const byId = new Map<string, T>();
  for (const m of current) byId.set(m.id, m);
  for (const m of incoming) byId.set(m.id, m);
  const all = [...byId.values()].sort((a, b) => a.createdAt - b.createdAt || (a.id < b.id ? -1 : a.id > b.id ? 1 : 0));
  if (all.length <= cap) return { messages: all, trimmed: false };
  return {
    messages: direction === "newer" ? all.slice(all.length - cap) : all.slice(0, cap),
    trimmed: true,
  };
}
