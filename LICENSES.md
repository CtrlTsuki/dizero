# Licenses

## DiZero's own license

DiZero is dual-licensed under **MIT OR Apache-2.0**: you may use it under either license, at your option. The texts are in [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE), and the installer copies both next to the app. Decided on 2026-11-08 ([ADR 0009](docs/decisions/0009-license.md)).

What this means in practice:

* **No GPL code can be incorporated.** Nothing from Vencord (GPL-3.0-or-later) may be copied, ported or translated into DiZero. Features inspired by a Vencord plugin must be clean-room re-implementations: describe the behaviour, then write new code. endcord's code is source-available and can't be reused either ([COMPATIBILITY.md](docs/COMPATIBILITY.md)).
* **Contributions come in under the same terms** (see [CONTRIBUTING.md](CONTRIBUTING.md)).
* **Plugins are separate works.** Plugin authors choose their own license; DiZero doesn't impose one.

Nothing in the dependency tree conflicts with this license (see below).

### Before the first public release

Several dependencies (MIT, BSD, Unicode, Zlib and the OFL fonts) require their copyright and license notices to travel with any binary we distribute. The installer doesn't include a third-party notices file yet; one must be generated from the dependency tree (for example with `cargo-about`, plus the npm packages listed below) and bundled before DiZero is published.

## Dependencies

Inventory generated from `cargo metadata` (Windows target) and npm package metadata on 2026-11-08. This is a manual snapshot; automated checking with `cargo-deny` is planned.

### Rust: 281 crates

| License expression | Crates |
|---|---|
| MIT OR Apache-2.0 (incl. "Apache-2.0 OR MIT", "MIT/Apache-2.0", "Apache-2.0 / MIT") | 183 |
| MIT | 54 |
| Unicode-3.0 | 18 |
| Unlicense OR MIT | 7 |
| MPL-2.0 | 5 |
| BSD-3-Clause, BSD-3-Clause/MIT, BSD-3-Clause AND MIT | 4 |
| Zlib; MIT OR Zlib OR Apache-2.0; MIT OR Apache-2.0 OR Zlib | 5 |
| Apache-2.0 (tao); Apache-2.0 AND MIT (dpi) | 2 |
| 0BSD OR MIT OR Apache-2.0; CC0-1.0 OR MIT-0 OR Apache-2.0; (MIT OR Apache-2.0) AND Unicode-3.0 | 3 |

No GPL or AGPL crates. MPL-2.0 is file-level copyleft: fine to depend on unmodified; modified MPL files must be published under MPL-2.0.

Notable components: Tauri (MIT OR Apache-2.0), rusqlite (MIT) with bundled **SQLite** (public domain), rquickjs (MIT) with bundled **QuickJS-NG** (MIT), windows-sys (MIT OR Apache-2.0), ts-rs (MIT).

### JavaScript (shipped in the app)

| Package | License |
|---|---|
| svelte | MIT |
| @tauri-apps/api | Apache-2.0 OR MIT |
| @lucide/svelte (icons) | ISC |
| @fontsource-variable/inter (Inter typeface) | OFL-1.1 |
| @fontsource-variable/geist-mono (Geist Mono typeface) | OFL-1.1 |

Development-only: Vite, Vitest, svelte-check, TypeScript, @tauri-apps/cli, playwright-core (all MIT or Apache-2.0).

The OFL-1.1 fonts may be bundled and redistributed with the app; they can't be sold on their own.

### Runtime not shipped by DiZero

Microsoft Edge WebView2 Runtime, provided by Windows under Microsoft's terms.
