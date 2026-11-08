//! Local demo provider: a fully offline workspace with seeded communities,
//! people and conversations, plus "Nova", a scripted guide bot that answers
//! mentions. Lets every interaction flow be exercised without any account.
//!
//! Nothing here pretends to be Discord: the demo is labelled as local data
//! throughout the UI.

use crate::db::Database;
use crate::error::{CoreError, Result};
use crate::integrations::{PlannedReply, Provider, SendOutcome};
use crate::models::{Capabilities, IntegrationInfo, IntegrationStatus, Message};
use crate::store::{self, NewMessage};
use crate::util::{new_id, now_ms};
use rusqlite::params;

pub const DEMO_INTEGRATION_ID: &str = "local-demo";
pub const SELF_ID: &str = "demo_self";
pub const NOVA_ID: &str = "demo_nova";

pub struct DemoProvider;

fn ensure_demo_conversation(db: &Database, conversation_id: &str) -> Result<()> {
    let integration = store::conversation_integration(db, conversation_id)?;
    if integration != DEMO_INTEGRATION_ID {
        return Err(CoreError::invalid("conversation does not belong to the demo integration"));
    }
    Ok(())
}

impl Provider for DemoProvider {
    fn integration_id(&self) -> &'static str {
        DEMO_INTEGRATION_ID
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            send_messages: true,
            delete_own_messages: true,
            edit_messages: false,
            attachments_upload: false,
            reactions: false,
            presence: true,
            typing_indicators: true,
            search: true,
            voice: false,
        }
    }

    fn info(&self) -> IntegrationInfo {
        IntegrationInfo {
            id: DEMO_INTEGRATION_ID.into(),
            provider: "local".into(),
            label: "Local demo".into(),
            status: IntegrationStatus::Connected,
            capabilities: self.capabilities(),
            description: "An offline workspace stored on this computer. Messages you send are saved locally and never leave your machine.".into(),
        }
    }

    fn send_message(&self, db: &Database, conversation_id: &str, content: &str) -> Result<SendOutcome> {
        let content = store::normalize_content(content)?;
        ensure_demo_conversation(db, conversation_id)?;
        let message = store::insert_message(
            db,
            NewMessage { conversation_id, author_id: SELF_ID, content: &content, created_at: now_ms() },
        )?;
        let reply = nova_reply(&content).map(|text| PlannedReply {
            delay_ms: 650 + (content.chars().count() as u64 * 6).min(900),
            author_id: NOVA_ID.into(),
            content: text,
        });
        Ok(SendOutcome { message, reply })
    }

    fn delete_message(&self, db: &Database, message_id: &str) -> Result<()> {
        let msg = store::get_message(db, message_id)?;
        ensure_demo_conversation(db, &msg.conversation_id)?;
        if !msg.author.is_self {
            return Err(CoreError::PermissionDenied("you can only delete your own messages".into()));
        }
        store::delete_message(db, message_id)
    }
}

/// Nova's scripted answers. Deterministic so behaviour is testable.
pub fn nova_reply(content: &str) -> Option<String> {
    let lower = content.to_lowercase();
    if !lower.contains("@nova") {
        return None;
    }
    let has = |words: &[&str]| words.iter().any(|w| lower.contains(w));
    let text = if has(&["discord"]) {
        "Discord support is planned for Phase 3, built only on Discord's documented **bot** and **OAuth2** APIs. DiZero will never ask for your Discord password or user token, and it won't automate personal accounts. Until then, everything here is local demo data."
    } else if has(&["plugin", "extension", "dzsdk"]) {
        "Plugins run in their own sandboxed QuickJS runtime with a memory cap and a time budget per call, and they only get the permissions you approve. Open **Plugins** in the left rail to install the bundled examples. Try `/shrug` or `/roll 2d6` once *Slash Tools* is enabled."
    } else if has(&["perf", "memory", "ram", "cpu", "fast", "speed"]) {
        "Open the **Performance Center** (gauge icon in the rail). It measures the whole process tree, DiZero plus its WebView2 helpers, so the number is honest. Switch between *Eco*, *Balanced* and *Performance* modes to trade cache size and animation for resources."
    } else if has(&["theme", "colour", "color", "accent", "dark", "contrast"]) {
        "Head to **Settings → Appearance** for accent colour, density, font scale, reduced motion and high contrast. Theme plugins like *Ember* can recolour the whole app with validated colour tokens."
    } else if has(&["shortcut", "keyboard", "hotkey", "keys"]) {
        "Shortcuts: `Ctrl+K` opens the quick switcher, `Alt+↑/↓` moves between channels, `Ctrl+F` searches, `Ctrl+,` opens settings and `Esc` closes overlays."
    } else if has(&["hello", "hi ", "hey", "hiya", "good morning", "good evening"]) || lower.trim() == "@nova" {
        "Hey! I'm Nova, the demo guide. Ask me about plugins, performance, themes, shortcuts or Discord support."
    } else {
        "I'm a scripted demo bot, so my range is small. Ask me about **plugins**, **performance**, **themes**, **shortcuts** or **Discord** support."
    };
    Some(text.to_string())
}

