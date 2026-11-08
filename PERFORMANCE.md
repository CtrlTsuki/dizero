# Performance

Resource efficiency is DiZero's defining requirement. This file says what was measured, how, and what was decided. Every number links back to raw data; nothing here is estimated.

## Current results (0.1.0, release build, idle)

| | Default | Hardware acceleration off |
|---|---|---|
| Private working set, whole process tree | **104.8 MiB** (median of 5) | **89.6 MiB** (median of 3) |
| Committed (private bytes) | 181.2 MiB | 123.8 MiB |
| Launch to first painted window | 685 ms (warm) | 718 ms (warm) |
| DiZero's own Rust process | 5.2 MiB private working set | |

Target: under 100 MiB idle for a minimal build (a stretch goal). Met with hardware acceleration off; 5% over with it on. Details: [benchmarks/2026-11-08-foundation.md](docs/benchmarks/2026-11-08-foundation.md).

## How to measure

```bash
npm run measure
```

This builds nothing; build first with `npx tauri build --no-bundle` in `apps/desktop`. The script launches the release exe several times, waits for idle, samples the entire process tree, closes the app gracefully and writes JSON to `docs/benchmarks/raw/`. Options: `-Runs`, `-SettleSeconds`, `-Samples`, `-BrowserArgs` (experiment with WebView2 flags), `-Label`.

Rules we follow:

* **Measure the whole process tree.** A Tauri app on Windows is DiZero plus 5–6 WebView2 processes. Reporting only `dizero.exe` would understate memory about twentyfold.
* **Headline private working set**, the figure Task Manager shows. Summing working sets double-counts shared DLLs across the Chromium processes and is reported separately.
* **Release builds only.** A dev build with Vite and HMR used 510–544 MiB working set; it is not representative.
* **Medians over several runs,** machine details recorded, raw data kept.

The in-app Performance Center uses the same method live (`dz-core/src/perf.rs`: Toolhelp process snapshot, `GetProcessMemoryInfo` with `PROCESS_MEMORY_COUNTERS_EX2`, `GetProcessTimes`), sampling only while the page is open.

## What we did about it

| Decision | Effect (measured) | Record |
|---|---|---|
| Network service in the browser process | −1 process, about −5 MiB | [ADR 0005](docs/decisions/0005-webview2-arguments.md) |
| Hardware acceleration as a user setting | Off: −15 MiB private WS, −57 MiB committed, +18% CPU while scrolling | [ADR 0005](docs/decisions/0005-webview2-arguments.md) |
| Rejected `--in-process-gpu` | Unstable across runs | [ADR 0005](docs/decisions/0005-webview2-arguments.md) |
| Rejected low-end device mode | No gain | [ADR 0005](docs/decisions/0005-webview2-arguments.md) |
| Plugins lazy and dormant | 0 bytes per installed plugin until first use; ~127 KiB heap for Slash Tools once active | [ADR 0003](docs/decisions/0003-quickjs-plugin-runtime.md) |
| Idle plugin unload | Eco 60 s, Balanced 10 min, Performance never | `settings.rs` |
| Code-split tool pages | Initial JS 199 KB → 140 KB | `App.svelte` |
| Bounded message window | 150 / 300 / 800 messages per open conversation by mode | `messageWindow.ts` |
| `content-visibility: auto` on message rows | Off-screen rows skip layout and paint | `MessageRow.svelte` |
| No background polling | Sampling and plugin-stats refresh only while their pages are open | |
| Bounded SQLite cache and pruning | 2 MiB page cache; 5,000 messages per conversation | [ADR 0006](docs/decisions/0006-sqlite-single-connection.md) |

## Performance modes

| | Eco | Balanced | Performance |
|---|---|---|---|
| Messages kept per open conversation | 150 | 300 | 800 |
| Messages fetched per page | 40 | 50 | 80 |
| Animations | Off | On | On |
| Prefetch channels on hover | Off | Off | On |
| Unload idle plugin runtimes | After 60 s | After 10 min | Never |
| Performance Center sampling | 5 s | 2 s | 1 s |

Sending, receiving and notifications behave identically in all modes.

## Known gaps

* The message list is a bounded window with `content-visibility`, not true virtualization. For conversations that page back thousands of messages, a virtual list would cut DOM size further. Not yet needed at the default window sizes; to be measured in Phase 6.
* Competitor comparisons, cold-start, soak and leak tests, and Windows 10 numbers are not done yet (see the benchmark report's "Not yet measured").
* The renderer (36 MiB) is now the largest process; profiling its JS heap and style recalculation is the next optimization step.
