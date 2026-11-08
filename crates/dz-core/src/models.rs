//! Unified, provider-independent communication model.
//!
//! Nothing in here is Discord-specific. Providers translate their own wire
//! formats into these types, and advertise what they can do through
//! [`Capabilities`] so the UI can hide or disable unsupported features
//! instead of rendering controls that do nothing.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Presence {
    Online,
    Idle,
    Dnd,
    Offline,
}

impl Presence {
    pub fn as_str(&self) -> &'static str {
        match self {
            Presence::Online => "online",
            Presence::Idle => "idle",
            Presence::Dnd => "dnd",
            Presence::Offline => "offline",
        }
    }
    pub fn parse(s: &str) -> Presence {
        match s {
            "online" => Presence::Online,
            "idle" => Presence::Idle,
            "dnd" => Presence::Dnd,
            _ => Presence::Offline,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct User {
    pub id: String,
    pub display_name: String,
    pub username: String,
    /// Hex colour used for the generated avatar.
    pub avatar_color: String,
    pub presence: Presence,
    pub is_bot: bool,
    pub is_self: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ConversationKind {
    Text,
    Announcement,
}

impl ConversationKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConversationKind::Text => "text",
            ConversationKind::Announcement => "announcement",
        }
    }
    pub fn parse(s: &str) -> ConversationKind {
        match s {
            "announcement" => ConversationKind::Announcement,
            _ => ConversationKind::Text,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Conversation {
    pub id: String,
    pub community_id: String,
    pub name: String,
    pub kind: ConversationKind,
    pub category: Option<String>,
    pub topic: Option<String>,
    pub position: i32,
    #[ts(type = "number")]
    pub unread_count: i64,
    #[ts(type = "number")]
    pub mention_count: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Community {
    pub id: String,
    pub integration_id: String,
    pub name: String,
    /// Short label rendered in the community rail (1–3 chars).
    pub initials: String,
    pub accent: String,
    pub description: Option<String>,
    pub position: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Attachment {
    pub id: String,
    pub name: String,
    #[ts(type = "number")]
    pub size_bytes: i64,
    pub mime: String,
    /// Where the bytes live. `None` means metadata only (e.g. demo data).
    pub url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Message {
    pub id: String,
    pub conversation_id: String,
    pub author: User,
    pub content: String,
    #[ts(type = "number")]
    pub created_at: i64,
    #[ts(type = "number | null")]
    pub edited_at: Option<i64>,
    pub attachments: Vec<Attachment>,
    pub mentions_self: bool,
}

/// What a provider supports. The UI must consult this before showing a
/// feature, so unsupported features never appear as dead controls.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Capabilities {
    pub send_messages: bool,
    pub delete_own_messages: bool,
    pub edit_messages: bool,
    pub attachments_upload: bool,
    pub reactions: bool,
    pub presence: bool,
    pub typing_indicators: bool,
    pub search: bool,
    pub voice: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum IntegrationStatus {
    Connected,
    Disconnected,
    Planned,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct IntegrationInfo {
    pub id: String,
    pub provider: String,
    pub label: String,
    pub status: IntegrationStatus,
    pub capabilities: Capabilities,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Member {
    pub user: User,
    pub role: Option<String>,
}

/// Everything the UI needs to paint the workspace on startup, in one call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WorkspaceSnapshot {
    pub integrations: Vec<IntegrationInfo>,
    pub communities: Vec<Community>,
    pub conversations: Vec<Conversation>,
    pub self_user: User,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MessagePage {
    pub messages: Vec<Message>,
    /// True when older messages exist before the first one returned.
    pub has_more: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SearchHit {
    pub message: Message,
    pub community_id: String,
    pub conversation_name: String,
    /// Content with matches wrapped in U+E000 / U+E001 marker characters,
    /// which the UI turns into highlights without ever parsing HTML.
    pub snippet: String,
}