pub fn deliver_reply(db: &Database, conversation_id: &str, reply: &PlannedReply) -> Result<Message> {
    ensure_demo_conversation(db, conversation_id)?;
    store::insert_message(
        db,
        NewMessage { conversation_id, author_id: &reply.author_id, content: &reply.content, created_at: now_ms() },
    )
}

pub fn set_self_name(db: &Database, name: &str) -> Result<()> {
    let name = crate::util::truncate_chars(name.trim(), 32);
    if name.is_empty() {
        return Err(CoreError::invalid("display name is empty"));
    }
    let username: String = name.to_lowercase().chars().map(|c| if c.is_alphanumeric() { c } else { '_' }).collect();
    db.with(|c| {
        c.execute("UPDATE users SET display_name = ?1, username = ?2 WHERE id = ?3", params![name, username, SELF_ID])?;
        Ok(())
    })
}

pub fn reset(db: &Database, self_name: &str) -> Result<()> {
    db.with(|c| {
        c.execute("DELETE FROM integrations WHERE id = ?1", [DEMO_INTEGRATION_ID])?;
        Ok(())
    })?;
    seed(db, self_name)
}

// ---------------------------------------------------------------------------
// Seed data
// ---------------------------------------------------------------------------

struct SeedUser {
    id: &'static str,
    name: &'static str,
    username: &'static str,
    color: &'static str,
    presence: &'static str,
    bot: bool,
}

const USERS: &[SeedUser] = &[
    SeedUser { id: NOVA_ID, name: "Nova", username: "nova", color: "#9278f5", presence: "online", bot: true },
    SeedUser {
        id: "demo_mara",
        name: "Mara Quinn",
        username: "maraq",
        color: "#e57883",
        presence: "online",
        bot: false,
    },
    SeedUser {
        id: "demo_theo",
        name: "Theo Vance",
        username: "theo.v",
        color: "#65c99a",
        presence: "idle",
        bot: false,
    },
    SeedUser {
        id: "demo_ivy",
        name: "Ivy Chen",
        username: "ivychen",
        color: "#6fb3e8",
        presence: "online",
        bot: false,
    },
    SeedUser {
        id: "demo_kai",
        name: "Kai Moreno",
        username: "kaimoreno",
        color: "#e8b96c",
        presence: "dnd",
        bot: false,
    },
    SeedUser {
        id: "demo_remy",
        name: "Remy Okafor",
        username: "remyo",
        color: "#c28cf0",
        presence: "offline",
        bot: false,
    },
    SeedUser {
        id: "demo_juno",
        name: "Juno Park",
        username: "junopark",
        color: "#f09a6b",
        presence: "online",
        bot: false,
    },
];

/// Attachment metadata: (file name, size in bytes, MIME type).
type SeedAttachment = (&'static str, i64, &'static str);
/// (author id, minutes ago, content, optional attachment)
type SeedMessage = (&'static str, i64, &'static str, Option<SeedAttachment>);

struct SeedChannel {
    name: &'static str,
    kind: &'static str,
    category: &'static str,
    topic: &'static str,
    /// Messages newer than this many minutes ago start unread (0 = all read).
    unread_since_min: i64,
    messages: &'static [SeedMessage],
}

