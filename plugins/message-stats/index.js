// Message Stats — listens to new messages (events.messages), keeps tallies in
// plugin-private storage (storage.local) and renders a dock panel (ui.panels).
// It never sees attachments and has no way to send data off the machine.

const EMPTY = { messages: 0, words: 0, mine: 0, authors: {}, since: null };
const MILESTONES = [10, 50, 100, 250, 500, 1000];

const load = () => dz.storage.get("stats") ?? { ...EMPTY, authors: {} };
const save = (s) => dz.storage.set("stats", s);

dz.plugin({
  activate() {
    const s = load();
    if (!s.since) {
      s.since = Date.now();
      save(s);
    }
  },

  events: {
    messageCreated(msg) {
      const s = load();
      s.messages += 1;
      s.words += msg.content.split(/\s+/).filter(Boolean).length;
      if (msg.isSelf) s.mine += 1;
      s.authors[msg.authorName] = (s.authors[msg.authorName] ?? 0) + 1;
      save(s);
      if (MILESTONES.includes(s.messages)) {
        dz.notify("Message Stats", `${s.messages} messages counted since you installed me.`);
      }
      dz.ui.refresh("activity");
    },
  },

  panels: {
    activity: {
      render() {
        const s = load();
        if (s.messages === 0) {
          return [
            { type: "heading", text: "Activity" },
            { type: "empty", text: "No new messages yet. Counts start from when this plugin was enabled." },
          ];
        }
        const top = Object.entries(s.authors)
          .sort((a, b) => b[1] - a[1])
          .slice(0, 5);
        const max = top.length ? top[0][1] : 1;
        return [
          { type: "heading", text: "Activity" },
          { type: "stat", label: "Messages", value: s.messages },
          { type: "stat", label: "Words", value: s.words, hint: `${(s.words / s.messages).toFixed(1)} per message` },
          { type: "stat", label: "Sent by you", value: s.mine },
          { type: "divider" },
          { type: "text", text: "Most active", tone: "muted" },
          ...top.map(([name, n]) => ({ type: "progress", label: `${name} (${n})`, value: n / max })),
          { type: "divider" },
          { type: "button", action: "reset", label: "Reset counts", variant: "ghost" },
        ];
      },
      action(id) {
        if (id === "reset") save({ ...EMPTY, authors: {}, since: Date.now() });
      },
    },
  },
});
