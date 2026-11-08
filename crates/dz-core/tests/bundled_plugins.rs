//! Every example plugin shipped in /plugins must validate, install and run.

use dz_core::db::Database;
use dz_core::plugins::host::{HostLimits, HostSink, PluginHost, RuntimeState};
use dz_core::plugins::manifest::PluginKind;
use dz_core::plugins::registry::{DbPluginStorage, PluginRegistry, PluginSource};
use dz_core::plugins::view::ViewNode;
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;

struct NullSink;
impl HostSink for NullSink {
    fn notify(&self, _: &str, _: &str, _: &str) {}
    fn refresh(&self, _: &str, _: &str) {}
    fn log(&self, _: &str, _: log::Level, _: &str) {}
    fn state_changed(&self, _: &str, _: RuntimeState, _: Option<&str>) {}
}

fn plugins_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../plugins").canonicalize().unwrap()
}

fn setup(name: &str) -> (Arc<Database>, PluginRegistry, PluginHost) {
    let db = Arc::new(Database::open_in_memory().unwrap());
    let root = std::env::temp_dir().join(format!("dz-bundled-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let registry = PluginRegistry::new(db.clone(), root);
    for entry in PluginRegistry::scan_catalog(&plugins_dir()) {
        let m = entry.manifest.unwrap_or_else(|| panic!("{} invalid: {:?}", entry.folder, entry.issues));
        registry.install_from_dir(&plugins_dir().join(&entry.folder), PluginSource::Bundled).unwrap();
        registry.approve_and_enable(&m.id, &m.permissions).unwrap();
    }
    let host = PluginHost::start(Arc::new(DbPluginStorage::new(db.clone())), Arc::new(NullSink), HostLimits::default());
    let (reqs, problems) = registry.load_requests().unwrap();
    assert!(problems.is_empty(), "{problems:?}");
    for r in reqs {
        host.register(r).unwrap();
    }
    (db, registry, host)
}

#[test]
fn catalog_contains_the_documented_examples() {
    let ids: Vec<String> =
        PluginRegistry::scan_catalog(&plugins_dir()).into_iter().map(|c| c.manifest.expect("valid").id).collect();
    for expected in [
        "com.dizero.slash-tools",
        "com.dizero.message-stats",
        "com.dizero.quick-notes",
        "com.dizero.ember-theme",
        "com.dizero.fault-lab",
    ] {
        assert!(ids.iter().any(|i| i == expected), "missing {expected}");
    }
}

#[test]
fn slash_tools_commands_work() {
    let (_db, _r, host) = setup("slash");
    let out = host.run_command("com.dizero.slash-tools", "shrug", "ok").unwrap();
    assert_eq!(out.replace_with.as_deref(), Some("ok ¯\\_(ツ)_/¯"));
    let out = host.run_command("com.dizero.slash-tools", "roll", "3d6").unwrap();
    assert!(out.replace_with.unwrap().starts_with("🎲 3d6:"));
    let out = host.run_command("com.dizero.slash-tools", "roll", "banana").unwrap();
    assert!(out.ephemeral.is_some());
    assert!(host.run_command("com.dizero.slash-tools", "time", "").unwrap().replace_with.is_some());
}

#[test]
fn message_stats_counts_events_and_renders() {
    let (_db, _r, host) = setup("stats");
    let empty = host.render_panel("com.dizero.message-stats", "activity").unwrap();
    assert!(empty.iter().any(|n| matches!(n, ViewNode::Empty { .. })));
    for i in 0..3 {
        host.dispatch_message_json(
            &json!({
                "id": format!("m{i}"), "conversationId": "c", "authorId": "u", "authorName": "Ada",
                "isSelf": i == 0, "isBot": false, "content": "one two three", "createdAt": 0
            })
            .to_string(),
        );
    }
    host.flush();
    let nodes = host.render_panel("com.dizero.message-stats", "activity").unwrap();
    assert!(
        nodes.iter().any(|n| matches!(n, ViewNode::Stat { label, value, .. } if label == "Messages" && value == "3"))
    );
    assert!(
        nodes
            .iter()
            .any(|n| matches!(n, ViewNode::Stat { label, value, .. } if label == "Sent by you" && value == "1"))
    );
    let reset = host.panel_action("com.dizero.message-stats", "activity", "reset", json!({})).unwrap();
    assert!(reset.iter().any(|n| matches!(n, ViewNode::Empty { .. })));
}

#[test]
fn quick_notes_add_pin_and_clear() {
    let (_db, _r, host) = setup("notes");
    let nodes = host.panel_action("com.dizero.quick-notes", "notes", "add", json!({ "text": "buy strings" })).unwrap();
    let id = nodes
        .iter()
        .find_map(|n| match n {
            ViewNode::List { items } => Some(items[0].id.clone()),
            _ => None,
        })
        .expect("note listed");
    host.panel_action("com.dizero.quick-notes", "notes", "pin", json!({ "itemId": id })).unwrap();
    let pinned = host.render_panel("com.dizero.quick-notes", "pinned").unwrap();
    assert!(matches!(&pinned[0], ViewNode::Text { text, .. } if text == "buy strings"));
    host.panel_action("com.dizero.quick-notes", "notes", "clear", json!({})).unwrap();
    let pinned = host.render_panel("com.dizero.quick-notes", "pinned").unwrap();
    assert!(matches!(&pinned[0], ViewNode::Text { text, .. } if text.starts_with("Pin a note")));
}

#[test]
fn fault_lab_is_contained() {
    let (_db, _r, host) = setup("faults");
    let id = "com.dizero.fault-lab";
    let escape = host.run_command(id, "escape", "").unwrap();
    assert!(escape.ephemeral.unwrap().starts_with("Nothing reachable"));
    let t = std::time::Instant::now();
    assert!(host.run_command(id, "hang", "").is_err());
    assert!(t.elapsed() < std::time::Duration::from_secs(2));
    assert!(host.run_command(id, "bomb", "").is_err());
    assert!(host.run_command(id, "crash", "").is_err());
    // Other plugins are unaffected.
    assert!(host.run_command("com.dizero.slash-tools", "shrug", "").is_ok());
}

#[test]
fn sdk_template_validates_and_runs() {
    let template =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../packages/dzsdk/template").canonicalize().unwrap();
    let report = PluginRegistry::validate_dir(&template);
    assert!(report.issues.is_empty(), "{:?}", report.issues);
    let m = report.manifest.unwrap();

    let db = Arc::new(Database::open_in_memory().unwrap());
    let host = PluginHost::start(Arc::new(DbPluginStorage::new(db)), Arc::new(NullSink), HostLimits::default());
    let source = std::fs::read_to_string(template.join("index.js")).unwrap();
    host.register(dz_core::plugins::host::LoadRequest {
        granted: m.permissions.iter().copied().collect(),
        manifest: m,
        source,
    })
    .unwrap();
    let out = host.run_command("com.example.hello", "hello", "Ada").unwrap();
    assert_eq!(out.replace_with.as_deref(), Some("Hello, Ada!"));
    let nodes = host.render_panel("com.example.hello", "main").unwrap();
    assert!(nodes.iter().any(|n| matches!(n, ViewNode::Stat { value, .. } if value == "1")));
}

#[test]
fn theme_plugin_is_data_only() {
    let (_db, registry, host) = setup("theme");
    let p = registry.get("com.dizero.ember-theme").unwrap();
    assert_eq!(p.manifest.kind, PluginKind::Theme);
    assert!(p.manifest.theme.unwrap().tokens.contains_key("accent"));
    assert!(host.stats().iter().all(|s| s.id != "com.dizero.ember-theme"), "themes never get a runtime");
}
