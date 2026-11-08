// Fault Lab — a plugin that misbehaves on purpose. Each command shows one
// containment mechanism of the DiZero plugin host:
//   /hang   → per-call time budget (interrupt handler)
//   /bomb   → per-plugin memory cap
//   /crash  → exception isolation; repeated failures auto-disable the plugin
//   /escape → there is nothing to escape to: no fs, network, process or DOM

dz.plugin({
  commands: {
    hang: {
      run() {
        for (;;) {}
      },
    },
    bomb: {
      run() {
        const hoard = [];
        for (;;) hoard.push("x".repeat(1024) + hoard.length);
      },
    },
    crash: {
      run() {
        throw new Error("Fault Lab crashed on purpose");
      },
    },
    escape: {
      run() {
        const probes = ["require", "process", "std", "os", "fetch", "XMLHttpRequest", "WebSocket", "document", "window", "__TAURI__", "__TAURI_INTERNALS__"];
        const reachable = probes.filter((n) => typeof globalThis[n] !== "undefined");
        let storage;
        try {
          dz.storage.get("x");
          storage = "storage reachable";
        } catch (e) {
          storage = `storage blocked (${e.message})`;
        }
        return {
          ephemeral: reachable.length
            ? `Reachable globals: ${reachable.join(", ")}; ${storage}`
            : `Nothing reachable: ${probes.length} host globals checked, ${storage}.`,
        };
      },
    },
  },
});
