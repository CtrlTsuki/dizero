# Architecture

DiZero is three layers with one rule between them: **the UI never owns truth, and the core never knows about the UI.**

```
┌──────────────────────────── WebView2 (Svelte 5 + TypeScript) ───────────────────────────┐
│  Rail · Sidebar · MessageList · Composer · Dock      Plugins · Performance · Developer · Settings │
│  Runes stores (app, workspace, plugins, toasts)       Pure libs: markdown, theme, slash, window   │
│  lib/api.ts — typed IPC, contracts generated from Rust by ts-rs (src/lib/bindings)                │
└───────────────────────▲─────────────── IPC (allow-listed commands) ──────┬──────────────────────┘
                        │ events: message-created, typing, plugin-*        │ invoke
┌───────────────────────┴──────────── Tauri shell (apps/desktop/src-tauri) ▼──────────────────────┐
│  commands.rs  validation, routing, Rust-side file dialogs        events.rs  TauriSink → UI events  │
│  state.rs     AppState (db, registry, plugin host, settings, sampler, logs), graceful shutdown     │
└────────────────────────────────────────────────▲─────────────────────────────────────────────────┘
                                                 │ plain Rust calls
┌──────────────────────────────── dz-core (crates/dz-core, no Tauri) ──────────────────────────────┐
│  models      Unified, provider-independent data model + Capabilities                              │
│  integrations Provider trait; planned integrations listed honestly                                 │
│  demo        Local demo provider (seeded workspace, Nova bot)                                      │
│  store       Queries: snapshot, pagination, FTS5 search, unread, prune, stats                      │
│  db          SQLite: migrations (user_version), WAL, integrity check + quarantine, backup           │
│  settings    Typed settings, normalisation, import/export, PerfMode → PerfProfile, WebView2 args   │
│  plugins/    manifest · permissions · registry (install/integrity/upgrade/rollback) · host · view  │
│  perf        Process-tree sampler (Toolhelp + GetProcessMemoryInfo + GetProcessTimes)              │
│  logging     File (size-capped, rotated) + ring buffer for the Developer Center                    │
└──────────────────────────────────────────────────────────────────────────────────────────────────┘
                        SQLite file (%APPDATA%\com.dizero.desktop\dizero.db)
```

## Why the core is a separate crate

`dz-core` compiles and tests without Tauri, a webview or a window (99 behaviour tests run in seconds). The shell is thin: it validates input, calls the core, and turns results into IPC responses and events. Swapping the shell (another UI technology, a headless test harness) would not touch the core. See [ADR 0002](docs/decisions/0002-core-crate-without-tauri.md).

## Data model and providers

`models.rs` defines `Community`, `Conversation`, `Message`, `User`, `Attachment` and `Capabilities` with no Discord-specific fields. A provider implements:

```rust
pub trait Provider: Send + Sync {
    fn integration_id(&self) -> &'static str;
    fn capabilities(&self) -> Capabilities;
    fn info(&self) -> IntegrationInfo;
    fn send_message(&self, db: &Database, conversation_id: &str, content: &str) -> Result<SendOutcome>;
    fn delete_message(&self, db: &Database, message_id: &str) -> Result<()>;
}
```

The shell routes each command to the provider that owns the conversation (`store::conversation_integration`). The UI reads `Capabilities` before showing a feature: for example, the delete action appears only when `delete_own_messages` is true, and the composer is disabled when `send_messages` is false. Planned integrations (Discord) appear in Settings → Integrations with status **Planned**, never as working controls.

Messages are cached in SQLite and loaded in pages (`before` cursor on `(created_at, id)`). The UI keeps a bounded window per conversation (150 / 300 / 800 messages by performance mode). Paging back past the window trims the newest messages and shows **Jump to present**.

## IPC and security boundaries

