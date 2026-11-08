# Discord integration

Status: **Planned (Phase 3).** DiZero 0.1.0 contains no Discord code and makes no network connections.

## The line DiZero won't cross

Discord's documented API is for **bot accounts** and **OAuth2 applications**. Automating a normal user account ("self-bots"), including by logging a third-party client in with a user's token, is against Discord's rules and can get accounts terminated. Discord's support article on self-bots directs developers to bot accounts, its Community Guidelines tell users not to use self-bots, and its platform-manipulation policy lists self-bots alongside spam bots.

Sources: [Discord support: self-bots](https://support.discord.com/hc/articles/115002192352), [Community Guidelines](https://discord.com/guidelines), [Platform manipulation policy explainer](https://discord.com/safety/platform-manipulation-policy-explainer). Re-read the current [Developer Terms](https://discord.com/developers/docs/policies-and-agreements/developer-terms-of-service) and [Developer Policy](https://discord.com/developers/docs/policies-and-agreements/developer-policy) before implementation; they change.

So DiZero will:

* never ask for a Discord password or user token, and never accept one pasted in;
* never automate a personal account or imitate the official client's traffic;
* never present a bot integration as if it were a full personal-account replacement.

This means DiZero will **not** be a drop-in replacement for the Discord client for personal use. That is a deliberate, permanent product boundary.

## What a supported integration can do

| Capability | Bot account in a server you manage | OAuth2 (user-authorised) |
|---|---|---|
| Read channels the bot can see, via Gateway events | Yes (message content needs the privileged Message Content intent, which bots in 100 or more servers must be approved for) | No |
| Send messages as the bot | Yes | No |
| Application (slash) commands | Yes | – |
| Identify the user, list their servers | – | Yes (`identify`, `guilds` scopes) |
| Read a user's DMs or act as the user | No | No |
| Voice | Bots can join voice channels; mixing with a desktop client's audio is a separate research track | No |

Useful products within these limits include a **community console** for server owners and moderators (watch channels, respond as the server's bot, run moderation commands) and a **dashboard** of the user's own servers through OAuth2.

## Planned design

* A `DiscordBotProvider` implementing `dz_core::integrations::Provider`, with honest `Capabilities` (for example `delete_own_messages` only for the bot's own messages).
* Gateway client in Rust (async, in the shell's runtime), with heartbeats, resume and reconnect with backoff, and per-route rate-limit buckets honouring `X-RateLimit-*` headers and global limits.
* The bot token is entered once, stored in **Windows Credential Manager**, never in SQLite, logs, settings exports or plugin storage, and is never sent to the webview.
* OAuth2 uses the authorisation-code flow with PKCE via the system browser and a loopback redirect.
* Messages map into the unified model; Discord-specific data goes in provider metadata, never into shared types.
* Every Discord feature the bot API can't support stays hidden, not disabled-looking.
