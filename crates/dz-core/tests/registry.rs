use dz_core::CoreError;
use dz_core::db::Database;
use dz_core::plugins::host::PluginStorage;
use dz_core::plugins::permissions::Permission;
use dz_core::plugins::registry::{DbPluginStorage, PluginRegistry, PluginSource, RegistryState};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

static N: AtomicU32 = AtomicU32::new(0);

fn temp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "dz-reg-{name}-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn write_plugin(dir: &Path, id: &str, version: &str, perms: &[&str], extra: serde_json::Value) {
    std::fs::create_dir_all(dir).unwrap();
    let mut m = json!({
        "id": id, "name": "Sample", "version": version, "description": "sample", "author": "me",
        "apiVersion": "1", "kind": "script", "category": "productivity", "entry": "index.js",
        "permissions": perms,
        "contributes": { "commands": [{ "name": "hi", "description": "say hi" }] }
    });
    if let Some(obj) = extra.as_object() {
        for (k, v) in obj {
            m[k] = v.clone();
        }
    }
    std::fs::write(dir.join("dizero-plugin.json"), serde_json::to_string_pretty(&m).unwrap()).unwrap();
    std::fs::write(
        dir.join("index.js"),
        r#"dz.plugin({ commands: { hi: { run(){ return { replaceWith: "hi" }; } } } });"#,
    )
    .unwrap();
}

struct Env {
    registry: PluginRegistry,
    db: Arc<Database>,
    src: PathBuf,
}

fn env(name: &str) -> Env {
    let db = Arc::new(Database::open_in_memory().unwrap());
    let root = temp(&format!("{name}-root"));
    let src = temp(&format!("{name}-src"));
    Env { registry: PluginRegistry::new(db.clone(), root), db, src }
}

#[test]
fn install_copies_files_and_starts_disabled_with_nothing_granted() {
    let e = env("install");
    let dir = e.src.join("p");
    write_plugin(&dir, "com.test.sample", "1.0.0", &["commands.slash", "storage.local"], json!({}));
    let p = e.registry.install_from_dir(&dir, PluginSource::Local).unwrap();
    assert_eq!(p.manifest.id, "com.test.sample");
    assert!(!p.enabled);
    assert_eq!(p.state, RegistryState::Disabled);
    assert!(p.permissions.iter().all(|g| !g.granted));
    assert_eq!(p.integrity.len(), 64);
    assert!(Path::new(&p.install_path).join("index.js").exists());
    assert!(!Path::new(&p.install_path).starts_with(&dir));
}

#[test]
fn install_rejects_invalid_manifest_with_issues() {
    let e = env("invalid");
    let dir = e.src.join("bad");
    write_plugin(&dir, "BAD ID", "1.0.0", &[], json!({}));
    match e.registry.install_from_dir(&dir, PluginSource::Local) {
        Err(CoreError::Invalid(msg)) => assert!(msg.contains("id"), "{msg}"),
        other => panic!("expected invalid, got {:?}", other.map(|p| p.manifest.id)),
    }
}

#[test]
fn install_rejects_missing_entry_file() {
    let e = env("noentry");
    let dir = e.src.join("p");
    write_plugin(&dir, "com.test.noentry", "1.0.0", &["commands.slash"], json!({}));
    std::fs::remove_file(dir.join("index.js")).unwrap();
    assert!(e.registry.install_from_dir(&dir, PluginSource::Local).is_err());
}

#[test]
fn install_rejects_oversized_files_and_disallowed_types() {
    let e = env("big");
    let dir = e.src.join("p");
    write_plugin(&dir, "com.test.big", "1.0.0", &["commands.slash"], json!({}));
    std::fs::write(dir.join("payload.exe"), b"MZ").unwrap();
    assert!(e.registry.install_from_dir(&dir, PluginSource::Local).is_err());
    std::fs::remove_file(dir.join("payload.exe")).unwrap();
    std::fs::write(dir.join("huge.js"), vec![b'a'; 3 * 1024 * 1024]).unwrap();
    assert!(e.registry.install_from_dir(&dir, PluginSource::Local).is_err());
}

