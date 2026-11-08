# 0009. License: MIT OR Apache-2.0

Status: accepted (2026-11-08)

DiZero is dual-licensed under MIT OR Apache-2.0, the same terms as Tauri and most of the Rust ecosystem it's built on. The texts are in `LICENSE-MIT` and `LICENSE-APACHE` at the repository root, the Cargo and npm manifests declare the SPDX expression, and the installer copies both files next to the app.

**Why:** the dependency tree is almost entirely MIT and Apache-2.0, so a matching permissive license keeps DiZero easy to embed, package and contribute to. Apache-2.0 adds an explicit patent grant; MIT keeps compatibility with GPL-2.0 projects that want to reuse DiZero code.

**Alternative considered:** GPL-3.0-or-later, which would have allowed copying Vencord code. Rejected because DiZero's compatibility plan doesn't need Vencord's code: Vencord plugins patch Discord's own client, which DiZero doesn't run, so useful ideas have to be rewritten for DZSDK anyway ([COMPATIBILITY.md](../COMPATIBILITY.md)).

**Consequences:**

* No GPL, AGPL or source-available code may be copied into DiZero. Features inspired by Vencord or endcord plugins are clean-room re-implementations.
* Contributions are accepted under the same dual license (CONTRIBUTING.md).
* Plugins are separate works and may use any license.
* Before the first public release, a third-party notices file must be generated and bundled with the installer (LICENSES.md).
