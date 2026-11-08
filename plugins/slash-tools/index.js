// Slash Tools — a minimal DZSDK v1 plugin.
// Each command receives the text typed after it and returns
// { replaceWith } to rewrite the composer. Nothing is sent until you press Enter.

const withSuffix = (text, suffix) => (text.trim() ? `${text.trim()} ${suffix}` : suffix);

function roll(spec) {
  const m = /^(\d{0,2})d(\d{1,4})$/i.exec(spec.trim() || "1d6");
  if (!m) return null;
  const count = Math.min(Math.max(parseInt(m[1] || "1", 10), 1), 20);
  const sides = Math.min(Math.max(parseInt(m[2], 10), 2), 1000);
  const rolls = Array.from({ length: count }, () => 1 + Math.floor(Math.random() * sides));
  const total = rolls.reduce((a, b) => a + b, 0);
  return `🎲 ${count}d${sides}: ${rolls.join(" + ")}${count > 1 ? ` = ${total}` : ""}`;
}

dz.plugin({
  commands: {
    shrug: { run: (args) => ({ replaceWith: withSuffix(args, "¯\\_(ツ)_/¯") }) },
    tableflip: { run: (args) => ({ replaceWith: withSuffix(args, "(╯°□°)╯︵ ┻━┻") }) },
    roll: {
      run(args) {
        const result = roll(args);
        return result
          ? { replaceWith: result }
          : { ephemeral: "Use dice notation like 2d6 or d20." };
      },
    },
    time: {
      run() {
        const d = new Date();
        const pad = (n) => String(n).padStart(2, "0");
        return { replaceWith: `It's ${pad(d.getHours())}:${pad(d.getMinutes())} here.` };
      },
    },
  },
});
