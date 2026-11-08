// DZSDK v1 — type definitions for DiZero plugins.
//
// Plugins run inside an isolated QuickJS runtime. Only ECMAScript built-ins
// and the global `dz` object exist: there is no `fetch`, `require`, `process`,
// DOM, filesystem or timers. Every handler is synchronous and must finish
// within the per-call time budget (200 ms by default; 750 ms for activation).
//
// Usage in an editor: add `/// <reference path="path/to/dzsdk-types/index.d.ts" />`
// to the top of your plugin's entry file, or list this package in `types`.

declare namespace DZ {
  type Permission =
    | "ui.panels"
    | "ui.sidebar"
    | "ui.themes"
    | "storage.local"
    | "events.messages"
    | "commands.slash"
    | "notifications";

  /** A JSON-serialisable value. */
  type Json = string | number | boolean | null | Json[] | { [key: string]: Json };

  /** What `events.messageCreated` receives. Requires `events.messages`. */
  interface Message {
    id: string;
    conversationId: string;
    authorId: string;
    authorName: string;
    isSelf: boolean;
    isBot: boolean;
    content: string;
    /** Milliseconds since the Unix epoch. */
    createdAt: number;
  }

  /** Returned from a command's `run`. Both fields are optional. */
  interface CommandResult {
    /** Replaces the composer text. The user still presses Enter to send. */
    replaceWith?: string;
    /** A note only the user sees; never sent. */
    ephemeral?: string;
  }

  interface Command {
    run(args: string): CommandResult | null | undefined;
  }

  type Tone = "default" | "muted" | "accent" | "success" | "warning" | "danger";

  /** Declarative view nodes. DiZero renders them as text; HTML is never interpreted. */
  type ViewNode =
    | { type: "heading"; text: string }
    | { type: "text"; text: string; tone?: Tone }
    | { type: "stat"; label: string; value: string | number; hint?: string }
    | { type: "progress"; label: string; value: number /* 0..1 */ }
    | {
        type: "list";
        items: { id: string; text: string; meta?: string; action?: string; actionLabel?: string }[];
      }
    | { type: "button"; action: string; label: string; variant?: "primary" | "ghost" | "danger" }
    | { type: "input"; id: string; placeholder?: string; value?: string; maxLength?: number }
    | { type: "divider" }
    | { type: "empty"; text: string };

  interface Panel {
    render(): ViewNode[];
    /**
     * Called when a button or list-item action fires. `values` holds the
     * current text of every input in the panel by id, plus `itemId` for
     * list-item actions. The panel is re-rendered afterwards.
     */
    action?(actionId: string, values: Record<string, string>): void;
  }

  interface PluginDefinition {
    /** Runs once when the plugin is first used (plugins load lazily). */
    activate?(): void;
    /** Runs before the runtime is unloaded (idle unload, disable, shutdown). */
    deactivate?(): void;
    /** Keys must match `contributes.commands[].name`. Requires `commands.slash`. */
    commands?: Record<string, Command>;
    /** Keys must match `contributes.events`. Requires `events.messages`. */
    events?: { messageCreated?(message: Message): void };
    /** Keys must match `contributes.panels[].id`. Requires `ui.panels` / `ui.sidebar`. */
    panels?: Record<string, Panel>;
  }

  interface Storage {
    /** `undefined` when the key doesn't exist. */
    get<T extends Json = Json>(key: string): T | undefined;
    /** Throws when the 64 KiB value limit or the per-plugin quota is exceeded. */
    set(key: string, value: Json): void;
    delete(key: string): void;
    keys(): string[];
  }

  interface Api {
    readonly apiVersion: "1";
    /** Register the plugin. Call exactly once. */
    plugin(definition: PluginDefinition): void;
    readonly log: {
      info(...args: unknown[]): void;
      warn(...args: unknown[]): void;
      error(...args: unknown[]): void;
    };
    /** Throws "permission denied" unless `storage.local` was granted. */
    readonly storage: Storage;
    /** In-app toast, rate-limited to 5 per 10 s. Requires `notifications`. */
    notify(title: string, body?: string): void;
    readonly ui: {
      /** Re-render one of your panels if it is on screen. Requires `ui.panels` or `ui.sidebar`. */
      refresh(panelId: string): void;
    };
  }
}

declare const dz: DZ.Api;
