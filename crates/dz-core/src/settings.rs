//! Typed, validated application settings persisted in SQLite.

use crate::db::Database;
use crate::error::{CoreError, Result};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, Default)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum PerfMode {
    Eco,
    #[default]
    Balanced,
    Performance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, Default)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Density {
    Compact,
    #[default]
    Cozy,
    Comfortable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, Default)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum MotionPref {
    #[default]
    System,
    Reduce,
    Full,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, Default)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum LogLevel {
    Error,
    Warn,
    #[default]
    Info,
    Debug,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct AppearanceSettings {
    /// Built-in theme id or an installed theme plugin id.
    pub theme_id: String,
    /// Optional accent override as `#rrggbb`.
    pub accent: Option<String>,
    pub font_scale: f32,
    pub density: Density,
    pub motion: MotionPref,
    pub high_contrast: bool,
    pub show_avatars: bool,
    pub sidebar_width: u32,
    pub show_member_panel: bool,
}

impl Default for AppearanceSettings {
    fn default() -> Self {
        AppearanceSettings {
            theme_id: "dizero-dark".into(),
            accent: None,
            font_scale: 1.0,
            density: Density::Cozy,
            motion: MotionPref::System,
            high_contrast: false,
            show_avatars: true,
            sidebar_width: 248,
            show_member_panel: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct PerformanceSettings {
    pub mode: PerfMode,
    /// GPU rendering in WebView2. Off = software rendering: less memory,
    /// more CPU while the UI moves. Applied at the next launch.
    pub hardware_acceleration: bool,
}

impl Default for PerformanceSettings {
    fn default() -> Self {
        PerformanceSettings { mode: PerfMode::Balanced, hardware_acceleration: true }
    }
}

/// WebView2 browser arguments for this launch. Supplying arguments replaces
/// wry's defaults, so those are restated first. Rationale and measurements:
/// docs/decisions/0005-webview2-arguments.md.
pub fn webview_browser_args(hardware_acceleration: bool) -> String {
    let mut args = String::from(
        "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --enable-features=NetworkServiceInProcess2",
    );
    if !hardware_acceleration {
        args.push_str(" --disable-gpu");
    }
    args
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct NotificationSettings {
    /// In-app toasts for new messages in conversations you are not viewing.
    pub in_app: bool,
    /// Only notify when you are mentioned.
    pub mentions_only: bool,
}

impl Default for NotificationSettings {
    fn default() -> Self {
        NotificationSettings { in_app: true, mentions_only: true }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct ProfileSettings {
    pub display_name: String,
}

impl Default for ProfileSettings {
    fn default() -> Self {
        ProfileSettings { display_name: "You".into() }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, Default)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct PluginSettings {
    /// Shows extra diagnostics and allows validating plugin folders.
    pub developer_mode: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, Default)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct AdvancedSettings {
    pub log_level: LogLevel,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, Default)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct WorkspaceState {
    pub last_community: Option<String>,
    pub last_conversation: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, Default)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct Settings {
    pub appearance: AppearanceSettings,
    pub performance: PerformanceSettings,
    pub notifications: NotificationSettings,
    pub profile: ProfileSettings,
    pub plugins: PluginSettings,
    pub advanced: AdvancedSettings,
    pub workspace: WorkspaceState,
}

/// Concrete runtime behaviour derived from a [`PerfMode`].
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PerfProfile {
    /// Max messages kept rendered/in memory per open conversation.
    pub message_window: u32,
    /// Messages fetched per page.
    pub page_size: u32,
    pub animations: bool,
    /// Prefetch neighbouring conversations on hover.
    pub preload_adjacent: bool,
    /// Unload idle plugin runtimes after this many seconds (None = never).
    #[ts(type = "number | null")]
    pub plugin_idle_unload_secs: Option<u64>,
    /// Performance Center sampling interval while it is open.
    pub metrics_interval_ms: u32,
}

impl PerfMode {
    pub fn profile(&self) -> PerfProfile {
        match self {
            PerfMode::Eco => PerfProfile {
                message_window: 150,
                page_size: 40,
                animations: false,
                preload_adjacent: false,
                plugin_idle_unload_secs: Some(60),
                metrics_interval_ms: 5000,
            },
            PerfMode::Balanced => PerfProfile {
                message_window: 300,
                page_size: 50,
                animations: true,
                preload_adjacent: false,
                plugin_idle_unload_secs: Some(600),
                metrics_interval_ms: 2000,
            },
            PerfMode::Performance => PerfProfile {
                message_window: 800,
                page_size: 80,
                animations: true,
                preload_adjacent: true,
                plugin_idle_unload_secs: None,
                metrics_interval_ms: 1000,
            },
        }
    }
}

const SETTINGS_KEY: &str = "settings";

/// Accepts `#rgb` or `#rrggbb` and returns the lowercase `#rrggbb` form.
pub fn normalize_hex_color(input: &str) -> Option<String> {
    let s = input.trim();
    let hex = s.strip_prefix('#')?;
    if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    match hex.len() {
        6 => Some(format!("#{}", hex.to_ascii_lowercase())),
        3 => {
            let expanded: String = hex.chars().flat_map(|c| [c, c]).collect();
            Some(format!("#{}", expanded.to_ascii_lowercase()))
        }
        _ => None,
    }
}

impl Settings {
    /// Clamp and sanitise every field so stored settings are always valid,
    /// regardless of what the UI (or an imported file) sent.
    pub fn normalized(mut self) -> Settings {
        let a = &mut self.appearance;
        a.font_scale = if a.font_scale.is_finite() { a.font_scale.clamp(0.85, 1.3) } else { 1.0 };
        a.font_scale = (a.font_scale * 100.0).round() / 100.0;
        a.accent = a.accent.as_deref().and_then(normalize_hex_color);
        a.sidebar_width = a.sidebar_width.clamp(200, 420);
        let theme = a.theme_id.trim();
        let theme_ok = !theme.is_empty()
            && theme.len() <= 64
            && theme.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.' || c == '_');
        a.theme_id = if theme_ok { theme.to_string() } else { AppearanceSettings::default().theme_id };

        let name = crate::util::truncate_chars(self.profile.display_name.trim(), 32);
        self.profile.display_name = if name.is_empty() { ProfileSettings::default().display_name } else { name };

        for id in [&mut self.workspace.last_community, &mut self.workspace.last_conversation] {
            if id.as_ref().is_some_and(|v| v.len() > 128) {
                *id = None;
            }
        }
        self
    }
}

pub fn load(db: &Database) -> Result<Settings> {
    let raw: Option<String> = db.with(|c| {
        let mut stmt = c.prepare_cached("SELECT value FROM app_settings WHERE key = ?1")?;
        let mut rows = stmt.query([SETTINGS_KEY])?;
        Ok(match rows.next()? {
            Some(row) => Some(row.get(0)?),
            None => None,
        })
    })?;
    Ok(match raw {
        None => Settings::default(),
        Some(json) => match serde_json::from_str::<Settings>(&json) {
            Ok(s) => s.normalized(),
            Err(e) => {
                log::warn!("stored settings unreadable ({e}); using defaults");
                Settings::default()
            }
        },
    })
}

pub fn save(db: &Database, s: Settings) -> Result<Settings> {
    let s = s.normalized();
    let json = serde_json::to_string(&s)?;
    db.with(|c| {
        c.prepare_cached(
            "INSERT INTO app_settings(key, value, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        )?
        .execute(rusqlite::params![SETTINGS_KEY, json, crate::util::now_ms()])?;
        Ok(())
    })?;
    Ok(s)
}

#[derive(Serialize, Deserialize)]
struct ExportEnvelope {
    format: String,
    version: u32,
    settings: Settings,
}

const EXPORT_FORMAT: &str = "dizero-settings";

pub fn export_json(s: &Settings) -> Result<String> {
    let env = ExportEnvelope { format: EXPORT_FORMAT.into(), version: 1, settings: s.clone() };
    Ok(serde_json::to_string_pretty(&env)?)
}

pub fn import_json(json: &str) -> Result<Settings> {
    let env: ExportEnvelope =
        serde_json::from_str(json).map_err(|e| CoreError::invalid(format!("not a DiZero settings file: {e}")))?;
    if env.format != EXPORT_FORMAT || env.version != 1 {
        return Err(CoreError::invalid("unsupported settings file format"));
    }
    Ok(env.settings.normalized())
}
