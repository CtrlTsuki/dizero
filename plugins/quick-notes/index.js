// Quick Notes — demonstrates panels with inputs and per-item actions, plus a
// compact sidebar section. Notes live in plugin-private storage.

const MAX_NOTES = 50;
const notes = () => dz.storage.get("notes") ?? [];
const pinnedId = () => dz.storage.get("pinned") ?? null;

function ago(ts) {
  const s = Math.max(0, Math.round((Date.now() - ts) / 1000));
  if (s < 60) return "just now";
  if (s < 3600) return `${Math.floor(s / 60)} min ago`;
  if (s < 86400) return `${Math.floor(s / 3600)} h ago`;
  return `${Math.floor(s / 86400)} d ago`;
}

dz.plugin({
  panels: {
    notes: {
      render() {
        const list = notes();
        const pin = pinnedId();
        return [
          { type: "input", id: "text", placeholder: "Write a note…", maxLength: 500 },
          { type: "button", action: "add", label: "Add note" },
          { type: "divider" },
          list.length === 0
            ? { type: "empty", text: "No notes yet. They stay on this computer." }
            : {
                type: "list",
                items: list.map((n) => ({
                  id: n.id,
                  text: n.text,
                  meta: `${n.id === pin ? "Pinned, " : ""}${ago(n.at)}`,
                  action: n.id === pin ? "unpin" : "pin",
                  actionLabel: n.id === pin ? "Unpin" : "Pin",
                })),
              },
          ...(list.length ? [{ type: "button", action: "clear", label: "Delete all notes", variant: "danger" }] : []),
        ];
      },
      action(id, values) {
        const list = notes();
        if (id === "add") {
          const text = String(values.text ?? "").trim();
          if (!text) return;
          list.unshift({ id: `n${Date.now().toString(36)}`, text: text.slice(0, 500), at: Date.now() });
          dz.storage.set("notes", list.slice(0, MAX_NOTES));
        } else if (id === "pin") {
          dz.storage.set("pinned", values.itemId);
        } else if (id === "unpin") {
          dz.storage.delete("pinned");
        } else if (id === "clear") {
          dz.storage.set("notes", []);
          dz.storage.delete("pinned");
        }
        dz.ui.refresh("pinned");
      },
    },
    pinned: {
      render() {
        const pin = pinnedId();
        const note = notes().find((n) => n.id === pin);
        return note
          ? [{ type: "text", text: note.text }]
          : [{ type: "text", text: "Pin a note from the Notes panel to keep it here.", tone: "muted" }];
      },
    },
  },
});
