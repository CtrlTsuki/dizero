# DZ Compatibility Bridge: Vencord and endcord

Status: **Research complete for Phase 0; no bridge code exists.** Conclusion: neither ecosystem's plugins can run in DiZero as-is, and attempting it would conflict with DiZero's security and policy rules. Individual plugin *ideas* can be re-implemented on DZSDK where they only need data DiZero legitimately has.

Facts below were checked on 2026-11-08 against the projects' own repositories and docs. Re-check before acting on them.

## Vencord

* **What it is:** a client mod for the official Discord web/desktop client, and the base of the Vesktop desktop app.
* **How plugins work:** TypeScript modules compiled into Vencord. They find Discord's internal webpack modules and **patch Discord's own JavaScript with regular-expression replacements**, then use Discord's React components and internal stores.
* **License:** GPL-3.0-or-later (source headers carry `SPDX-License-Identifier: GPL-3.0-or-later`; the README notes user plugins in `src/userplugins` may be licensed differently).

Why plugins can't run in DiZero:

1. DiZero doesn't run Discord's client code, so there is nothing to patch: no webpack modules, stores or React tree.
2. Vencord plugins act inside the logged-in user's session, which DiZero will never have (see [DISCORD_INTEGRATION.md](DISCORD_INTEGRATION.md)).
3. They execute with full page privileges; DZSDK plugins are sandboxed with no DOM or network access.

Licensing: copying Vencord code into DiZero would require DiZero to be distributed under the GPL. DiZero is MIT OR Apache-2.0 ([ADR 0009](decisions/0009-license.md)), so any feature inspired by a Vencord plugin must be a clean-room re-implementation: describe the behaviour, don't port the code.

Themes: Vencord themes are arbitrary CSS targeting Discord's class names. DiZero themes are validated colour tokens, so CSS themes can't be imported; a converter that extracts colour variables into a DiZero token theme is feasible.

## endcord

* **What it is:** a terminal (TUI) Discord client written in Python with ncurses ([sparklost/endcord](https://github.com/sparklost/endcord)).
* **Authentication:** logs into a personal account by email, MFA, QR code or a pasted token. This is the personal-account automation DiZero won't do.
* **Extensions:** Python files given the whole app object; its docs say they "can even access all the tokens". There is no sandbox or permission model; safety relies on users reading the code.
* **License:** the README now states endcord is **source-available, not open source**, and forbids public modified forks. (Some package listings still show GPL-3.0; the repository's own statement takes precedence.)

Why extensions can't run in DiZero: they're Python (DiZero has no Python runtime and won't embed one for plugins), they assume unrestricted access to the app and the user's tokens, and they target a personal-account client. Its license also rules out reusing its code.

## Compatibility matrix

Classification of common plugin *types* (not individual plugins, which would need per-plugin review):

| Plugin type (examples) | Class | Path in DiZero |
|---|---|---|
| Composer text tools (shrug, dice, text transforms) | 2: Adaptable | Re-implement as `commands.slash` (Slash Tools already does) |
| Local notes, reminders, personal organisation | 2: Adaptable | `ui.panels` + `storage.local` (Quick Notes) |
| Message statistics, word counts | 2: Adaptable | `events.messages` + panel (Message Stats) |
| Themes and colour schemes | 2: Adaptable | Convert colour variables to DiZero theme tokens |
| Accessibility tweaks (contrast, font size, reduced motion) | 1: Native | Built into DiZero settings |
| UI rearrangement by patching Discord components | 4: Incompatible | No Discord UI to patch; DiZero layout settings cover some needs |
| Features reading Discord internals (hidden channels, client experiments, spoofing) | 5: Blocked | Requires a personal-account client and/or circumventing Discord's client; refused |
| Message logging of deleted/edited messages from other users | 5: Blocked | Requires personal-account access; privacy concerns |
| Anything using the user's token or acting as the user | 5: Blocked | Against Discord's rules and DiZero's security model |
| Integrations with external services (GitHub, music status) | 3: Extensive reimplementation | Needs `network.request` with domain allow-lists (planned API v2) |
| Media players and previews | 3: Extensive reimplementation | Needs `media.controls` and richer view nodes (planned) |

Classes: 1 native-compatible, 2 adaptable through DZSDK, 3 needs extensive reimplementation, 4 incompatible, 5 blocked by legal, security or platform restrictions.

## Next steps (Phase 4)

* Pick the ten most-used plugins in each ecosystem and classify them individually, recording sources.
* Build a colour-variable theme converter (class 2).
* Add the planned DZSDK capabilities that unblock class-3 plugins.
