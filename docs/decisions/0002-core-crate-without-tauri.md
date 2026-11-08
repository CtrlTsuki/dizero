# 0002. Application core as a Tauri-free crate

Status: accepted (2026-11-07)

## Decision

All domain logic (storage, models, providers, plugin host, metrics, logging) lives in `crates/dz-core`, which does not depend on Tauri. The Tauri shell (`apps/desktop/src-tauri`) only validates IPC input, calls the core, and forwards events.

## Why

* The core is testable headlessly and fast: 99 behaviour tests (plus 58 ts-rs contract exports) run in a few seconds without a window or webview.
* Module boundaries stay honest: the core can't reach UI concerns.
* Contracts are generated from the core's types with `ts-rs` into `apps/desktop/src/lib/bindings`, so TypeScript breaks at compile time when a Rust type changes. This caught two real mismatches during development (a snake_case field in a tagged enum, and a new settings field).

## Consequences

* Effects the core needs from the shell (plugin notifications, refreshes, state changes) go through traits (`HostSink`, `PluginStorage`), implemented by the shell.
* A few response shapes that only exist in the shell (e.g. `SettingsView`, `PerfView`) are typed by hand in `lib/api.ts`. Moving them into the core would let ts-rs generate them too; tracked in the roadmap.