struct SeedCommunity {
    id: &'static str,
    name: &'static str,
    initials: &'static str,
    accent: &'static str,
    description: &'static str,
    members: &'static [(&'static str, Option<&'static str>)],
    channels: &'static [SeedChannel],
}

const COMMUNITIES: &[SeedCommunity] = &[
    SeedCommunity {
        id: "demo_lounge",
        name: "DiZero Lounge",
        initials: "DZ",
        accent: "#9278f5",
        description: "The demo home base. Mention @Nova in any channel to get a reply.",
        members: &[
            (NOVA_ID, Some("Guide")),
            ("demo_mara", Some("Moderator")),
            ("demo_theo", None),
            ("demo_ivy", None),
            ("demo_kai", None),
            ("demo_remy", None),
            ("demo_juno", None),
            (SELF_ID, None),
        ],
        channels: &[
            SeedChannel {
                name: "welcome",
                kind: "announcement",
                category: "Start here",
                topic: "What DiZero is, and what this demo can do",
                unread_since_min: 0,
                messages: &[
                    (
                        NOVA_ID,
                        2880,
                        "**Welcome to DiZero.** This workspace is a *local demo*: every community, person and message here lives in a SQLite file on your computer. Nothing is sent anywhere.",
                        None,
                    ),
                    (
                        NOVA_ID,
                        2879,
                        "Things to try:\n• Send a message in #general and mention `@Nova`\n• Press `Ctrl+K` to jump between channels\n• Open **Plugins** to install the bundled examples\n• Open the **Performance Center** to see what DiZero actually costs your machine",
                        None,
                    ),
                    (
                        NOVA_ID,
                        2878,
                        "Real service integrations come later. The Discord adapter is planned around the official bot and OAuth2 APIs, and DiZero will never ask for a user token.",
                        None,
                    ),
                ],
            },
            SeedChannel {
                name: "changelog",
                kind: "announcement",
                category: "Start here",
                topic: "Release notes for DiZero builds",
                unread_since_min: 0,
                messages: &[(
                    NOVA_ID,
                    1500,
                    "**0.1.0 · Foundation**\n• Tauri 2 shell with a Rust core\n• SQLite storage with migrations and recovery\n• DZSDK v1: sandboxed QuickJS plugins with granular permissions\n• Local demo provider, search, and the Performance Center",
                    None,
                )],
            },
            SeedChannel {
                name: "general",
                kind: "text",
                category: "Community",
                topic: "Anything goes. Be kind.",
                unread_since_min: 45,
                messages: &[
                    (
                        "demo_theo",
                        190,
                        "morning all. finally moved my chat client onto the old laptop and it doesn't sound like a jet engine anymore",
                        None,
                    ),
                    ("demo_ivy", 184, "ha, same. idle memory was the thing that got me", None),
                    ("demo_kai", 170, "anyone tried writing a plugin yet? the sandbox docs look tidy", None),
                    (
                        "demo_mara",
                        160,
                        "Reminder: #plugin-lab is the place for DZSDK questions, #showcase for finished things 🙂",
                        None,
                    ),
                    ("demo_juno", 52, "just finished a notes plugin prototype, it's ~40 lines", None),
                    (
                        "demo_mara",
                        41,
                        "@{self} welcome aboard! Say hi whenever you're ready. @Nova can show you around",
                        None,
                    ),
                    ("demo_ivy", 30, "the quick switcher on Ctrl+K is so fast it feels like it's lying", None),
                    ("demo_theo", 12, "does anyone know if the Eco mode suspends plugins? asking for my battery", None),
                    (
                        "demo_kai",
                        9,
                        "it unloads idle plugin runtimes after a minute, they reload on next use. their saved data stays",
                        None,
                    ),
                ],
            },
            SeedChannel {
                name: "showcase",
                kind: "text",
                category: "Community",
                topic: "Show off themes, plugins and setups",
                unread_since_min: 600,
                messages: &[
                    (
                        "demo_remy",
                        400,
                        "Made a warm theme called Ember, it ships as an example now. Settings → Appearance once it's installed",
                        None,
                    ),
                    ("demo_juno", 380, "the accent override + compact density combo is *chef's kiss*", None),
                    ("demo_ivy", 120, "here's my layout: member panel off, sidebar at 220px. very calm", None),
                ],
            },
            SeedChannel {
                name: "plugin-lab",
                kind: "text",
                category: "Community",
                topic: "Building with DZSDK. Manifests, permissions, debugging",
                unread_since_min: 0,
                messages: &[
                    (
                        "demo_kai",
                        900,
                        "Quick tip: declare every command and panel in `contributes` in dizero-plugin.json. The host won't call anything you didn't declare",
                        None,
                    ),
                    (
                        "demo_theo",
                        880,
                        "and `dz.storage` throws a permission error unless the user granted `storage.local`. nice and explicit",
                        None,
                    ),
                    (
                        "demo_ivy",
                        860,
                        "Developer Center has a validator, point it at a folder and it lists every manifest issue at once",
                        None,
                    ),
                    (
                        "demo_kai",
                        300,
                        "```js\ndz.plugin({\n  commands: {\n    hello: { run: (args) => ({ replaceWith: `hello ${args}` }) }\n  }\n});\n```",
                        None,
                    ),
                ],
            },
        ],
    },
    SeedCommunity {
        id: "demo_synth",
        name: "Synth Lab",
        initials: "SL",
        accent: "#e8b96c",
        description: "Sound design, mixing and gear.",
        members: &[
            ("demo_remy", Some("Host")),
            ("demo_juno", None),
            ("demo_kai", None),
            ("demo_ivy", None),
            (SELF_ID, None),
            (NOVA_ID, Some("Guide")),
        ],
        channels: &[
            SeedChannel {
                name: "patch-share",
                kind: "text",
                category: "Studio",
                topic: "Presets, patches and sample packs",
                unread_since_min: 240,
                messages: &[
                    (
                        "demo_remy",
                        1200,
                        "new pad bank. warm, slow attack, lots of movement in the filter",
                        Some(("patch-bank-v3.fxp", 482_304, "application/octet-stream")),
                    ),
                    ("demo_juno", 1100, "the third patch is gorgeous under a vocal", None),
                    ("demo_kai", 200, "dropping a bass rack later tonight", None),
                ],
            },
            SeedChannel {
                name: "mixing-desk",
                kind: "text",
                category: "Studio",
                topic: "Mix feedback. Be specific, be kind",
                unread_since_min: 0,
                messages: &[
                    ("demo_ivy", 3000, "rule of thumb I keep coming back to: cut before you boost", None),
                    ("demo_remy", 2990, "and reference at the same loudness or your ears will lie to you", None),
                ],
            },
            SeedChannel {
                name: "gear-talk",
                kind: "text",
                category: "Studio",
                topic: "Synths, controllers, interfaces",
                unread_since_min: 0,
                messages: &[
                    ("demo_juno", 5000, "anyone running a 4-voice poly into a cheap interface? noise floor ok?", None),
                    (
                        "demo_kai",
                        4980,
                        "fine if you gain stage properly. the interface was rarely my problem, the cables were",
                        None,
                    ),
                ],
            },
        ],
    },
    SeedCommunity {
        id: "demo_night",
        name: "Night Shift",
        initials: "NS",
        accent: "#65c99a",
        description: "Engineers who ship after dark.",
        members: &[
            ("demo_theo", Some("Maintainer")),
            ("demo_ivy", None),
            ("demo_kai", None),
            ("demo_mara", None),
            (SELF_ID, None),
            (NOVA_ID, Some("Guide")),
        ],
        channels: &[
            SeedChannel {
                name: "rust",
                kind: "text",
                category: "Engineering",
                topic: "Ownership, lifetimes, and the occasional borrow-checker therapy",
                unread_since_min: 0,
                messages: &[
                    (
                        "demo_theo",
                        700,
                        "hot take: a single SQLite connection behind a mutex beats a pool for desktop apps",
                        None,
                    ),
                    ("demo_ivy", 690, "for a chat cache? absolutely. writes are tiny and WAL keeps reads cheap", None),
                ],
            },
            SeedChannel {
                name: "frontend",
                kind: "text",
                category: "Engineering",
                topic: "Svelte, CSS and the DOM",
                unread_since_min: 0,
                messages: &[(
                    "demo_ivy",
                    650,
                    "content-visibility: auto on long lists is free performance, change my mind",
                    None,
                )],
            },
            SeedChannel {
                name: "perf-tuning",
                kind: "text",
                category: "Engineering",
                topic: "Measure first. Then measure again.",
                unread_since_min: 90,
                messages: &[
                    (
                        "demo_kai",
                        300,
                        "Uploaded frame timings from the long-history scroll test",
                        Some(("frame-timing.csv", 18_944, "text/csv")),
                    ),
                    (
                        "demo_theo",
                        70,
                        "remember: a WebView app's memory is the whole process tree, not just the main exe. measure all of it or the number is fiction",
                        None,
                    ),
                    (
                        "demo_mara",
                        60,
                        "@{self} could you sanity-check the Performance Center numbers against Task Manager later?",
                        None,
                    ),
                ],
            },
        ],
    },
];

