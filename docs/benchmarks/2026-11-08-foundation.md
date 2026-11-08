# Benchmark: Phase 1 foundation (2026-11-08)

Raw JSON for every run is in [raw/](raw/). Reproduce with `npm run measure` (wraps `scripts/measure-memory.ps1`).

## Environment

| | |
|---|---|
| CPU | AMD Ryzen 5 5600G with Radeon Graphics (6 cores, 12 threads, integrated GPU) |
| RAM | 15.8 GiB |
| OS | Windows 11 Pro, build 26300 |
| WebView2 Runtime | 157.0.4341.42 |
| DiZero | 0.1.0 release build (`tauri build --no-bundle`), exe 13.18 MiB |
| Workload | Local demo workspace, #general open, member panel visible, 4 plugins installed (dormant), nothing else running in DiZero |

## Method

1. Launch `dizero.exe`; time until the visible "DiZero" window exists (the window is only shown after the UI's first paint).
2. Read the app's own "UI ready" time from its log (`run()` to first paint).
3. Wait 20 s for the app to settle.
4. Take 10 samples 1 s apart of every process in the tree rooted at DiZero's PID: private working set (`Win32_PerfRawData_PerfProc_Process.WorkingSetPrivate`), working set and private bytes (`Get-Process`). Use medians.
5. Close with WM_CLOSE (graceful shutdown, verified in the log), wait 2 s, repeat.

Private working set is the headline because it is what Task Manager's Memory column shows and it doesn't double-count DLLs shared between the six Chromium processes. Summed working set is reported for completeness and is not comparable to Task Manager.

## Results: shipped default (network service in-process, hardware acceleration on)

5 runs.

| Run | Launch → visible | UI ready | Private WS | Working set (summed) | Committed |
|---|---|---|---|---|---|
| 1 | 1,741 ms | 902 ms | 103.6 MiB | 356.0 MiB | 178.4 MiB |
| 2 | 671 ms | 638 ms | 112.2 MiB | 368.7 MiB | 188.0 MiB |
| 3 | 685 ms | 652 ms | 103.8 MiB | 357.1 MiB | 178.9 MiB |
| 4 | 674 ms | 647 ms | 106.2 MiB | 358.1 MiB | 183.2 MiB |
| 5 | 704 ms | 671 ms | 104.8 MiB | 357.1 MiB | 181.2 MiB |
| **Median** | **685 ms** | **652 ms** | **104.8 MiB** | **357.1 MiB** | **181.2 MiB** |

Run 1 was the first launch after rebuilding the binary, so its startup is closer to a cold start (the new exe was not yet in the file cache). Source: `raw/20261108-003453-release-final-defaults.json`.

Per process (run 5):

| Process | Private WS | Working set | Committed |
|---|---|---|---|
| WebView2 renderer | 36.2 MiB | 91.7 MiB | 47.2 MiB |
| WebView2 browser (incl. network service) | 32.2 MiB | 130.0 MiB | 40.6 MiB |
| WebView2 GPU process | 26.1 MiB | 71.6 MiB | 75.3 MiB |
| **DiZero core (Rust)** | **5.2 MiB** | 31.6 MiB | 7.1 MiB |
| WebView2 storage service | 3.1 MiB | 19.8 MiB | 7.6 MiB |
| WebView2 crashpad handler | 2.1 MiB | 12.3 MiB | 3.3 MiB |

## Results: WebView2 argument experiments

See [ADR 0005](../decisions/0005-webview2-arguments.md) for the full table. Summary: hardware acceleration off (plus network in-process) measured **89.6 MiB** private working set (3 runs: 89.4, 89.6, 91.8) and 123.8 MiB committed.

## Results: scroll CPU, GPU vs software rendering

`tests/e2e/scroll-bench.mjs`: 10 s of continuous wheel scrolling in a 300-message conversation, driven over CDP, CPU time summed over the process tree.

| Rendering | Run 1 | Run 2 |
|---|---|---|
| GPU | 12.34% of machine (148% of one core) | 12.88% (155%) |
| Software (`--disable-gpu`) | 14.70% (176%) | 14.80% (178%) |

The benchmark is deliberately extreme (wheel events as fast as CDP delivers them) and includes CDP overhead equally in both configurations; it is a relative comparison only.

## Not yet measured

These scenarios from the benchmark plan have **no results yet**; nothing here should be read as a claim about them:

* Comparison against official Discord, Vesktop and endcord (equivalent workloads need real accounts and are a Phase 6 task).
* Cold start after reboot (the runs above are warm: the exe and WebView2 were in the file cache).
* Large-history and attachment-heavy conversations under controlled conditions. One uncontrolled observation: after the full E2E suite (300+ rendered messages, 4 active plugins, DevTools protocol attached), the Performance Center showed 243 MiB private working set, mostly the renderer.
* Five- and ten-plugin workloads, long-duration soak and leak tests, database query timings.
* Windows 10 and machines with discrete GPUs.
