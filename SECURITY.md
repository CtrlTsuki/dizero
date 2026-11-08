# Security

## Reporting a vulnerability

Please report privately to the maintainers rather than opening a public issue. (A security contact address will be published with the first public release.)

## Threat model

| Threat | Assumption |
|---|---|
| Malicious or buggy **plugin** | Assume any third-party plugin may be hostile. |
| Compromised **renderer** (webview) | Assume web content could be subverted by a future bug; the core must not trust it. |
| Tampered **plugin files** on disk after install | Assume another program could modify them. |
| Damaged **local database** | Assume crashes, disk errors or tampering. |
| Network attackers | Out of scope for 0.1.0: the app makes no network connections. Becomes in scope with Phase 3 integrations. |
| Malware running as the same Windows user | Out of scope; it can already read the user's files and memory. |

## Controls in 0.1.0

### Plugins

* **Isolation:** one QuickJS-NG runtime per plugin with its own heap and a 16 MiB cap; no filesystem, network, process, timer, DOM or host-internal access exists in the sandbox (tested: `sandbox_exposes_no_io_globals`, Fault Lab `/escape`).
* **Time budget** of 200 ms per call (750 ms to activate), enforced by the interpreter's interrupt handler. Five consecutive failures fault the plugin until the user restarts it.
* **Permissions** are granular, requested in the manifest, reviewed by the user, revocable at any time. Native host functions are installed only for granted permissions and re-check them. Requesting an unknown or not-yet-supported permission (e.g. `network.request`) fails validation.
* **Declared contributions only:** the host refuses to call commands or panels the manifest doesn't declare.
* **No plugin-controlled markup:** plugin UI is a sanitised, declarative node list rendered as text ([ADR 0007](docs/decisions/0007-declarative-plugin-ui.md)). Themes are validated hex colours only.
* **Private storage** per plugin with a 512 KiB quota and 64 KiB value limit; plugins can't read each other's data.
* **Install hygiene:** plugins are copied into an app-owned folder; symlinks, executables and unknown file types are rejected; size and file-count limits apply; entry paths can't escape the folder.
* **Integrity:** SHA-256 over all installed files, recorded at install, verified before every load. A mismatch blocks the plugin (`tampering_is_detected_by_integrity_check`).
* **Upgrades** that request new permissions require a fresh review; downgrades via install are refused; the previous version is kept for rollback.
* **Containment of host bugs:** calls run under `catch_unwind`, and release builds keep `panic = "unwind"` so this works ([ADR 0004](docs/decisions/0004-panic-unwind-in-release.md)).

### Webview and IPC

* Every IPC command is allow-listed; the webview has no filesystem, shell, dialog or opener permissions.
* File and folder paths are chosen in native dialogs opened by Rust, never supplied by the webview ([ADR 0008](docs/decisions/0008-dialogs-in-rust.md)).
* Strict CSP (no inline or remote scripts, no frames, no objects); Tauri `freezePrototype` on.
* Message text is parsed by a small tokenizer and rendered as text; HTML is never interpreted (`never interprets HTML` test). Links open only after confirmation and only for `http(s)`.
* Inputs are validated in Rust (message length, query length, panel payload size, settings normalisation).

### Data

* The database is opened with an integrity check; a corrupt file is quarantined, not deleted, and the app starts fresh.
* No credentials are stored. Future integration secrets will go to Windows Credential Manager, never SQLite, logs or plugin storage.
* DiZero itself doesn't write message contents or secrets to its log. Plugins can write their own lines through `dz.log` (rate-limited, tagged `plugin:<id>`), so a plugin with `events.messages` could log what it reads.
* No telemetry.

## Known limitations

* **Signatures are not verified.** The manifest's `signature` field is recorded and displayed as "not verified". Signed distribution needs a key infrastructure (Phase 7).
* **In-process plugin host.** QuickJS is C code running inside DiZero's process. A memory-safety bug in QuickJS triggered by a malicious plugin could compromise the app. Mitigations today: user approval, integrity checks, minimal API surface. Planned: move the plugin host into a separate low-privilege process before opening a public plugin repository.
* **Same-user tampering.** A program running as the user can modify the database or plugin folder; integrity checks detect plugin-file changes but the expected hashes live in the same database.
* **No automated dependency audit in CI yet.** `cargo-audit` / `cargo-deny` and `npm audit` are not wired in (no CI exists yet). A manual license inventory is in [LICENSES.md](LICENSES.md).
* **WebView2 updates** are managed by Windows; DiZero relies on Microsoft for renderer security patches.