/// Seed the demo workspace if it doesn't exist yet. Idempotent.
pub fn seed(db: &Database, self_name: &str) -> Result<()> {
    let exists: bool = db.with(|c| {
        Ok(c.query_row("SELECT COUNT(*) FROM integrations WHERE id = ?1", [DEMO_INTEGRATION_ID], |r| {
            r.get::<_, i64>(0)
        })? > 0)
    })?;
    if exists {
        return Ok(());
    }
    let self_name = {
        let n = crate::util::truncate_chars(self_name.trim(), 32);
        if n.is_empty() { "You".to_string() } else { n }
    };
    let now = now_ms();
    db.with_tx(|tx| {
        tx.execute(
            "INSERT INTO integrations(id, provider, label, enabled, config, created_at) VALUES (?1, 'local', 'Local demo', 1, '{}', ?2)",
            params![DEMO_INTEGRATION_ID, now],
        )?;
        let mut user_stmt = tx.prepare(
            "INSERT INTO users(id, integration_id, display_name, username, avatar_color, presence, is_bot, is_self)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )?;
        user_stmt.execute(params![SELF_ID, DEMO_INTEGRATION_ID, self_name, "you", "#a48dff", "online", false, true])?;
        for u in USERS {
            user_stmt.execute(params![u.id, DEMO_INTEGRATION_ID, u.name, u.username, u.color, u.presence, u.bot, false])?;
        }
        let mut msg_stmt = tx.prepare(
            "INSERT INTO messages(id, conversation_id, author_id, content, created_at, attachments, mentions_self)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )?;
        for (ci, c) in COMMUNITIES.iter().enumerate() {
            tx.execute(
                "INSERT INTO communities(id, integration_id, name, initials, accent, description, position) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![c.id, DEMO_INTEGRATION_ID, c.name, c.initials, c.accent, c.description, ci as i64],
            )?;
            for (uid, role) in c.members {
                tx.execute(
                    "INSERT INTO community_members(community_id, user_id, role) VALUES (?1, ?2, ?3)",
                    params![c.id, uid, role],
                )?;
            }
            for (pi, ch) in c.channels.iter().enumerate() {
                let conv_id = format!("{}_{}", c.id, ch.name.replace('-', "_"));
                let last_read = if ch.unread_since_min > 0 { now - ch.unread_since_min * 60_000 } else { now };
                tx.execute(
                    "INSERT INTO conversations(id, community_id, name, kind, category, topic, position, last_read_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![conv_id, c.id, ch.name, ch.kind, ch.category, ch.topic, pi as i64, last_read],
                )?;
                for (author, minutes, text, attachment) in ch.messages {
                    let content = text.replace("{self}", &self_name);
                    let mentions_self = content.to_lowercase().contains(&format!("@{}", self_name.to_lowercase()));
                    let attachments = match attachment {
                        Some((name, size, mime)) => serde_json::json!([{
                            "id": new_id("att"), "name": name, "sizeBytes": size, "mime": mime, "url": null
                        }])
                        .to_string(),
                        None => "[]".into(),
                    };
                    msg_stmt.execute(params![
                        new_id("msg"),
                        conv_id,
                        author,
                        content,
                        now - minutes * 60_000,
                        attachments,
                        mentions_self
                    ])?;
                }
            }
        }
        Ok(())
    })
}