* **Allow-listed commands.** `build.rs` lists every command; Tauri generates one permission per command, and `capabilities/default.json` grants exactly those plus `core:event` listen/unlisten. No filesystem, shell, dialog or opener permission is granted to the webview.
* **Dialogs in Rust.** Folder pickers (plugin install/validate), save dialogs (backup, settings export) and file pickers (settings import) are opened by Rust; the webview never supplies a path. See [ADR 0008](docs/decisions/0008-dialogs-in-rust.md).
* **CSP:** `default-src 'self'`, `script-src 'self'`, `connect-src ipc: http://ipc.localhost`, no objects, frames or base URIs; `freezePrototype` on.
* **Links** from messages open only after a confirmation dialog, and only `http(s)` URLs are accepted by the `open_link` command.
* **Errors** cross IPC as `{ code, message }` (`IpcError`); messages never contain secrets.

## Plugin system (DZSDK v1)

```
manifest (dizero-plugin.json) ─► validate ─► install (copy + SHA-256) ─► permission review
     ─► register (Dormant: no memory) ─► first use: new QuickJS runtime ─► activate()
     ─► calls under time budget ─► idle unload (Eco 60 s, Balanced 10 min) ─► Dormant
     ─► repeated failures ─► Faulted (auto-disabled until restarted by the user)
```

* **One QuickJS-NG runtime per plugin** with its own heap and memory cap (16 MiB), all on one dedicated host thread. See [ADR 0003](docs/decisions/0003-quickjs-plugin-runtime.md).
* **Time budget** per call (200 ms; 750 ms for activation) enforced by the QuickJS interrupt handler. The Fault Lab example shows an infinite loop stopped in about 300 ms end to end.
* **No ambient authority.** The sandbox has only ECMAScript built-ins plus `dz`. Native host functions exist only for granted permissions and re-check them; denied APIs throw `permission denied`.
* **Declarative UI.** Plugins return view nodes (heading, text, stat, progress, list, button, input, divider, empty). The host sanitises them (`view.rs`: allow-listed types, length caps, 100-node cap) and Svelte renders them as text. See [ADR 0007](docs/decisions/0007-declarative-plugin-ui.md).
* **Contributions are declared** in the manifest (`contributes.commands/panels/events`). The UI builds menus and dock tabs from the manifest without running plugin code, and the host refuses to call anything undeclared.
* **Integrity.** A SHA-256 over every installed file is recorded at install and re-checked before each load; a mismatch blocks the plugin.
* **Upgrades** keep grants for permissions still requested; any new permission requires a fresh review. The previous version is kept for **Roll back**. Downgrades through install are refused.

## Threads and background work

| Thread / task | When it runs |
|---|---|
| Main (Tauri event loop) | Always; handles window events |
| Tokio async workers (Tauri) | Only while IPC commands run |
| `dz-plugin-host` | Blocks on a channel; wakes for plugin calls or the next idle-unload deadline |
| Demo reply timers | One short-lived task per planned Nova reply |
| Performance sampling | Only while the Performance Center page is open and visible |

Nothing polls in the background while the app is idle. The plugins page refreshes runtime stats every 4 s only while it is open.

## Startup and shutdown

1. Logger → database (open, `quick_check`, migrate; quarantine and recreate if corrupt) → settings → seed demo (idempotent) → prune cache (5,000 messages per conversation).
2. Plugin registry builds load requests (integrity + dependency checks); plugins are registered **dormant**.
3. The main window is created in Rust with WebView2 arguments derived from settings ([ADR 0005](docs/decisions/0005-webview2-arguments.md)), hidden until the UI calls `app_ready` after its first paint (a 5 s safety net shows it regardless).
4. On exit: the plugin host deactivates loaded plugins and joins its thread, the WAL is checkpointed, logs are flushed.

## Persistence

Schema v1 tables: `app_settings`, `integrations`, `communities`, `conversations`, `users`, `community_members`, `messages` (+ `message_fts` FTS5 with sync triggers), `plugin_registry`, `plugin_permissions`, `plugin_storage`. One connection behind a mutex, WAL mode, 2 MiB page cache, prepared-statement cache. See [ADR 0006](docs/decisions/0006-sqlite-single-connection.md).

No credentials are stored anywhere in this build. When integrations arrive, secrets will live in Windows Credential Manager, never in SQLite (see [SECURITY.md](SECURITY.md)).
