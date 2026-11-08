# Contributing

## Setup

Windows 10/11 x64 with Rust stable (via rustup), Node.js 22+, Visual Studio Build Tools (C++ workload) and the WebView2 runtime.

```bash
npm install
```

```bash
npm run dev
```

## Before you open a pull request

All of these must pass:

```bash
cargo fmt --all --check
```

```bash
npm run lint:rust
```

```bash
npm test
```

```bash
npm run check
```

If you changed anything a user can see or do, run the end-to-end smoke test ([TESTING.md](TESTING.md)). If you changed anything that could affect memory or startup, run `npm run measure` on a release build and include the numbers.

## How we work

* **Test first** for core logic: write the test against the API you want, run it and watch it fail for the right reason, then implement.
* **Core stays Tauri-free.** Domain logic goes in `crates/dz-core`. The shell only validates, routes and emits events.
* **Contracts are generated.** Rust types crossing IPC derive `ts_rs::TS` with `#[ts(export)]`; `cargo test` regenerates `apps/desktop/src/lib/bindings`. Never hand-edit those files.
* **No fake controls.** A visible control either works or is clearly marked as planned. Features a provider can't do are hidden based on `Capabilities`.
* **No invented numbers.** Performance claims link to raw measurements in `docs/benchmarks/raw/`.
* **Security first for plugins.** New plugin capabilities need a permission, a test proving it's denied without the permission, and an entry in SECURITY.md.
* **Record decisions.** Anything architectural gets an ADR in `docs/decisions/`.
* **Dependencies:** explain why in the PR, check the license is compatible ([LICENSES.md](LICENSES.md)), and prefer small crates and packages. GPL, AGPL and source-available code can't be added.
* **No copied code from Vencord or endcord.** Their licenses are incompatible with DiZero's. Re-implement behaviour from a description, never from their source.

## License of contributions

DiZero is licensed under MIT OR Apache-2.0. Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in DiZero by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

## Style

* Rust: rustfmt (`max_width = 120`), clippy clean with `-D warnings`, `thiserror` for errors, no `unwrap` outside tests unless a comment explains why it can't fail.
* TypeScript/Svelte: strict mode, Svelte 5 runes, no `{@html}`. svelte-check must report 0 warnings.
* UI copy: sentence case, plain verbs, buttons say what they do, errors say what happened and how to recover.
