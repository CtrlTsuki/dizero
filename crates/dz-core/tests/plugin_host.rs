use dz_core::plugins::host::{
    HostLimits, HostSink, LoadRequest, MemoryPluginStorage, PluginHost, PluginStorage, RuntimeState,
};
use dz_core::plugins::manifest::PluginManifest;
use dz_core::plugins::permissions::Permission;
use dz_core::plugins::view::ViewNode;
use serde_json::json;
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Default)]
struct RecordingSink {
    notifications: Mutex<Vec<(String, String)>>,
    refreshes: Mutex<Vec<String>>,
    logs: Mutex<Vec<String>>,
    states: Mutex<Vec<(String, RuntimeState)>>,
}

impl HostSink for RecordingSink {
    fn notify(&self, plugin_id: &str, title: &str, _body: &str) {
        self.notifications.lock().unwrap().push((plugin_id.into(), title.into()));
    }
    fn refresh(&self, _plugin_id: &str, panel: &str) {
        self.refreshes.lock().unwrap().push(panel.into());
    }
    fn log(&self, _plugin_id: &str, _level: log::Level, msg: &str) {
        self.logs.lock().unwrap().push(msg.into());
    }
    fn state_changed(&self, plugin_id: &str, state: RuntimeState, _error: Option<&str>) {
        self.states.lock().unwrap().push((plugin_id.into(), state));
    }
}

fn manifest(id: &str, perms: &[&str], contributes: serde_json::Value) -> PluginManifest {
    let v = json!({
        "id": id, "name": "Test", "version": "1.0.0", "description": "t", "author": "t",
        "apiVersion": "1", "kind": "script", "category": "developer", "entry": "index.js",
        "permissions": perms, "contributes": contributes
    });
    PluginManifest::parse(&v.to_string()).expect("test manifest must be valid")
}

fn limits() -> HostLimits {
    HostLimits {
        memory_bytes: 4 * 1024 * 1024,
        call_budget: Duration::from_millis(100),
        activate_budget: Duration::from_millis(250),
        storage_quota_bytes: 4096,
        max_failures: 3,
    }
}

struct Fixture {
    host: PluginHost,
    sink: Arc<RecordingSink>,
    storage: Arc<MemoryPluginStorage>,
}

fn fixture() -> Fixture {
    let sink = Arc::new(RecordingSink::default());
    let storage = Arc::new(MemoryPluginStorage::default());
    let host = PluginHost::start(storage.clone(), sink.clone(), limits());
    Fixture { host, sink, storage }
}

fn load(f: &Fixture, m: PluginManifest, source: &str) {
    let granted: BTreeSet<Permission> = m.permissions.iter().copied().collect();
    f.host.register(LoadRequest { manifest: m, source: source.into(), granted }).unwrap();
}

const SHRUG: &str = r#"
dz.plugin({
  commands: {
    shrug: { run(args) { return { replaceWith: (args ? args + " " : "") + "¯\\_(ツ)_/¯" }; } }
  }
});
"#;

fn shrug_manifest(id: &str) -> PluginManifest {
    manifest(id, &["commands.slash"], json!({ "commands": [{ "name": "shrug", "description": "shrug" }] }))
}

#[test]
fn runs_a_declared_command() {
    let f = fixture();
    load(&f, shrug_manifest("com.test.shrug"), SHRUG);
    let out = f.host.run_command("com.test.shrug", "shrug", "ok").unwrap();
    assert_eq!(out.replace_with.as_deref(), Some("ok ¯\\_(ツ)_/¯"));
}

#[test]
fn plugins_are_dormant_until_first_use() {
    let f = fixture();
    load(&f, shrug_manifest("com.test.lazy"), SHRUG);
    let s = f.host.stats().into_iter().find(|s| s.id == "com.test.lazy").unwrap();
    assert_eq!(s.state, RuntimeState::Dormant);
    assert!(s.heap_bytes.is_none());
    f.host.run_command("com.test.lazy", "shrug", "").unwrap();
    let s = f.host.stats().into_iter().find(|s| s.id == "com.test.lazy").unwrap();
    assert_eq!(s.state, RuntimeState::Active);
    assert!(s.heap_bytes.unwrap() > 0);
}

#[test]
fn undeclared_commands_cannot_be_called() {
    let f = fixture();
    let src = r#"dz.plugin({ commands: { shrug: { run(){ return {}; } }, secret: { run(){ return { replaceWith: "leak" }; } } } });"#;
    load(&f, shrug_manifest("com.test.undeclared"), src);
    assert!(f.host.run_command("com.test.undeclared", "secret", "").is_err());
}

