# 0004. Keep `panic = "unwind"` in release builds

Status: accepted (2026-11-07)

The Tauri template sets `panic = "abort"` in release to shave binary size. DiZero's plugin host wraps every call into a plugin runtime in `catch_unwind`, so that a panic in a host binding faults the plugin instead of the app. With `abort`, any such panic would terminate DiZero, contradicting the requirement that plugin failures can't crash the core.

Cost: a slightly larger binary (the release exe is 13.2 MiB including QuickJS, SQLite and the embedded UI). No measurable memory difference.
