# Changelog

## 0.1.0 — Foundation (2026-11-08)

First runnable build. Windows 10/11 x64.

### Added

* Tauri 2 desktop app with a Rust core (`dz-core`) and a Svelte 5 interface in a dark console-inspired design system (Inter, Lucide icons, LED status indicators).
* Local demo workspace: three communities, eleven channels, seven people, a scripted bot (Nova), typing indicators, unread and mention tracking.
* Messaging: send, delete own, safe rich text (bold, italic, code, quotes, mentions, confirmed links), pagination with a bounded in-memory window and "Jump to present".
* Full-text search (SQLite FTS5) with highlighted results.
* Quick switcher (`Ctrl+K`), channel navigation (`Alt+↑/↓`), context menus, tooltips, toasts, resizable and collapsible sidebar, side dock.
* Settings: profile, theme, accent, density, text size, avatars, motion, high contrast, notifications, performance mode, hardware acceleration, log level, import/export, reset, data management (compact, trim, back up, reset demo, clear plugin data).
* Performance Center: per-process private working set, working set, committed memory and CPU for the whole process tree, plugin heaps, storage stats, background-activity report, live trend.
* DZSDK v1 plugin system: QuickJS-NG sandbox per plugin, memory cap, time budget, granular permissions, declarative panels and sidebar sections, slash commands, message events, private storage with quotas, notifications, integrity verification, lazy activation, idle unload, auto-faulting, upgrade and rollback.
* Example plugins: Slash Tools, Message Stats, Quick Notes, Ember theme, Fault Lab.
* Developer Center: environment, sandbox limits, folder validator, runtime log viewer, SDK reference.
* Plugin template and TypeScript definitions.
* Database migrations, integrity check with quarantine of corrupt files, WAL checkpoint on shutdown.
* Measurement script, scroll CPU benchmark, end-to-end smoke test over CDP.
* License: MIT OR Apache-2.0 (`LICENSE-MIT`, `LICENSE-APACHE`), declared in every Cargo and npm manifest and copied by the installer.

### Performance

* Idle, release build: 104.8 MiB private working set by default; 89.6 MiB with hardware acceleration off. See [PERFORMANCE.md](PERFORMANCE.md).