#[test]
fn storage_api_is_absent_without_permission() {
    let f = fixture();
    let src = r#"dz.plugin({ commands: { shrug: { run(){ dz.storage.set("k", 1); return {}; } } } });"#;
    load(&f, shrug_manifest("com.test.nostorage"), src);
    let err = f.host.run_command("com.test.nostorage", "shrug", "").unwrap_err();
    assert!(err.to_string().to_lowercase().contains("storage"), "{err}");
    assert!(f.storage.get("com.test.nostorage", "k").unwrap().is_none());
}

#[test]
fn storage_persists_across_runtime_reloads() {
    let f = fixture();
    let m = manifest(
        "com.test.counter",
        &["commands.slash", "storage.local"],
        json!({ "commands": [{ "name": "count", "description": "c" }] }),
    );
    let src = r#"dz.plugin({ commands: { count: { run(){ const n = (dz.storage.get("n") ?? 0) + 1; dz.storage.set("n", n); return { replaceWith: String(n) }; } } } });"#;
    load(&f, m.clone(), src);
    assert_eq!(f.host.run_command("com.test.counter", "count", "").unwrap().replace_with.as_deref(), Some("1"));
    f.host.unregister("com.test.counter").unwrap();
    load(&f, m, src);
    assert_eq!(f.host.run_command("com.test.counter", "count", "").unwrap().replace_with.as_deref(), Some("2"));
}

#[test]
fn storage_quota_is_enforced() {
    let f = fixture();
    let m = manifest(
        "com.test.hog",
        &["commands.slash", "storage.local"],
        json!({ "commands": [{ "name": "fill", "description": "f" }] }),
    );
    let src =
        r#"dz.plugin({ commands: { fill: { run(){ dz.storage.set("big", "x".repeat(10000)); return {}; } } } });"#;
    load(&f, m, src);
    let err = f.host.run_command("com.test.hog", "fill", "").unwrap_err();
    assert!(err.to_string().contains("quota"), "{err}");
}

#[test]
fn infinite_loop_times_out_and_host_stays_usable() {
    let f = fixture();
    let src = r#"dz.plugin({ commands: { shrug: { run(){ while(true){} } } } });"#;
    load(&f, shrug_manifest("com.test.loop"), src);
    load(&f, shrug_manifest("com.test.good"), SHRUG);
    let started = std::time::Instant::now();
    let err = f.host.run_command("com.test.loop", "shrug", "").unwrap_err();
    assert!(err.to_string().contains("time"), "{err}");
    assert!(started.elapsed() < Duration::from_secs(2));
    assert!(f.host.run_command("com.test.good", "shrug", "").is_ok());
}

#[test]
fn memory_bomb_hits_the_limit_without_crashing_host() {
    let f = fixture();
    let src = r#"dz.plugin({ commands: { shrug: { run(){ const a = []; for(;;) a.push("xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx" + a.length); } } } });"#;
    load(&f, shrug_manifest("com.test.bomb"), src);
    load(&f, shrug_manifest("com.test.fine"), SHRUG);
    assert!(f.host.run_command("com.test.bomb", "shrug", "").is_err());
    assert!(f.host.run_command("com.test.fine", "shrug", "").is_ok());
}

#[test]
fn syntax_errors_fail_activation_cleanly() {
    let f = fixture();
    load(&f, shrug_manifest("com.test.syntax"), "dz.plugin({ commands: {");
    let err = f.host.run_command("com.test.syntax", "shrug", "").unwrap_err();
    assert!(!err.to_string().is_empty());
}

#[test]
fn repeated_failures_fault_the_plugin() {
    let f = fixture();
    let src = r#"dz.plugin({ commands: { shrug: { run(){ throw new Error("boom"); } } } });"#;
    load(&f, shrug_manifest("com.test.flaky"), src);
    for _ in 0..3 {
        assert!(f.host.run_command("com.test.flaky", "shrug", "").is_err());
    }
    let s = f.host.stats().into_iter().find(|s| s.id == "com.test.flaky").unwrap();
    assert_eq!(s.state, RuntimeState::Faulted);
    assert!(s.last_error.unwrap().contains("boom"));
    assert!(
        f.sink.states.lock().unwrap().iter().any(|(id, st)| id == "com.test.flaky" && *st == RuntimeState::Faulted)
    );
    // Faulted plugins refuse further calls until reset.
    assert!(f.host.run_command("com.test.flaky", "shrug", "").is_err());
}

