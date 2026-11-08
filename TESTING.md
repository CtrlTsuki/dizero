# Testing

Every result below was produced on the development machine on 2026-11-08. Nothing is reported as passing that wasn't run.

## Suites

| Suite | Command | Count | Last result |
|---|---|---|---|
| Rust behaviour tests (`dz-core`) | `cargo test --workspace` | 99 | pass |
| Rust contract exports (ts-rs regenerates `apps/desktop/src/lib/bindings`) | same | 58 | pass |
| Frontend unit tests (Vitest) | `npm run test:web` | 25 | pass |
| Type check (svelte-check, strict TS, fails on warnings) | `npm run check` | – | 0 errors, 0 warnings |
| Lint | `cargo clippy --workspace --all-targets -- -D warnings` | – | clean |
| Format | `cargo fmt --all --check` | – | clean |
| End-to-end, real app over CDP | `node tests/e2e/smoke.mjs` | 23 checks in 11 flows | pass on debug and final release builds |
| Design detector (impeccable) | `impeccable detect apps/desktop/src` | – | no findings |

## What the Rust tests cover

| File | Area | Notable cases |
|---|---|---|
| `tests/storage.rs` | Database | Fresh migration, reopen keeps data, **corrupt file quarantined and replaced**, backup, vacuum |
| `tests/settings.rs` | Settings | Defaults, persistence across reopen, normalisation of hostile values (`javascript:` accent), partial/unknown JSON, import/export, perf profiles, hardware acceleration, WebView2 args |
| `tests/manifest.rs` | Manifest validation | Bad ids and versions, unknown and planned permissions, **path traversal in entry**, permission/contribution consistency, theme colour injection, all issues reported at once |
| `tests/view.rs` | Plugin UI sanitisation | Unknown/HTML nodes dropped, truncation, node and list caps |
| `tests/plugin_host.rs` | Sandbox | **Infinite loop timeout**, **memory bomb**, syntax errors, **auto-fault after repeated failures**, permission-gated storage and notifications, **storage quota**, **no I/O globals**, undeclared commands refused, lazy activation, idle unload and reload, events only to subscribers |
| `tests/registry.rs` | Plugin lifecycle | Install copies and starts disabled, invalid/missing/oversized/disallowed files rejected, approve only requested permissions, revoke, **tamper detection**, uninstall removes files/rows/storage, dependencies, **upgrade with re-review and rollback**, downgrade refused |
| `tests/demo.rs` | Messaging | Idempotent seed, send/validate, pagination, Nova replies, **FTS search tolerates query syntax injection**, delete own vs others, unread/mark read, pruning, reset |
| `tests/bundled_plugins.rs` | Shipped examples | Every bundled plugin and the SDK template validates, installs and runs; Fault Lab is contained |
| `src/perf.rs` | Metrics | Process-tree walk (with cycles), live Win32 sampling including private working set and CPU delta |
| `src/logging.rs`, `src/util.rs`, `src/store.rs` | Misc | Ring buffer, ids, hex, FTS query escaping |

The core was built test-first: each module's tests were written against stubbed signatures, run to confirm they failed for the right reason, then implemented.

## Frontend unit tests

`apps/desktop/src/lib/*.test.ts`: the message formatter (never produces HTML, rejects `javascript:` links, mentions, code blocks), formatting helpers, theme resolution and WCAG contrast under high contrast, slash-command parsing, and the bounded message window.

## End-to-end tests

`tests/e2e/smoke.mjs` attaches to the running app's WebView2 over the Chrome DevTools Protocol and drives the real UI and backend:

1. Workspace renders with demo data.
2. Send a message; typing indicator; Nova's reply.
3. Quick switcher navigation.
4. Search returns results.
5. Install Slash Tools through the permission review; `/shrug` rewrites and sends.
6. Fault Lab: `/hang` is stopped (≈300 ms), `/escape` finds nothing, UI stays responsive.
7. Message Stats and Quick Notes panels: add, pin (sidebar section updates), live counting.
8. Performance Center measures the process tree.
9. Settings change applies.
10. Developer Center shows logs.
11. Uninstall Fault Lab through the UI: it leaves the installed list, is installable again from the catalog, and its `/hang` command disappears. No page errors throughout.

Run it:

```bash
npm run dev
```

with `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222` set in the environment, then:

```bash
node tests/e2e/smoke.mjs
```

For a release build, include DiZero's own arguments, because the variable replaces them: `--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --enable-features=NetworkServiceInProcess2 --remote-debugging-port=9222`. Screenshots land in `tests/e2e/out/`. Re-running is safe: already-installed plugins are reused, which also exercises persistence across restarts.

Also verified manually during development: graceful shutdown (WM_CLOSE → log line "DiZero shut down cleanly", WAL checkpointed to 0 bytes), settings and plugin state surviving restarts.

## Gaps (not yet automated)

* Component tests for individual Svelte components, visual regression screenshots and an automated accessibility audit (axe).
* Concurrency and soak tests (long sessions, leak detection), performance regression gates in CI.
* `cargo-audit` / `cargo-deny` / `npm audit` and secret scanning (no CI exists yet).
* Plugin timeouts during activation of a real-world large plugin, and malformed UTF-8 in plugin files.
* Keyboard-only and screen-reader walkthroughs were checked by reading the markup (roles, labels, focus order), not with a screen reader.
