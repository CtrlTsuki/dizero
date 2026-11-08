# 0007. Plugins describe UI; DiZero renders it

Status: accepted (2026-11-07)

Plugins can't touch the DOM, inject HTML or CSS, or run code in the webview. Panels and sidebar sections return a list of view nodes (`heading`, `text`, `stat`, `progress`, `list`, `button`, `input`, `divider`, `empty`). The host validates them (`plugins/view.rs`): unknown types are dropped, text is length-capped, identifiers are restricted, at most 100 nodes and 100 list items. Svelte renders them with DiZero's own components as text.

Themes are data: a validated map of colour tokens (`#rrggbb` only). No custom CSS.

**Why:** it makes the webview a non-target for plugins (no XSS, no reading other panels, no access to IPC), keeps the UI consistent, and keeps plugin cost predictable.

**Cost:** less expressive than Vencord-style UI patching. Richer node types (tables, toggles, selects, images from plugin-bundled assets) can be added within the same model.