#[test]
fn approving_grants_only_requested_permissions_and_enables() {
    let e = env("approve");
    let dir = e.src.join("p");
    write_plugin(&dir, "com.test.ok", "1.0.0", &["commands.slash", "storage.local"], json!({}));
    e.registry.install_from_dir(&dir, PluginSource::Local).unwrap();
    // Granting something not requested is refused.
    assert!(e.registry.approve_and_enable("com.test.ok", &[Permission::Notifications]).is_err());
    let p = e.registry.approve_and_enable("com.test.ok", &[Permission::CommandsSlash]).unwrap();
    assert!(p.enabled);
    let granted: Vec<_> = p.permissions.iter().filter(|g| g.granted).map(|g| g.permission).collect();
    assert_eq!(granted, vec![Permission::CommandsSlash]);
    let (reqs, problems) = e.registry.load_requests().unwrap();
    assert!(problems.is_empty());
    assert_eq!(reqs.len(), 1);
    assert!(reqs[0].granted.contains(&Permission::CommandsSlash));
    assert!(!reqs[0].granted.contains(&Permission::StorageLocal));
}

#[test]
fn permissions_can_be_revoked_later() {
    let e = env("revoke");
    let dir = e.src.join("p");
    write_plugin(&dir, "com.test.rev", "1.0.0", &["commands.slash", "storage.local"], json!({}));
    e.registry.install_from_dir(&dir, PluginSource::Local).unwrap();
    e.registry.approve_and_enable("com.test.rev", &[Permission::CommandsSlash, Permission::StorageLocal]).unwrap();
    e.registry.set_permission("com.test.rev", Permission::StorageLocal, false).unwrap();
    let (reqs, _) = e.registry.load_requests().unwrap();
    assert!(!reqs[0].granted.contains(&Permission::StorageLocal));
    assert!(e.registry.set_permission("com.test.rev", Permission::Notifications, true).is_err());
}

#[test]
fn tampering_is_detected_by_integrity_check() {
    let e = env("tamper");
    let dir = e.src.join("p");
    write_plugin(&dir, "com.test.tamper", "1.0.0", &["commands.slash"], json!({}));
    let p = e.registry.install_from_dir(&dir, PluginSource::Local).unwrap();
    e.registry.approve_and_enable("com.test.tamper", &[Permission::CommandsSlash]).unwrap();
    assert!(e.registry.verify_integrity("com.test.tamper").unwrap());
    std::fs::write(Path::new(&p.install_path).join("index.js"), "/* evil */").unwrap();
    assert!(!e.registry.verify_integrity("com.test.tamper").unwrap());
    let (reqs, problems) = e.registry.load_requests().unwrap();
    assert!(reqs.is_empty());
    assert_eq!(problems.len(), 1);
    assert_eq!(e.registry.get("com.test.tamper").unwrap().state, RegistryState::IntegrityFailed);
}

#[test]
fn uninstall_removes_files_rows_and_storage() {
    let e = env("uninstall");
    let dir = e.src.join("p");
    write_plugin(&dir, "com.test.gone", "1.0.0", &["commands.slash"], json!({}));
    let p = e.registry.install_from_dir(&dir, PluginSource::Local).unwrap();
    let storage = DbPluginStorage::new(e.db.clone());
    storage.set("com.test.gone", "k", "\"v\"").unwrap();
    e.registry.uninstall("com.test.gone").unwrap();
    assert!(!Path::new(&p.install_path).exists());
    assert!(e.registry.get("com.test.gone").is_err());
    assert!(storage.get("com.test.gone", "k").unwrap().is_none());
}

