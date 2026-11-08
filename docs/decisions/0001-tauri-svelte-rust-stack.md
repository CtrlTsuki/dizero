# 0001. Tauri 2 + Rust core + Svelte 5 for the desktop app

Status: accepted (2026-11-07)

## Context

DiZero must be light on memory, start fast on Windows, look like a premium app, and host third-party plugins. The directive proposed Tauri 2, Rust, Svelte and SQLite, and asked for any replacement to be justified with measurements.

## Options considered

| Option | For | Against |
|---|---|---|
| **Tauri 2 + WebView2 + Svelte** | System webview is shared with Windows (no bundled Chromium); Rust backend; small binary; mature IPC permission model | WebView2 still spawns 5–6 helper processes, which dominate memory |
| Electron | Most compatible with existing Discord client mods | Bundles its own Chromium and Node; typically several hundred MB idle; conflicts with the product's purpose |
| Native Rust UI (egui, Iced, Slint, GPUI) | Single process, potentially much lower memory | Weaker text shaping, accessibility and rich-text rendering on Windows today; custom theming of a chat UI and plugin-rendered UI is far more work; ecosystem churn |
| WinUI 3 / C# | Native Windows look | Leaves the Rust core; plugin hosting and cross-platform future harder |

## Decision

Keep the proposed stack. Svelte 5 (runes, no SvelteKit router) keeps the bundle small: 81 KB entry chunk plus 59 KB shared at launch after code-splitting the tool pages.

## Evidence

Release build, idle: private working set 104.8 MiB total, of which the Rust core is 5.2 MiB and WebView2 is the rest ([benchmark](../benchmarks/2026-11-08-foundation.md)). A native UI could cut the WebView2 share, but no measurement here justifies a rewrite: the target is within reach by configuration ([0005](0005-webview2-arguments.md)), and rendering quality, accessibility and plugin UI would regress.

## Revisit when

Measurements across real workloads (large histories, many plugins, real integrations) show WebView2 overhead dominating in a way configuration can't fix, and a native toolkit's Windows text and accessibility support is demonstrably adequate.
