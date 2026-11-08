# 0005. WebView2 browser arguments

Status: accepted (2026-11-08)

## Context

95% of DiZero's idle memory is the WebView2 runtime. WebView2 accepts Chromium arguments at environment creation. We measured candidates on the release build with `scripts/measure-memory.ps1` (idle, private working set summed over the process tree; Windows 11 26300, Ryzen 5 5600G, WebView2 157).

## Measurements

| Arguments (plus wry's defaults) | Runs | Processes | Private working set (median) | Committed |
|---|---|---|---|---|
| none (baseline) | 3 | 7 | 109.6 MiB | 189.5 MiB |
| `--in-process-gpu` | 2 | 6 | 100.5 MiB | 171.3 MiB |
| `--enable-features=NetworkServiceInProcess2` | 2 | 6 | 101.3 MiB | 181.8 MiB |
| `--enable-low-end-device-mode` | 2 | 7 | 110.3 MiB | 191.7 MiB |
| `--disable-gpu` | 2 | 7 | 96.9 MiB | 135.5 MiB |
| network in-process + `--in-process-gpu` | 3 | 5–6 | 98.1 MiB, one run 144 MiB / 429 MiB committed | unstable |
| network in-process + `--disable-gpu` | 3 | 6 | 89.6 MiB (89.4–91.8) | 123.8 MiB |
| **network in-process (shipped default)** | 5 | 6 | **104.8 MiB** (103.8–112.2) | 181.2 MiB |

CPU while continuously wheel-scrolling a 300-message conversation (whole tree, 2 runs each): GPU on 12.3–12.9% of the machine, `--disable-gpu` 14.7–14.8% (about 18% more).

## Decision

* Always run the network service in the browser process (`NetworkServiceInProcess2`): the UI makes no network requests (CSP limits it to IPC), so a separate network process buys nothing. E2E tests pass with it on the release build.
* Keep GPU rendering on by default for smoother scrolling and lower CPU under load. Offer **Settings → Performance → Hardware acceleration**; off adds `--disable-gpu` at the next launch and meets the 100 MiB target.
* Reject `--in-process-gpu`: unstable across runs, and a GPU driver crash would take down the browser process.
* Reject low-end device mode: no gain.

Supplying arguments replaces wry's defaults, so `--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection` is restated (`settings::webview_browser_args`, covered by a test).

## Revisit when

WebView2 ships memory-saver features we can enable, or measurements on other hardware (integrated vs discrete GPUs, Windows 10) disagree.
