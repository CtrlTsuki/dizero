# 0008. File and folder dialogs run in Rust

Status: accepted (2026-11-07)

Commands that touch the filesystem (install a plugin from a folder, validate a folder, back up the database, export or import settings) open the native dialog from Rust with `tauri-plugin-dialog`'s Rust API and use the path the user picked. The webview never passes a path, and has no dialog, filesystem or opener permission.

**Why:** if the renderer were ever compromised (for example by a future rendering bug), it still couldn't make DiZero read or write arbitrary files; the user has to pick every path in a native dialog.

The same rule applies to opening the log folder (`logs_open_folder` takes no arguments) and links (`open_link` accepts only `http(s)` URLs and the UI confirms first).
