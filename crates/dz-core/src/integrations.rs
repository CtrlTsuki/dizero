//! Provider abstraction and the integration manager.
//!
//! The UI never talks to a provider directly: it calls generic commands
//! (send, delete, list) which the manager routes to the provider that owns
//! the conversation. New services (e.g. a Discord bot adapter) plug in by
//! implementing [`Provider`].

use crate::db::Database;
use crate::error::Result;
use crate::models::{Capabilities, IntegrationInfo, Message};

/// A reply a provider wants delivered later (the demo bot "typing").
#[derive(Debug, Clone, PartialEq)]
pub struct PlannedReply {
    pub delay_ms: u64,
    pub author_id: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct SendOutcome {
    pub message: Message,
    pub reply: Option<PlannedReply>,
}

pub trait Provider: Send + Sync {
    fn integration_id(&self) -> &'static str;
    fn capabilities(&self) -> Capabilities;
    fn info(&self) -> IntegrationInfo;
    fn send_message(&self, db: &Database, conversation_id: &str, content: &str) -> Result<SendOutcome>;
    fn delete_message(&self, db: &Database, message_id: &str) -> Result<()>;
}

/// Integrations that are designed but not implemented in this build. They are
/// listed in the UI with an honest status instead of fake controls.
pub fn planned_integrations() -> Vec<IntegrationInfo> {
    vec![IntegrationInfo {
        id: "discord".into(),
        provider: "discord".into(),
        label: "Discord (bot / OAuth2)".into(),
        status: crate::models::IntegrationStatus::Planned,
        capabilities: Capabilities {
            send_messages: false,
            delete_own_messages: false,
            edit_messages: false,
            attachments_upload: false,
            reactions: false,
            presence: false,
            typing_indicators: false,
            search: false,
            voice: false,
        },
        description: "Phase 3. Uses Discord's documented Bot and OAuth2 APIs only. DiZero will never ask for your Discord password or user token, and will not automate personal accounts.".into(),
    }]
}
