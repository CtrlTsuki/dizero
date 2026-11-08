// A deliberately small, safe message formatter. It produces a token tree
// that components render as text nodes. Nothing is ever parsed as HTML, so
// message content cannot inject markup or scripts.

export type Inline =
  | { t: "text" | "bold" | "italic" | "code" | "link"; v: string }
  | { t: "mention"; v: string; self: boolean };

export type Block =
  | { type: "para"; inlines: Inline[] }
  | { type: "quote"; inlines: Inline[] }
  | { type: "code"; lang: string; text: string };

const escapeRegExp = (s: string) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
const TRAILING_PUNCTUATION = /[.,;:!?)\]'"]+$/;

function inlineRegex(selfName: string): RegExp {
  const self = selfName.trim() ? `${escapeRegExp(selfName.trim())}|` : "";
  return new RegExp(
    [
      "`(?<code>[^`\\n]+)`",
      "\\*\\*(?<bold>[^*\\n]+?)\\*\\*",
      "\\*(?<italic>[^*\\s][^*\\n]*?)\\*",
      "(?<link>https?:\\/\\/[^\\s<>]+)",
      `(?<mention>@(?:${self}[\\p{L}\\p{N}_.-]+))`,
    ].join("|"),
    "giu",
  );
}

function pushText(out: Inline[], v: string) {
  if (!v) return;
  const last = out[out.length - 1];
  if (last && last.t === "text") last.v += v;
  else out.push({ t: "text", v });
}

export function parseInline(text: string, selfName: string): Inline[] {
  const out: Inline[] = [];
  const re = inlineRegex(selfName);
  const selfLower = `@${selfName.trim().toLowerCase()}`;
  let last = 0;
  for (let m = re.exec(text); m; m = re.exec(text)) {
    pushText(out, text.slice(last, m.index));
    const g = m.groups ?? {};
    if (g.code !== undefined) out.push({ t: "code", v: g.code });
    else if (g.bold !== undefined) out.push({ t: "bold", v: g.bold });
    else if (g.italic !== undefined) out.push({ t: "italic", v: g.italic });
    else if (g.link !== undefined) {
      const trail = TRAILING_PUNCTUATION.exec(g.link)?.[0] ?? "";
      const url = trail ? g.link.slice(0, -trail.length) : g.link;
      out.push({ t: "link", v: url });
      pushText(out, trail);
    } else if (g.mention !== undefined) {
      out.push({ t: "mention", v: g.mention, self: g.mention.toLowerCase() === selfLower });
    }
    last = m.index + m[0].length;
  }
  pushText(out, text.slice(last));
  return out;
}

export function parseMessage(text: string, selfName: string): Block[] {
  const blocks: Block[] = [];
  const lines = text.split("\n");
  let para: string[] = [];
  const flush = () => {
    if (para.length) {
      blocks.push({ type: "para", inlines: parseInline(para.join("\n"), selfName) });
      para = [];
    }
  };
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i] ?? "";
    const fence = /^```([A-Za-z0-9_+-]{0,20})\s*$/.exec(line);
    if (fence) {
      const end = lines.findIndex((l, j) => j > i && l.trim() === "```");
      if (end !== -1) {
        flush();
        blocks.push({ type: "code", lang: fence[1] ?? "", text: lines.slice(i + 1, end).join("\n") });
        i = end;
        continue;
      }
    }
    if (line.startsWith("> ")) {
      flush();
      const quote = [line.slice(2)];
      while (i + 1 < lines.length && (lines[i + 1] ?? "").startsWith("> ")) quote.push((lines[++i] ?? "").slice(2));
      blocks.push({ type: "quote", inlines: parseInline(quote.join("\n"), selfName) });
      continue;
    }
    para.push(line);
  }
  flush();
  return blocks;
}
