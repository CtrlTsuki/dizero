use dz_core::plugins::manifest::{PluginKind, PluginManifest};
use dz_core::plugins::permissions::Permission;
use serde_json::json;

fn base() -> serde_json::Value {
    json!({
        "id": "com.example.stats",
        "name": "Message Stats",
        "version": "1.2.3",
        "description": "Counts things.",
        "author": "Example Dev",
        "apiVersion": "1",
        "kind": "script",
        "category": "productivity",
        "entry": "index.js",
        "permissions": ["events.messages", "storage.local", "ui.panels"],
        "contributes": {
            "panels": [{ "id": "stats", "title": "Stats", "placement": "dock", "icon": "bar-chart" }],
            "events": ["messageCreated"]
        }
    })
}

fn issues_for(v: serde_json::Value) -> Vec<String> {
    match PluginManifest::parse(&v.to_string()) {
        Ok(_) => vec![],
        Err(issues) => issues.into_iter().map(|i| format!("{}: {}", i.field, i.message)).collect(),
    }
}

fn assert_rejected(v: serde_json::Value, field: &str) {
    let issues = issues_for(v);
    assert!(issues.iter().any(|i| i.starts_with(field)), "expected an issue on `{field}`, got {issues:?}");
}

#[test]
fn valid_manifest_parses() {
    let m = PluginManifest::parse(&base().to_string()).expect("valid");
    assert_eq!(m.id, "com.example.stats");
    assert_eq!(m.kind, PluginKind::Script);
    assert!(m.permissions.contains(&Permission::EventsMessages));
    assert_eq!(m.contributes.panels.len(), 1);
}

#[test]
fn rejects_malformed_id() {
    let mut v = base();
    v["id"] = json!("Not A Valid ID");
    assert_rejected(v, "id");
}

#[test]
fn rejects_non_semver_version() {
    let mut v = base();
    v["version"] = json!("one point oh");
    assert_rejected(v, "version");
}

#[test]
fn rejects_unknown_permission() {
    let mut v = base();
    v["permissions"] = json!(["storage.local", "root.everything"]);
    assert_rejected(v, "permissions");
}

#[test]
fn rejects_recognised_but_unsupported_permission_with_clear_message() {
    let mut v = base();
    v["permissions"] = json!(["network.request"]);
    let issues = issues_for(v);
    assert!(issues.iter().any(|i| i.contains("network.request") && i.contains("not supported")), "{issues:?}");
}

#[test]
fn rejects_entry_path_traversal() {
    for bad in ["../evil.js", "C:\\Windows\\evil.js", "/etc/evil.js", "sub/../../x.js", "index.exe"] {
        let mut v = base();
        v["entry"] = json!(bad);
        assert_rejected(v, "entry");
    }
}

#[test]
fn script_plugins_require_an_entry() {
    let mut v = base();
    v.as_object_mut().unwrap().remove("entry");
    assert_rejected(v, "entry");
}

#[test]
fn rejects_unsupported_api_version() {
    let mut v = base();
    v["apiVersion"] = json!("99");
    assert_rejected(v, "apiVersion");
}

#[test]
fn contributions_require_matching_permissions() {
    let mut v = base();
    v["permissions"] = json!(["storage.local"]);
    let issues = issues_for(v);
    assert!(issues.iter().any(|i| i.contains("ui.panels")), "{issues:?}");
    assert!(issues.iter().any(|i| i.contains("events.messages")), "{issues:?}");
}

#[test]
fn command_names_are_validated() {
    let mut v = base();
    v["permissions"] = json!(["commands.slash"]);
    v["contributes"] = json!({ "commands": [{ "name": "Bad Name!", "description": "x" }] });
    assert_rejected(v, "contributes.commands");
}

#[test]
fn theme_plugins_need_valid_color_tokens() {
    let theme = json!({
        "id": "com.example.ember",
        "name": "Ember",
        "version": "1.0.0",
        "description": "Warm theme",
        "author": "Example",
        "apiVersion": "1",
        "kind": "theme",
        "category": "appearance",
        "permissions": ["ui.themes"],
        "theme": { "label": "Ember", "tokens": { "accent": "#ff8a5c", "background": "#120f0e" } }
    });
    let m = PluginManifest::parse(&theme.to_string()).expect("valid theme");
    assert_eq!(m.kind, PluginKind::Theme);

    let mut bad = theme.clone();
    bad["theme"]["tokens"]["accent"] = json!("red; background: url(http://evil)");
    assert_rejected(bad, "theme.tokens.accent");

    let mut unknown = theme;
    unknown["theme"]["tokens"]["--anything"] = json!("#000000");
    assert_rejected(unknown, "theme.tokens");
}

#[test]
fn dependency_requirements_must_be_valid() {
    let mut v = base();
    v["dependencies"] = json!({ "com.example.core": "not a range" });
    assert_rejected(v, "dependencies");
}

#[test]
fn reports_all_issues_at_once() {
    let mut v = base();
    v["id"] = json!("x");
    v["version"] = json!("bad");
    v["name"] = json!("");
    assert!(issues_for(v).len() >= 3);
}

#[test]
fn rejects_non_object_json() {
    assert!(PluginManifest::parse("[]").is_err());
    assert!(PluginManifest::parse("{").is_err());
}
