//! Event names and payloads pushed from Rust to the UI.

use dz_core::plugins::host::{HostSink, RuntimeState};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

pub const MESSAGE_CREATED: &str = "dz://message-created";
pub const MESSAGE_DELETED: &str = "dz://message-deleted";
pub const TYPING: &str = "dz://typing";
pub const PLUGIN_NOTIFY: &str = "dz://plugin-notify";
pub const PLUGIN_REFRESH: &str = "dz://plugin-refresh";
pub const PLUGIN_STATE: &str = "dz://plugin-state";
pub const PLUGINS_CHANGED: &str = "dz://plugins-changed";

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageDeleted {
    pub id: String,
    pub conversation_id: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Typing {
    pub conversation_id: String,
    pub user_id: String,
    pub display_name: String,
    pub active: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginNotify {
    pub plugin_id: String,
    pub title: String,
    pub body: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginRefresh {
    pub plugin_id: String,
    pub panel: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginStateChanged {
    pub plugin_id: String,
    pub state: RuntimeState,
    pub error: Option<String>,
}

/// Forwards plugin host effects to the UI and the log.
pub struct TauriSink {
    pub app: AppHandle,
}

impl HostSink for TauriSink {
    fn notify(&self, plugin_id: &str, title: &str, body: &str) {
        let _ = self
            .app
            .emit(PLUGIN_NOTIFY, PluginNotify { plugin_id: plugin_id.into(), title: title.into(), body: body.into() });
    }

    fn refresh(&self, plugin_id: &str, panel: &str) {
        let _ = self.app.emit(PLUGIN_REFRESH, PluginRefresh { plugin_id: plugin_id.into(), panel: panel.into() });
    }

    fn log(&self, plugin_id: &str, level: log::Level, msg: &str) {
        log::log!(target: &format!("plugin:{plugin_id}"), level, "{msg}");
    }

    fn state_changed(&self, plugin_id: &str, state: RuntimeState, error: Option<&str>) {
        if state == RuntimeState::Faulted {
            if let Some(s) = self.app.try_state::<crate::state::AppState>() {
                let _ = s.registry.set_last_error(plugin_id, error);
            }
        }
        let _ = self.app.emit(
            PLUGIN_STATE,
            PluginStateChanged { plugin_id: plugin_id.into(), state, error: error.map(str::to_string) },
        );
    }
}