#[test]
fn message_events_reach_only_subscribed_plugins() {
    let f = fixture();
    let listener = manifest(
        "com.test.listener",
        &["events.messages", "storage.local", "commands.slash"],
        json!({ "events": ["messageCreated"], "commands": [{ "name": "seen", "description": "s" }] }),
    );
    let src = r#"dz.plugin({
        events: { messageCreated(m) { dz.storage.set("last", m.content); } },
        commands: { seen: { run(){ return { replaceWith: String(dz.storage.get("last")) }; } } }
    });"#;
    load(&f, listener, src);
    f.host.dispatch_message_json(r#"{"id":"m1","conversationId":"c1","authorId":"u1","authorName":"Ada","isSelf":false,"isBot":false,"content":"hello there","createdAt":0}"#);
    f.host.flush();
    let out = f.host.run_command("com.test.listener", "seen", "").unwrap();
    assert_eq!(out.replace_with.as_deref(), Some("hello there"));
}

#[test]
fn panels_render_sanitized_nodes_and_handle_actions() {
    let f = fixture();
    let m = manifest(
        "com.test.panel",
        &["ui.panels", "storage.local"],
        json!({ "panels": [{ "id": "main", "title": "Main", "placement": "dock", "icon": "list" }] }),
    );
    let src = r#"dz.plugin({ panels: { main: {
        render(){ return [
            { type: "heading", text: "Notes" },
            { type: "html", html: "<script>" },
            { type: "text", text: String(dz.storage.get("note") ?? "none") }
        ]; },
        action(id, values){ if (id === "save") dz.storage.set("note", values.note); }
    } } });"#;
    load(&f, m, src);
    let nodes = f.host.render_panel("com.test.panel", "main").unwrap();
    assert_eq!(nodes.len(), 2, "html node must be dropped");
    let nodes = f.host.panel_action("com.test.panel", "main", "save", json!({ "note": "buy milk" })).unwrap();
    assert!(matches!(&nodes[1], ViewNode::Text { text, .. } if text == "buy milk"));
}

#[test]
fn notify_requires_permission_and_reaches_sink() {
    let f = fixture();
    let with = manifest(
        "com.test.notify",
        &["commands.slash", "notifications"],
        json!({ "commands": [{ "name": "ping", "description": "p" }] }),
    );
    load(&f, with, r#"dz.plugin({ commands: { ping: { run(){ dz.notify("Hi", "there"); return {}; } } } });"#);
    f.host.run_command("com.test.notify", "ping", "").unwrap();
    assert_eq!(f.sink.notifications.lock().unwrap().len(), 1);

    let without = manifest(
        "com.test.silent",
        &["commands.slash"],
        json!({ "commands": [{ "name": "ping", "description": "p" }] }),
    );
    load(&f, without, r#"dz.plugin({ commands: { ping: { run(){ dz.notify("Hi", "there"); return {}; } } } });"#);
    assert!(f.host.run_command("com.test.silent", "ping", "").is_err());
    assert_eq!(f.sink.notifications.lock().unwrap().len(), 1);
}

#[test]
fn sandbox_exposes_no_io_globals() {
    let f = fixture();
    let src = r#"dz.plugin({ commands: { shrug: { run(){
        const found = ["require","process","std","os","fetch","XMLHttpRequest","WebSocket","__dz_host","Deno","Bun"]
            .filter(n => typeof globalThis[n] !== "undefined");
        return { replaceWith: found.join(",") };
    } } } });"#;
    load(&f, shrug_manifest("com.test.globals"), src);
    let out = f.host.run_command("com.test.globals", "shrug", "").unwrap();
    assert_eq!(out.replace_with.as_deref(), Some(""));
}

#[test]
fn idle_runtimes_are_unloaded_and_transparently_reloaded() {
    let f = fixture();
    load(&f, shrug_manifest("com.test.idle"), SHRUG);
    f.host.run_command("com.test.idle", "shrug", "").unwrap();
    f.host.set_idle_unload(Some(Duration::from_millis(50)));
    std::thread::sleep(Duration::from_millis(400));
    f.host.flush();
    let s = f.host.stats().into_iter().find(|s| s.id == "com.test.idle").unwrap();
    assert_eq!(s.state, RuntimeState::Dormant);
    assert!(f.host.run_command("com.test.idle", "shrug", "").is_ok());
}

#[test]
fn reset_clears_a_faulted_plugin() {
    let f = fixture();
    let src = r#"dz.plugin({ commands: { shrug: { run(){ throw new Error("boom"); } } } });"#;
    load(&f, shrug_manifest("com.test.reset"), src);
    for _ in 0..3 {
        let _ = f.host.run_command("com.test.reset", "shrug", "");
    }
    f.host.reset("com.test.reset").unwrap();
    let s = f.host.stats().into_iter().find(|s| s.id == "com.test.reset").unwrap();
    assert_eq!(s.state, RuntimeState::Dormant);
}