#[test]
fn enabling_requires_dependencies() {
    let e = env("deps");
    let dir = e.src.join("p");
    write_plugin(
        &dir,
        "com.test.child",
        "1.0.0",
        &["commands.slash"],
        json!({ "dependencies": { "com.test.parent": "^1.0" } }),
    );
    e.registry.install_from_dir(&dir, PluginSource::Local).unwrap();
    let err = e.registry.approve_and_enable("com.test.child", &[Permission::CommandsSlash]).unwrap_err();
    assert!(err.to_string().contains("com.test.parent"), "{err}");

    let parent = e.src.join("parent");
    write_plugin(&parent, "com.test.parent", "1.4.0", &["commands.slash"], json!({}));
    e.registry.install_from_dir(&parent, PluginSource::Local).unwrap();
    e.registry.approve_and_enable("com.test.parent", &[Permission::CommandsSlash]).unwrap();
    assert!(e.registry.approve_and_enable("com.test.child", &[Permission::CommandsSlash]).is_ok());
}

#[test]
fn upgrade_keeps_grants_requires_review_for_new_permissions_and_can_roll_back() {
    let e = env("upgrade");
    let v1 = e.src.join("v1");
    write_plugin(&v1, "com.test.up", "1.0.0", &["commands.slash"], json!({}));
    e.registry.install_from_dir(&v1, PluginSource::Local).unwrap();
    e.registry.approve_and_enable("com.test.up", &[Permission::CommandsSlash]).unwrap();

    let v2 = e.src.join("v2");
    write_plugin(&v2, "com.test.up", "2.0.0", &["commands.slash", "notifications"], json!({}));
    let p = e.registry.install_from_dir(&v2, PluginSource::Local).unwrap();
    assert_eq!(p.manifest.version, "2.0.0");
    assert!(!p.enabled, "new permissions require a fresh review");
    assert!(p.permissions.iter().any(|g| g.permission == Permission::CommandsSlash && g.granted));
    assert!(p.permissions.iter().any(|g| g.permission == Permission::Notifications && !g.granted));
    assert!(p.has_backup);

    let back = e.registry.rollback("com.test.up").unwrap();
    assert_eq!(back.manifest.version, "1.0.0");
    assert!(e.registry.verify_integrity("com.test.up").unwrap());
}

#[test]
fn downgrade_via_install_is_refused() {
    let e = env("downgrade");
    let v2 = e.src.join("v2");
    write_plugin(&v2, "com.test.down", "2.0.0", &["commands.slash"], json!({}));
    e.registry.install_from_dir(&v2, PluginSource::Local).unwrap();
    let v1 = e.src.join("v1");
    write_plugin(&v1, "com.test.down", "1.0.0", &["commands.slash"], json!({}));
    assert!(e.registry.install_from_dir(&v1, PluginSource::Local).is_err());
}

#[test]
fn catalog_scan_reports_valid_and_invalid_folders() {
    let e = env("catalog");
    write_plugin(&e.src.join("good"), "com.test.good", "1.0.0", &["commands.slash"], json!({}));
    write_plugin(&e.src.join("bad"), "nope", "x", &[], json!({}));
    let entries = PluginRegistry::scan_catalog(&e.src);
    assert_eq!(entries.len(), 2);
    assert!(entries.iter().any(|c| c.manifest.as_ref().is_some_and(|m| m.id == "com.test.good")));
    assert!(entries.iter().any(|c| c.manifest.is_none() && !c.issues.is_empty()));
}

#[test]
fn db_storage_tracks_usage() {
    let db = Arc::new(Database::open_in_memory().unwrap());
    let s = DbPluginStorage::new(db);
    s.set("p", "a", "12345").unwrap();
    s.set("p", "b", "1").unwrap();
    s.set("other", "a", "xxxxxxxx").unwrap();
    assert_eq!(s.usage_bytes("p").unwrap(), 1 + 5 + 1 + 1);
    assert_eq!(s.keys("p").unwrap(), vec!["a".to_string(), "b".to_string()]);
    s.delete("p", "a").unwrap();
    assert_eq!(s.get("p", "a").unwrap(), None);
}
