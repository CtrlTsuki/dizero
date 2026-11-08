# 0003. QuickJS-NG (via rquickjs) as the plugin runtime

Status: accepted (2026-11-07)

## Context

Plugins must be easy to write, cheap when idle, isolated from each other and from the app, interruptible, and memory-capped. The directive asked to compare QuickJS, Boa, V8 and Wasmtime.

## Comparison

| Runtime | Startup / memory | Isolation controls | Ergonomics | Notes |
|---|---|---|---|---|
| **QuickJS-NG (rquickjs 0.14)** | Runtime + context in well under a millisecond; ~100–250 KiB heap for the example plugins (measured in the Plugins page) | Per-runtime memory limit, interrupt handler for time budgets, separate heaps per runtime, exact heap measurement (`memory_usage`) | Plain JavaScript, ES2023 | C library; compiled cleanly with MSVC (verified first, as a spike) |
| Boa | Pure Rust | Fewer resource controls; slower interpreter | JavaScript | Larger compile footprint, less mature |
| V8 (rusty_v8 / deno_core) | Fast JIT but tens of MB per isolate | Isolates, heap limits | JavaScript/TypeScript | Too heavy for "cheap when idle"; large build |
| Wasmtime | Fuel/epoch interruption, strong sandbox | Excellent | Authors must compile to Wasm; UI-centric plugins awkward | Good future option for compute-heavy plugins |

## Decision

QuickJS-NG through rquickjs, one runtime per plugin, all runtimes owned by a single host thread. Plugins communicate with the host only through JSON strings and a small set of native functions installed per granted permission.

## Verified properties (tests in `crates/dz-core/tests/plugin_host.rs`)

* Infinite loops are interrupted within the time budget; other plugins keep working.
* Memory bombs hit the per-plugin cap without affecting the host.
* No `require`, `process`, `std`, `os`, `fetch`, `XMLHttpRequest`, `WebSocket` or host internals are reachable.
* Storage quotas, permission checks, undeclared-command refusal, lazy activation, idle unload and auto-faulting all behave as specified.

## Limits we accept

* Handlers are synchronous; there are no timers. This keeps idle CPU at zero and the model simple. Async APIs would arrive with `network.request` in a later API version.
* QuickJS is a C library; a memory-safety bug in it would be inside our process. Mitigations: plugins are user-approved, integrity-checked, and limited to a small attack surface (no I/O). Moving the host into a separate process is on the roadmap for third-party plugin distribution.
