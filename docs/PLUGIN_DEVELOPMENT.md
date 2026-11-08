# Plugin development (DZSDK v1)

DiZero plugins are small JavaScript programs that run in a sandbox. They can add composer commands, side-panel and sidebar UI, react to new messages, store private data and show notifications, each only with the user's permission. Themes are plugins too, made of colour tokens only.

Start from the template in [`packages/dzsdk/template`](../packages/dzsdk/template) and the type definitions in [`packages/dzsdk-types/index.d.ts`](../packages/dzsdk-types/index.d.ts). The bundled examples in [`plugins/`](../plugins) are complete, tested references.

## The runtime

* **QuickJS-NG**, ES2023. One isolated runtime per plugin.
* **Available:** ECMAScript built-ins (`JSON`, `Math`, `Date`, `Map`, `Promise`, …) and the global `dz`.
* **Not available, by design:** `fetch` and any network, files, processes, `setTimeout`/timers, the DOM, `require`/`import` of other files, and DiZero internals.
* **Synchronous handlers.** Each call must finish within **200 ms** (activation: 750 ms). Work that runs longer is interrupted and counts as a failure.
* **Memory cap:** 16 MiB per plugin.
* **Lazy.** Your code doesn't run until something needs it (a command, a visible panel, a subscribed event). Idle runtimes are unloaded (after 60 s in Eco mode, 10 min in Balanced), so **keep state in `dz.storage`**, not in variables.
* **Failure policy.** Five consecutive failures stop the plugin until the user restarts it from the Plugins page.

## Folder layout

```
my-plugin/
  dizero-plugin.json   required
  index.js             your entry file (any name, declared in the manifest)
  README.md            optional
```

Allowed files: `.js .json .md .txt .png .svg` and `LICENSE`, `README`, `NOTICE`, `CHANGELOG`. Limits: 200 files, 2 MiB per file, 5 MiB total, 5 folder levels. Symbolic links are rejected. Dot-files and dot-folders (like `.git`) are ignored.

## Manifest: `dizero-plugin.json`

```json
{
  "id": "com.example.hello",
  "name": "Hello",
  "version": "0.1.0",
  "description": "A starting point: one command, one panel, one saved value.",
  "author": "Your name",
  "apiVersion": "1",
  "kind": "script",
  "category": "productivity",
  "entry": "index.js",
  "permissions": ["commands.slash", "ui.panels", "storage.local"],
  "contributes": {
    "commands": [{ "name": "hello", "description": "Say hello", "usage": "/hello [name]" }],
    "panels": [{ "id": "main", "title": "Hello", "placement": "dock", "icon": "sparkles" }],
    "events": []
  },
  "dependencies": { "com.example.core": "^1.2" },
  "engine": ">=0.1.0",
  "homepage": "https://example.com/hello"
}
```

| Field | Rules |
|---|---|
| `id` | Reverse-domain, lowercase letters, digits and dashes, 3–64 chars. Never change it after publishing. |
| `version` | Semantic version. Installing a lower version than installed is refused (users roll back instead). |
| `apiVersion` | `"1"` for this SDK. |
| `kind` | `script` or `theme`. |
| `category` | `interface`, `appearance`, `messaging`, `productivity`, `accessibility`, `media`, `integration`, `developer`. |
| `entry` | Relative path to a `.js` file inside the folder. Required for `script`. |
| `permissions` | See below. Unknown or not-yet-supported permissions fail validation. |
| `contributes.commands` | Name: lowercase slug starting with a letter, max 32. Description 1–120 chars. Max 20. |
| `contributes.panels` | `id` slug, `title` 1–40 chars, `placement` `dock` or `sidebar`, optional `icon`: `bar-chart`, `notebook`, `list`, `sparkles`, `clock`, `star`, `puzzle`, `bell`, `bookmark`, `zap`. Max 8. |
| `contributes.events` | `messageCreated` is the only event in v1. |
| `dependencies` | Plugin id → semver range. The plugin can't be enabled until each dependency is installed, enabled and matches. |
| `signature` | Reserved. Recorded and shown as "not verified" in this version. |

Everything your code implements must be declared in `contributes`: DiZero builds menus and tabs from the manifest without running your code, and refuses to call anything undeclared.

## Permissions

