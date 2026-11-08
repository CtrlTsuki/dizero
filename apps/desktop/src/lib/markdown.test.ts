import { describe, expect, it } from "vitest";
import { parseMessage, type Block, type Inline } from "./markdown";

const para = (blocks: Block[]): Inline[] => {
  const b = blocks[0];
  if (!b || b.type !== "para") throw new Error("expected paragraph");
  return b.inlines;
};

describe("parseMessage", () => {
  it("returns plain text as one text inline", () => {
    expect(para(parseMessage("hello world", "Ada"))).toEqual([{ t: "text", v: "hello world" }]);
  });

  it("parses bold, italic and inline code", () => {
    const out = para(parseMessage("a **b** *c* `d`", "Ada"));
    expect(out).toEqual([
      { t: "text", v: "a " },
      { t: "bold", v: "b" },
      { t: "text", v: " " },
      { t: "italic", v: "c" },
      { t: "text", v: " " },
      { t: "code", v: "d" },
    ]);
  });

  it("never interprets HTML", () => {
    const out = para(parseMessage("<img src=x onerror=alert(1)>", "Ada"));
    expect(out).toEqual([{ t: "text", v: "<img src=x onerror=alert(1)>" }]);
  });

  it("extracts fenced code blocks with language", () => {
    const blocks = parseMessage("before\n```js\nconst x = 1;\n```\nafter", "Ada");
    expect(blocks.map((b) => b.type)).toEqual(["para", "code", "para"]);
    const code = blocks[1];
    expect(code).toEqual({ type: "code", lang: "js", text: "const x = 1;" });
  });

  it("leaves an unterminated fence as text", () => {
    const blocks = parseMessage("```js\nnever closed", "Ada");
    expect(blocks).toHaveLength(1);
    expect(blocks[0]?.type).toBe("para");
  });

  it("marks mentions and flags self mentions", () => {
    const out = para(parseMessage("hi @Ada and @Nova", "Ada"));
    expect(out).toContainEqual({ t: "mention", v: "@Ada", self: true });
    expect(out).toContainEqual({ t: "mention", v: "@Nova", self: false });
  });

  it("recognises links without making them HTML", () => {
    const out = para(parseMessage("see https://example.com/x?y=1 now", "Ada"));
    expect(out).toContainEqual({ t: "link", v: "https://example.com/x?y=1" });
  });

  it("does not treat javascript: as a link", () => {
    const out = para(parseMessage("javascript:alert(1)", "Ada"));
    expect(out.every((i) => i.t !== "link")).toBe(true);
  });

  it("parses quote lines", () => {
    const blocks = parseMessage("> quoted\nnormal", "Ada");
    expect(blocks[0]?.type).toBe("quote");
    expect(blocks[1]?.type).toBe("para");
  });

  it("keeps unmatched markers literal", () => {
    expect(para(parseMessage("2 * 3 = 6 and a**b", "Ada")).map((i) => i.v).join("")).toBe("2 * 3 = 6 and a**b");
  });
});
