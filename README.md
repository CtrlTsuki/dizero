# DiZero

A lightweight, extensible desktop communication client for Windows, built on Tauri 2, Rust, Svelte 5 and SQLite.

DiZero's core idea: communication shouldn't cost your computer. The Rust core uses about **5 MiB** of private memory, plugins run in isolated sandboxes that cost nothing until used, and the Performance Center shows exactly what the whole app costs, measured from Windows.

> **Status: Phase 1 foundation (0.1.0).** DiZero runs, persists, and is extensible today, but it connects to **no real service yet**. Everything you see comes from a local, offline demo workspace. Discord support is planned for Phase 3 using only Discord's documented bot and OAuth2 APIs. DiZero will never ask for your Discord password or user token. See [docs/DISCORD_INTEGRATION.md](docs/DISCORD_INTEGRATION.md).

## What works today

| Area | Status | Notes |
|---|---|---|
| Desktop app (Windows 10/11 x64) | Implemented | Tauri 2.11, WebView2, release binary 13 MiB |
| Workspace UI: communities, channels, messages, composer, member list | Implemented | Local demo provider |
| Local demo with a scripted bot (Nova) and typing indicator | Implemented | Mention `@Nova` |
| Full-text search across conversations | Implemented | SQLite FTS5 |
| Quick switcher (`Ctrl+K`), keyboard navigation, context menus | Implemented | |
| Settings with persistence, import/export, reset | Implemented | |
| Themes, accent colour, density, text size, high contrast, reduced motion | Implemented | |
| Performance modes (Eco / Balanced / Performance) | Implemented | Each mode changes real behaviour; see Performance Center |
| Performance Center (per-process memory and CPU, plugin heaps) | Implemented | Whole process tree, Win32 APIs |
| Hardware acceleration toggle | Implemented | Applies on restart |
| DZSDK plugin system: sandbox, permissions, panels, commands, events, storage | Implemented | QuickJS-NG per plugin |
| Plugin manager: install, review permissions, enable, revoke, roll back, uninstall | Implemented | |
| Plugin integrity verification (SHA-256 of installed files) | Implemented | Signature field recorded, **not verified** yet |
| Developer Center: logs, folder validator, SDK reference | Implemented | |
| Windows system notifications | Planned | In-app notifications work today |
| Discord integration (bot / OAuth2) | Planned (Phase 3) | |
| Vencord / endcord plugin compatibility | Researched, mostly incompatible | See [docs/COMPATIBILITY.md](docs/COMPATIBILITY.md) |
| Voice and video | Planned (Phase 5), separate research track | No placeholder controls are shown |
| Installer and auto-update | Planned (Phase 7) | `targets: nsis` is configured but not yet produced or signed |

## Measured resource use

Release build, idle in the demo workspace, Windows 11 26300, Ryzen 5 5600G, WebView2 157. Median of 5 runs ([full report](docs/benchmarks/2026-11-08-foundation.md)):

| | Default (hardware acceleration on) | Hardware acceleration off |
|---|---|---|
| **Private working set, all processes** (Task Manager's number) | **104.8 MiB** | **89.6 MiB** (3 runs) |
| Committed memory (private bytes) | 181 MiB | 124 MiB |
| Launch to first painted window | 685 ms | 718 ms |
| Processes | 6 | 6 |

The stretch target was under 100 MiB idle. DiZero's own Rust process accounts for 5 MiB; the rest is the Microsoft Edge WebView2 runtime (renderer, browser, GPU). Turning hardware acceleration off meets the target, at the cost of about 18% more CPU while scrolling. See [PERFORMANCE.md](PERFORMANCE.md).

## Getting started (development)

Prerequisites: Windows 10/11 x64, [Rust](https://rustup.rs) stable (1.85+), Node.js 22+, Visual Studio Build Tools with the C++ workload, and the WebView2 runtime (preinstalled on Windows 11).

```bash
npm install
```

```bash
npm run dev
```

```bash
npm test
```

Other commands:

| Command | What it does |
|---|---|
| `npm run build` | Release build via `tauri build` (the NSIS installer step hasn't been exercised yet; `npx tauri build --no-bundle` builds just the exe) |
| `npm run check` | Strict TypeScript + Svelte checks (fails on warnings) |
| `npm run test:rust` / `npm run test:web` | Rust: 99 behaviour tests plus 58 that regenerate the TypeScript contracts / frontend unit tests (25) |
| `npm run lint:rust` | Clippy with warnings as errors |
| `npm run measure` | Reproducible memory and startup measurement of the release build |
| `node tests/e2e/smoke.mjs` | End-to-end test against a running app (see [TESTING.md](TESTING.md)) |

## Repository layout

```
apps/desktop/            Svelte 5 UI (src/) and the Tauri shell (src-tauri/)
crates/dz-core/          The application core: storage, models, providers, plugin host, metrics. No Tauri dependency.
plugins/                 Example plugins bundled with the app
packages/dzsdk/          Plugin template
packages/dzsdk-types/    TypeScript definitions for plugin authors
tests/e2e/               End-to-end tests that drive the real app over CDP
scripts/                 Measurement tooling
docs/                    Architecture decisions, guides, benchmarks
```

## Documentation

[ARCHITECTURE.md](ARCHITECTURE.md) · [SECURITY.md](SECURITY.md) · [PERFORMANCE.md](PERFORMANCE.md) · [TESTING.md](TESTING.md) · [ROADMAP.md](ROADMAP.md) · [CHANGELOG.md](CHANGELOG.md) · [CONTRIBUTING.md](CONTRIBUTING.md) · [LICENSES.md](LICENSES.md) · [Plugin development](docs/PLUGIN_DEVELOPMENT.md) · [Discord integration](docs/DISCORD_INTEGRATION.md) · [Compatibility](docs/COMPATIBILITY.md) · [Decision records](docs/decisions/)

## License

Licensed under either of

* MIT license ([LICENSE-MIT](LICENSE-MIT))
* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))

at your option. Dependency licenses are listed in [LICENSES.md](LICENSES.md).

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in DiZero by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
