# 0006. SQLite with one connection, versioned migrations, quarantine on corruption

Status: accepted (2026-11-07)

* **rusqlite with bundled SQLite** (FTS5 included and verified by a spike test) instead of `tauri-plugin-sql`: SQL never crosses IPC, so the webview can't run arbitrary queries.
* **One connection behind a mutex.** A chat client's local cache has tiny write volume; a pool would add memory and complexity without measurable benefit. WAL mode keeps reads cheap; `synchronous = NORMAL`; page cache bounded to 2 MiB.
* **Migrations** are an append-only list applied by `PRAGMA user_version`, each in a transaction. A database newer than the build is refused rather than downgraded.
* **Recovery.** On open, `PRAGMA quick_check` runs; an unreadable file is renamed to `dizero.db.corrupt-<ms>` (with its WAL/SHM) and a fresh database is created, so a damaged cache can't prevent startup. The UI tells the user. Covered by `corrupt_database_file_is_quarantined_and_replaced`.
* **Settings** are one validated JSON document (`app_settings.settings`); every field has a default, so older or newer files load field by field.
* **Bounded cache.** Messages beyond 5,000 per conversation are pruned at startup; users can trim further, compact (VACUUM) and back up (`VACUUM INTO`) from Settings.
