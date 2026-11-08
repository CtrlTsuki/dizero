//! Granular plugin permissions.
//!
//! A permission is only ever granted by the user. The host installs an API
//! surface into a plugin's sandbox *only* for granted permissions, and every
//! native host function re-checks the grant (defence in depth).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum Permission {
    #[serde(rename = "ui.panels")]
    UiPanels,
    #[serde(rename = "ui.sidebar")]
    UiSidebar,
    #[serde(rename = "ui.themes")]
    UiThemes,
    #[serde(rename = "storage.local")]
    StorageLocal,
    #[serde(rename = "events.messages")]
    EventsMessages,
    #[serde(rename = "commands.slash")]
    CommandsSlash,
    #[serde(rename = "notifications")]
    Notifications,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PermissionInfo {
    pub permission: Permission,
    pub key: String,
    pub title: String,
    pub description: String,
    pub risk: RiskLevel,
}

/// Permissions that exist in the DZSDK design but are not implemented by
/// this build. Requesting them fails validation with an explicit message
/// rather than being silently ignored.
pub const PLANNED_PERMISSIONS: &[&str] =
    &["network.request", "media.controls", "fs.read", "fs.write", "process.spawn", "clipboard.write"];

impl Permission {
    pub const ALL: [Permission; 7] = [
        Permission::UiPanels,
        Permission::UiSidebar,
        Permission::UiThemes,
        Permission::StorageLocal,
        Permission::EventsMessages,
        Permission::CommandsSlash,
        Permission::Notifications,
    ];

    pub fn key(&self) -> &'static str {
        match self {
            Permission::UiPanels => "ui.panels",
            Permission::UiSidebar => "ui.sidebar",
            Permission::UiThemes => "ui.themes",
            Permission::StorageLocal => "storage.local",
            Permission::EventsMessages => "events.messages",
            Permission::CommandsSlash => "commands.slash",
            Permission::Notifications => "notifications",
        }
    }

    pub fn parse(key: &str) -> Option<Permission> {
        Permission::ALL.into_iter().find(|p| p.key() == key)
    }

    pub fn info(&self) -> PermissionInfo {
        let (title, description, risk) = match self {
            Permission::UiPanels => (
                "Show panels",
                "Add panels to the right-hand dock. Panels can only display text, lists, stats, buttons and inputs rendered by DiZero; plugins cannot inject HTML or scripts.",
                RiskLevel::Low,
            ),
            Permission::UiSidebar => (
                "Show sidebar sections",
                "Add a compact section to the bottom of the channel sidebar, using the same safe rendering as panels.",
                RiskLevel::Low,
            ),
            Permission::UiThemes => (
                "Provide a theme",
                "Offer a colour theme. Only validated colour tokens are accepted; no custom CSS.",
                RiskLevel::Low,
            ),
            Permission::StorageLocal => (
                "Store local data",
                "Save small amounts of data on this computer, private to this plugin and subject to a size quota.",
                RiskLevel::Low,
            ),
            Permission::EventsMessages => (
                "Read new messages",
                "Receive the text, author name and time of new messages as they arrive. The plugin cannot send network requests, so it cannot transmit them anywhere.",
                RiskLevel::Medium,
            ),
            Permission::CommandsSlash => (
                "Add slash commands",
                "Register /commands in the message composer. A command can rewrite the text you are about to send; you still press Enter to send it.",
                RiskLevel::Medium,
            ),
            Permission::Notifications => {
                ("Show notifications", "Show in-app notification toasts, rate-limited by DiZero.", RiskLevel::Low)
            }
        };
        PermissionInfo {
            permission: *self,
            key: self.key().to_string(),
            title: title.to_string(),
            description: description.to_string(),
            risk,
        }
    }
}

pub fn catalogue() -> Vec<PermissionInfo> {
    Permission::ALL.iter().map(|p| p.info()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_round_trip() {
        for p in Permission::ALL {
            assert_eq!(Permission::parse(p.key()), Some(p));
            let json = serde_json::to_string(&p).unwrap();
            assert_eq!(json, format!("\"{}\"", p.key()));
        }
    }

    #[test]
    fn planned_permissions_are_not_parseable() {
        for k in PLANNED_PERMISSIONS {
            assert!(Permission::parse(k).is_none());
        }
    }
}
