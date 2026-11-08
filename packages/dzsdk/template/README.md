# Hello — DZSDK plugin template

1. Copy this folder and rename it.
2. In `dizero-plugin.json`, set a unique reverse-domain `id` (for example `io.github.you.my-plugin`), a name and a description.
3. Declare every command, panel and event you implement under `contributes`, and request only the permissions they need.
4. In DiZero, open **Developer Center → Validate a plugin** and choose the folder. Fix every reported issue.
5. Open **Plugins → Install from folder**, review the permissions, and enable it.

DiZero copies the folder when installing. After editing your source, install it again (bump `version` for upgrades; downgrades are refused, use **Roll back** instead).

Full guide: [docs/PLUGIN_DEVELOPMENT.md](../../../docs/PLUGIN_DEVELOPMENT.md).
