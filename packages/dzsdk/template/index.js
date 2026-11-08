/// <reference path="../../dzsdk-types/index.d.ts" />
// A DZSDK v1 starter plugin. Copy this folder, change the id in
// dizero-plugin.json, then validate it in DiZero's Developer Center.

dz.plugin({
  activate() {
    dz.log.info("Hello plugin loaded");
  },

  commands: {
    hello: {
      run(args) {
        const name = args.trim() || "there";
        dz.storage.set("greeted", (dz.storage.get("greeted") ?? 0) + 1);
        dz.ui.refresh("main");
        return { replaceWith: `Hello, ${name}!` };
      },
    },
  },

  panels: {
    main: {
      render() {
        return [
          { type: "heading", text: "Hello" },
          { type: "stat", label: "Greetings sent", value: dz.storage.get("greeted") ?? 0 },
          { type: "button", action: "reset", label: "Reset", variant: "ghost" },
        ];
      },
      action(id) {
        if (id === "reset") dz.storage.set("greeted", 0);
      },
    },
  },
});
