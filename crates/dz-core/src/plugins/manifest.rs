//! Plugin manifest (`dizero-plugin.json`) parsing and validation.

use super::permissions::Permission;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

pub const MANIFEST_FILE: &str = "dizero-plugin.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum PluginKind {
    Script,
    Theme,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum PluginCategory {
    Interface,
    Appearance,
    Messaging,
    Productivity,
    Accessibility,
    Media,
    Integration,
    Developer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum PanelPlacement {
    Dock,
    Sidebar,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CommandContribution {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub usage: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PanelContribution {
    pub id: String,
    pub title: String,
    pub placement: PanelPlacement,
    #[serde(default)]
    pub icon: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct Contributions {
    pub commands: Vec<CommandContribution>,
    pub panels: Vec<PanelContribution>,
    /// Event names the plugin subscribes to. v1 supports `messageCreated`.
    pub events: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ThemeDefinition {
    pub label: String,
    /// Token name → `#rrggbb`. See [`THEME_TOKENS`].
    pub tokens: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub api_version: String,
    pub kind: PluginKind,
    pub category: PluginCategory,
    #[serde(default)]
    pub entry: Option<String>,
    #[serde(default)]
    pub permissions: Vec<Permission>,
    #[serde(default)]
    pub contributes: Contributions,
    /// Plugin id → semver requirement.
    #[serde(default)]
    pub dependencies: BTreeMap<String, String>,
    /// Minimum DiZero version as a semver requirement, e.g. `>=0.1.0`.
    #[serde(default)]
    pub engine: Option<String>,
    #[serde(default)]
    pub theme: Option<ThemeDefinition>,
    #[serde(default)]
    pub homepage: Option<String>,
    /// Reserved for signed distribution. Present signatures are recorded but
    /// NOT verified by this build (no signing infrastructure yet).
    #[serde(default)]
    pub signature: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ValidationIssue {
    pub field: String,
    pub message: String,
}

/// Colour tokens a theme may override.
pub const THEME_TOKENS: &[&str] = &[
    "background",
    "surface",
    "surfaceSecondary",
    "surfaceElevated",
    "accent",
    "accentHover",
    "accentMuted",
    "textPrimary",
    "textSecondary",
    "textMuted",
    "border",
    "success",
    "warning",
    "danger",
];

/// Icon names a panel may request. The UI maps these to bundled icons.
pub const PANEL_ICONS: &[&str] =
    &["bar-chart", "notebook", "list", "sparkles", "clock", "star", "puzzle", "bell", "bookmark", "zap"];

/// Hard cap on manifest size so a hostile file can't exhaust memory.
pub const MAX_MANIFEST_BYTES: usize = 64 * 1024;

struct Issues(Vec<ValidationIssue>);

impl Issues {
    fn push(&mut self, field: impl Into<String>, message: impl Into<String>) {
        self.0.push(ValidationIssue { field: field.into(), message: message.into() });
    }
}

fn is_valid_id(id: &str) -> bool {
    // Reverse-domain: at least two dot-separated segments of [a-z0-9-],
    // each starting with a letter or digit. 3–64 chars total.
    if id.len() < 3 || id.len() > 64 {
        return false;
    }
    let segments: Vec<&str> = id.split('.').collect();
    segments.len() >= 2
        && segments.iter().all(|s| {
            !s.is_empty()
                && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
                && !s.starts_with('-')
                && !s.ends_with('-')
        })
}

fn is_valid_slug(s: &str, max: usize) -> bool {
    !s.is_empty()
        && s.len() <= max
        && s.chars().next().is_some_and(|c| c.is_ascii_lowercase())
        && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// A safe relative path to a `.js` file inside the plugin folder.
pub fn is_safe_entry(entry: &str) -> bool {
    if entry.is_empty() || entry.len() > 128 || !entry.ends_with(".js") {
        return false;
    }
    if entry.starts_with('/') || entry.starts_with('\\') || entry.contains(':') {
        return false;
    }
    entry.split(['/', '\\']).all(|seg| {
        !seg.is_empty()
            && seg != "."
            && seg != ".."
            && seg.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    })
}

fn str_field<'a>(
    obj: &'a serde_json::Map<String, serde_json::Value>,
    field: &str,
    max: usize,
    required: bool,
    issues: &mut Issues,
) -> Option<&'a str> {
    match obj.get(field) {
        None | Some(serde_json::Value::Null) => {
            if required {
                issues.push(field, "is required");
            }
            None
        }
        Some(serde_json::Value::String(s)) => {
            let t = s.trim();
            if required && t.is_empty() {
                issues.push(field, "must not be empty");
                None
            } else if s.chars().count() > max {
                issues.push(field, format!("must be at most {max} characters"));
                None
            } else if s.chars().any(|c| c.is_control()) {
                issues.push(field, "must not contain control characters");
                None
            } else {
                Some(s.as_str())
            }
        }
        Some(_) => {
            issues.push(field, "must be a string");
            None
        }
    }
}

impl PluginManifest {
    /// Parse and fully validate. Returns every problem found, not just the first.
    pub fn parse(json: &str) -> Result<PluginManifest, Vec<ValidationIssue>> {
        let mut issues = Issues(Vec::new());
        if json.len() > MAX_MANIFEST_BYTES {
            issues.push("manifest", format!("is larger than {MAX_MANIFEST_BYTES} bytes"));
            return Err(issues.0);
        }
        let value: serde_json::Value = match serde_json::from_str(json) {
            Ok(v) => v,
            Err(e) => {
                issues.push("manifest", format!("is not valid JSON: {e}"));
                return Err(issues.0);
            }
        };
        let Some(obj) = value.as_object() else {
            issues.push("manifest", "must be a JSON object");
            return Err(issues.0);
        };

        let id = str_field(obj, "id", 64, true, &mut issues);
        if let Some(id) = id {
            if !is_valid_id(id) {
                issues.push(
                    "id",
                    "must be a reverse-domain id like `com.example.my-plugin` (lowercase letters, digits, dashes)",
                );
            }
        }
        str_field(obj, "name", 48, true, &mut issues);
        str_field(obj, "description", 300, true, &mut issues);
        str_field(obj, "author", 64, true, &mut issues);
        if let Some(v) = str_field(obj, "version", 32, true, &mut issues) {
            if semver::Version::parse(v).is_err() {
                issues.push("version", "must be a semantic version such as 1.0.0");
            }
        }
        if let Some(v) = str_field(obj, "apiVersion", 8, true, &mut issues) {
            if v != super::API_VERSION {
                issues.push(
                    "apiVersion",
                    format!("`{v}` is not supported; this DiZero build implements DZSDK API v{}", super::API_VERSION),
                );
            }
        }
        if let Some(h) = str_field(obj, "homepage", 200, false, &mut issues) {
            if !h.starts_with("https://") {
                issues.push("homepage", "must be an https:// URL");
            }
        }
        str_field(obj, "signature", 4096, false, &mut issues);
        if let Some(e) = str_field(obj, "engine", 64, false, &mut issues) {
            if semver::VersionReq::parse(e).is_err() {
                issues.push("engine", "must be a semver requirement such as >=0.1.0");
            }
        }

        let kind: Option<PluginKind> = match obj.get("kind") {
            Some(v) => match serde_json::from_value(v.clone()) {
                Ok(k) => Some(k),
                Err(_) => {
                    issues.push("kind", "must be `script` or `theme`");
                    None
                }
            },
            None => {
                issues.push("kind", "is required");
                None
            }
        };
        match obj.get("category") {
            Some(v) if serde_json::from_value::<PluginCategory>(v.clone()).is_ok() => {}
            Some(_) => issues.push(
                "category",
                "must be one of interface, appearance, messaging, productivity, accessibility, media, integration, developer",
            ),
            None => issues.push("category", "is required"),
        }

        // Permissions: collect parsed ones; report unknown / planned keys.
        let mut perms: Vec<Permission> = Vec::new();
        match obj.get("permissions") {
            None => {}
            Some(serde_json::Value::Array(items)) => {
                if items.len() > 32 {
                    issues.push("permissions", "lists too many permissions");
                }
                for item in items {
                    match item.as_str() {
                        Some(k) => match Permission::parse(k) {
                            Some(p) if !perms.contains(&p) => perms.push(p),
                            Some(_) => {}
                            None if super::permissions::PLANNED_PERMISSIONS.contains(&k) => {
                                issues.push("permissions", format!("`{k}` is not supported by this DiZero version yet"))
                            }
                            None => issues.push("permissions", format!("`{k}` is not a known permission")),
                        },
                        None => issues.push("permissions", "entries must be strings"),
                    }
                }
            }
            Some(_) => issues.push("permissions", "must be an array of strings"),
        }
        let has = |p: Permission| perms.contains(&p);

        // Entry file.
        let entry = str_field(obj, "entry", 128, false, &mut issues);
        match (kind, entry) {
            (Some(PluginKind::Script), None) => issues.push("entry", "is required for script plugins"),
            (_, Some(e)) if !is_safe_entry(e) => {
                issues.push("entry", "must be a relative path to a .js file inside the plugin folder (no `..`, drive letters or absolute paths)")
            }
            _ => {}
        }

        // Contributions.
        let contributes: Contributions = match obj.get("contributes") {
            None => Contributions::default(),
            Some(v) => match serde_json::from_value(v.clone()) {
                Ok(c) => c,
                Err(e) => {
                    issues.push("contributes", format!("is malformed: {e}"));
                    Contributions::default()
                }
            },
        };
        if contributes.commands.len() > 20 {
            issues.push("contributes.commands", "at most 20 commands");
        }
        for c in &contributes.commands {
            if !is_valid_slug(&c.name, 32) {
                issues.push(
                    "contributes.commands",
                    format!("`{}` must be lowercase letters, digits or dashes, starting with a letter", c.name),
                );
            }
            if c.description.trim().is_empty() || c.description.chars().count() > 120 {
                issues.push("contributes.commands", format!("`{}` needs a description of 1–120 characters", c.name));
            }
        }
        if !contributes.commands.is_empty() && !has(Permission::CommandsSlash) {
            issues.push("permissions", "contributing commands requires the `commands.slash` permission");
        }
        if contributes.panels.len() > 8 {
            issues.push("contributes.panels", "at most 8 panels");
        }
        let mut seen_panels = BTreeMap::new();
        for p in &contributes.panels {
            if !is_valid_slug(&p.id, 32) {
                issues.push("contributes.panels", format!("panel id `{}` must be a lowercase slug", p.id));
            }
            if seen_panels.insert(p.id.clone(), ()).is_some() {
                issues.push("contributes.panels", format!("duplicate panel id `{}`", p.id));
            }
            if p.title.trim().is_empty() || p.title.chars().count() > 40 {
                issues.push("contributes.panels", format!("panel `{}` needs a title of 1–40 characters", p.id));
            }
            if let Some(icon) = &p.icon {
                if !PANEL_ICONS.contains(&icon.as_str()) {
                    issues.push(
                        "contributes.panels",
                        format!("icon `{icon}` is not available; use one of {}", PANEL_ICONS.join(", ")),
                    );
                }
            }
            match p.placement {
                PanelPlacement::Dock if !has(Permission::UiPanels) => {
                    issues.push("permissions", "dock panels require the `ui.panels` permission")
                }
                PanelPlacement::Sidebar if !has(Permission::UiSidebar) => {
                    issues.push("permissions", "sidebar sections require the `ui.sidebar` permission")
                }
                _ => {}
            }
        }
        for ev in &contributes.events {
            if ev != "messageCreated" {
                issues.push(
                    "contributes.events",
                    format!("`{ev}` is not a supported event (v1 supports messageCreated)"),
                );
            }
        }
        if contributes.events.iter().any(|e| e == "messageCreated") && !has(Permission::EventsMessages) {
            issues.push("permissions", "subscribing to messageCreated requires the `events.messages` permission");
        }

        // Theme.
        let theme: Option<ThemeDefinition> = match obj.get("theme") {
            None | Some(serde_json::Value::Null) => None,
            Some(v) => match serde_json::from_value(v.clone()) {
                Ok(t) => Some(t),
                Err(e) => {
                    issues.push("theme", format!("is malformed: {e}"));
                    None
                }
            },
        };
        if kind == Some(PluginKind::Theme) {
            if !has(Permission::UiThemes) {
                issues.push("permissions", "theme plugins require the `ui.themes` permission");
            }
            match &theme {
                None => issues.push("theme", "is required for theme plugins"),
                Some(t) => {
                    if t.label.trim().is_empty() || t.label.chars().count() > 40 {
                        issues.push("theme.label", "must be 1–40 characters");
                    }
                    if t.tokens.is_empty() {
                        issues.push("theme.tokens", "must define at least one token");
                    }
                    for (k, v) in &t.tokens {
                        if !THEME_TOKENS.contains(&k.as_str()) {
                            issues.push("theme.tokens", format!("`{k}` is not a themeable token"));
                        } else if crate::settings::normalize_hex_color(v).is_none() {
                            issues.push(format!("theme.tokens.{k}"), "must be a hex colour like #9278f5");
                        }
                    }
                }
            }
        } else if theme.is_some() {
            issues.push("theme", "is only allowed on theme plugins");
        }

        // Dependencies.
        let deps: BTreeMap<String, String> = match obj.get("dependencies") {
            None => BTreeMap::new(),
            Some(v) => match serde_json::from_value(v.clone()) {
                Ok(d) => d,
                Err(_) => {
                    issues.push("dependencies", "must map plugin ids to version requirements");
                    BTreeMap::new()
                }
            },
        };
        for (dep_id, req) in &deps {
            if !is_valid_id(dep_id) {
                issues.push("dependencies", format!("`{dep_id}` is not a valid plugin id"));
            }
            if semver::VersionReq::parse(req).is_err() {
                issues.push("dependencies", format!("`{req}` for `{dep_id}` is not a valid version requirement"));
            }
            if Some(dep_id.as_str()) == id {
                issues.push("dependencies", "a plugin cannot depend on itself");
            }
        }

        if !issues.0.is_empty() {
            return Err(issues.0);
        }

        let mut manifest: PluginManifest = serde_json::from_value(value)
            .map_err(|e| vec![ValidationIssue { field: "manifest".into(), message: e.to_string() }])?;
        // Normalise: dedupe permissions, canonical colour casing.
        manifest.permissions = perms;
        if let Some(t) = manifest.theme.as_mut() {
            for v in t.tokens.values_mut() {
                if let Some(n) = crate::settings::normalize_hex_color(v) {
                    *v = n;
                }
            }
        }
        Ok(manifest)
    }
}
