import { describe, expect, it } from "vitest";
import { formatBytes, formatDuration, clockTime, dayLabel, sameGroup } from "./format";
import { hexToRgb, mix, contrastRatio, resolveTheme, BASE_TOKENS } from "./theme";
import { parseSlash, matchCommands } from "./slash";
import { mergeWindow } from "./messageWindow";

describe("format", () => {
  it("formats bytes with binary units", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(1536)).toBe("1.5 KiB");
    expect(formatBytes(73 * 1024 * 1024)).toBe("73.0 MiB");
  });
  it("formats durations", () => {
    expect(formatDuration(850)).toBe("850 ms");
    expect(formatDuration(65_000)).toBe("1 min 5 s");
    expect(formatDuration(3_720_000)).toBe("1 h 2 min");
  });
  it("formats clock time with two digits", () => {
    expect(clockTime(new Date(2026, 0, 1, 9, 5).getTime())).toBe("09:05");
  });
  it("labels days relative to now", () => {
    const now = new Date(2026, 10, 7, 12).getTime();
    expect(dayLabel(new Date(2026, 10, 7, 8).getTime(), now)).toBe("Today");
    expect(dayLabel(new Date(2026, 10, 6, 23).getTime(), now)).toBe("Yesterday");
  });
  it("groups consecutive messages by author within five minutes", () => {
    expect(sameGroup({ authorId: "a", createdAt: 0 }, { authorId: "a", createdAt: 299_000 })).toBe(true);
    expect(sameGroup({ authorId: "a", createdAt: 0 }, { authorId: "a", createdAt: 301_000 })).toBe(false);
    expect(sameGroup({ authorId: "a", createdAt: 0 }, { authorId: "b", createdAt: 1 })).toBe(false);
  });
});

describe("theme", () => {
  it("parses hex", () => {
    expect(hexToRgb("#9278f5")).toEqual([146, 120, 245]);
    expect(hexToRgb("nope")).toBeNull();
  });
  it("mixes colours", () => {
    expect(mix("#000000", "#ffffff", 0.5)).toBe("#808080");
  });
  it("computes WCAG contrast", () => {
    expect(contrastRatio("#000000", "#ffffff")).toBeCloseTo(21, 0);
  });
  it("applies theme tokens, then accent override, then high contrast", () => {
    const t = resolveTheme({ tokens: { accent: "#f08a5d" }, accentOverride: "#22aa88", highContrast: false });
    expect(t["--dz-accent"]).toBe("#22aa88");
    expect(t["--dz-background"]).toBe(BASE_TOKENS.background);
    const hc = resolveTheme({ tokens: {}, accentOverride: null, highContrast: true });
    expect(contrastRatio(hc["--dz-text-secondary"]!, hc["--dz-surface"]!)).toBeGreaterThanOrEqual(7);
  });
  it("ignores invalid token values", () => {
    const t = resolveTheme({ tokens: { accent: "url(evil)" }, accentOverride: null, highContrast: false });
    expect(t["--dz-accent"]).toBe(BASE_TOKENS.accent);
  });
});

describe("slash commands", () => {
  const cmds = [
    { pluginId: "p", pluginName: "P", name: "shrug", description: "", usage: null },
    { pluginId: "p", pluginName: "P", name: "roll", description: "", usage: null },
  ];
  it("parses a command and its arguments", () => {
    expect(parseSlash("/roll 2d6")).toEqual({ name: "roll", args: "2d6" });
    expect(parseSlash("/shrug")).toEqual({ name: "shrug", args: "" });
    expect(parseSlash("hello")).toBeNull();
    expect(parseSlash("/ spaced")).toBeNull();
  });
  it("matches by prefix while typing the name only", () => {
    expect(matchCommands("/r", cmds).map((c) => c.name)).toEqual(["roll"]);
    expect(matchCommands("/", cmds)).toHaveLength(2);
    expect(matchCommands("/roll 2d6", cmds)).toHaveLength(0);
  });
});

describe("message window", () => {
  const m = (id: string, t: number) => ({ id, createdAt: t });
  it("appends, dedupes and keeps order", () => {
    const out = mergeWindow([m("a", 1), m("b", 2)], [m("b", 2), m("c", 3)], 10);
    expect(out.messages.map((x) => x.id)).toEqual(["a", "b", "c"]);
    expect(out.trimmed).toBe(false);
  });
  it("caps to the window size by dropping the oldest", () => {
    const out = mergeWindow([m("a", 1), m("b", 2)], [m("c", 3)], 2);
    expect(out.messages.map((x) => x.id)).toEqual(["b", "c"]);
    expect(out.trimmed).toBe(true);
  });
  it("prepends older pages without exceeding the cap from the old end", () => {
    const out = mergeWindow([m("c", 3), m("d", 4)], [m("a", 1), m("b", 2)], 3, "older");
    expect(out.messages.map((x) => x.id)).toEqual(["a", "b", "c"]);
  });
});