| Permission | Grants | Risk shown to users |
|---|---|---|
| `commands.slash` | Composer `/commands` that rewrite the draft (the user still sends it) | Review |
| `ui.panels` | Panels in the right-hand dock | Low |
| `ui.sidebar` | A compact section at the bottom of the channel list | Low |
| `ui.themes` | Theme tokens (theme plugins) | Low |
| `storage.local` | Private key/value storage, 512 KiB quota | Low |
| `events.messages` | Text, author name and time of new messages | Review |
| `notifications` | In-app toasts, max 5 per 10 s | Low |

Planned and currently rejected: `network.request` (domain-restricted), `media.controls`, `fs.read`, `fs.write`, `process.spawn`, `clipboard.write`.

Users choose which requested permissions to grant and can revoke them later. Code against a denied permission throws `Error("permission denied: …")`. Handle that if your plugin can work in a reduced mode.

## The `dz` API

```js
dz.plugin({
  activate() {},                 // first use
  deactivate() {},               // before unload
  commands: {
    hello: { run(args) { return { replaceWith: `Hello, ${args}` }; } },
  },
  events: {
    messageCreated(msg) { /* { id, conversationId, authorId, authorName, isSelf, isBot, content, createdAt } */ },
  },
  panels: {
    main: {
      render() { return [{ type: "heading", text: "Hi" }]; },
      action(actionId, values) { /* values: input text by id, plus itemId for list actions */ },
    },
  },
});

dz.storage.get(key)            // → value or undefined
dz.storage.set(key, jsonValue) // throws past 64 KiB per value or the quota
dz.storage.delete(key)
dz.storage.keys()
dz.notify(title, body)
dz.ui.refresh(panelId)         // re-render a panel that's on screen
dz.log.info(...) / warn(...) / error(...)   // Developer Center log, tagged plugin:<id>
```

A command returns `{ replaceWith }` to rewrite the draft, `{ ephemeral }` to show a note only the user sees, or nothing.

## View nodes

| Node | Fields |
|---|---|
| `heading` | `text` |
| `text` | `text`, `tone?`: `default` `muted` `accent` `success` `warning` `danger` |
| `stat` | `label`, `value` (string or number), `hint?` |
| `progress` | `label`, `value` 0–1 |
| `list` | `items`: `{ id, text, meta?, action?, actionLabel? }` (max 100) |
| `button` | `action`, `label`, `variant?`: `primary` `ghost` `danger` |
| `input` | `id`, `placeholder?`, `value?`, `maxLength?` (Enter triggers the next button) |
| `divider` | |
| `empty` | `text` |

Rendering is text-only; HTML in any field is shown literally. Unknown nodes are dropped (and logged), text is truncated (2,000 chars for text, 120 for labels), and at most 100 nodes render.

## Themes

```json
{
  "id": "com.example.midnight", "name": "Midnight", "version": "1.0.0",
  "description": "Deep blue surfaces.", "author": "You", "apiVersion": "1",
  "kind": "theme", "category": "appearance", "permissions": ["ui.themes"],
  "theme": { "label": "Midnight", "tokens": { "background": "#0b0f1a", "accent": "#5b8cff" } }
}
```

Tokens: `background`, `surface`, `surfaceSecondary`, `surfaceElevated`, `accent`, `accentHover`, `accentMuted`, `textPrimary`, `textSecondary`, `textMuted`, `border`, `success`, `warning`, `danger`. Values must be `#rgb` or `#rrggbb`. Unset tokens keep DiZero's defaults; hover and muted accents are derived when only `accent` is set.

## Workflow

1. **Validate:** Developer Center → Validate a plugin → choose your folder. Every problem is listed at once; nothing is installed.
2. **Install:** Plugins → Install from folder. DiZero copies the folder and records a SHA-256 of its files.
3. **Enable:** review permissions and enable.
4. **Iterate:** edit your source, bump `version`, install again. Grants carry over unless you request new permissions.
5. **Debug:** Developer Center → Runtime log (filter "Plugins only"); the Plugins page shows calls, failures, average time, heap and the last error.

Editing files inside DiZero's installed copy fails the integrity check and blocks the plugin; always reinstall from your source folder.

## Testing your plugin

Plugins are plain JS, so you can unit-test handlers by stubbing a `dz` global in Node. DiZero's own examples are tested through the real host in `crates/dz-core/tests/bundled_plugins.rs`; copy that pattern if you contribute a bundled plugin.
