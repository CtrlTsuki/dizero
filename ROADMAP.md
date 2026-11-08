# Roadmap

Status key: **Done**, **Partial**, **Next**, **Planned**, **Blocked** (external restriction).

## Phase 0: Research and validation — Done

Environment inspected; Tauri 2, rquickjs and WebView2 behaviour validated against current docs and spikes; plugin runtimes compared ([ADR 0003](docs/decisions/0003-quickjs-plugin-runtime.md)); Discord policy and Vencord/endcord architecture and licenses reviewed ([COMPATIBILITY.md](docs/COMPATIBILITY.md)); baseline measured ([benchmark](docs/benchmarks/2026-11-08-foundation.md)).

## Phase 1: Working foundation — Done

Tauri app, Rust core, Svelte UI and design system, navigation, settings, SQLite with migrations and recovery, local demo mode, lifecycle and graceful shutdown, logging, Performance Center, Developer Center.

## Phase 2: Plugin foundation — Done (v1 scope)

DZSDK v1, manifest validation, plugin manager, granular permissions, QuickJS sandbox, lifecycle with lazy loading and idle unload, integrity checks, upgrade/rollback, four example plugins plus Fault Lab, template, TypeScript definitions, developer guide.

Next in this area:
* **Next:** run the plugin host in a separate low-privilege process.
* **Planned:** `network.request` with per-domain allow-lists and async handlers (API v2); richer view nodes (toggle, select, table, bundled images); plugin signing and a public repository.

## Phase 3: Communication foundation — Next

* Discord **bot** provider: Gateway, REST, rate limits, reconnection, application commands ([plan](docs/DISCORD_INTEGRATION.md)).
* OAuth2 with PKCE for identity and server lists.
* Bot token in Windows Credential Manager.
* Windows system notifications.
* Attachments: upload where supported, previews with size limits.
* Move shell-only response types into `dz-core` so ts-rs generates all contracts.

**Blocked permanently:** personal-account login or user-token clients (Discord policy, DiZero security rules).

## Phase 4: Compatibility — Planned

Per-plugin classification of popular Vencord and endcord plugins, a colour-variable theme converter, clean-room re-implementations of class-2 features. DiZero is MIT OR Apache-2.0, so no Vencord or endcord code is reused ([ADR 0009](docs/decisions/0009-license.md)).

## Phase 5: Advanced features — Planned

Voice and video research track (bot voice, WebRTC, audio devices; no placeholder controls until real), media previews, advanced search filters, multiple profiles, custom key bindings, additional providers (for example Matrix or IRC, which have open client APIs).

## Phase 6: Optimization and hardening — Planned

Competitor and cold-start benchmarks, soak and leak tests, renderer heap profiling, true list virtualization if measurements call for it, dependency auditing (`cargo-deny`, `npm audit`) in CI, security review of the plugin host, accessibility audit with a screen reader.

## Phase 7: Release preparation — Planned

Signed NSIS installer, third-party license notices bundled with it, portable build, updater with signature verification, versioning policy, release notes, user documentation, security contact.
