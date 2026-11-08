use dz_core::db::{Database, SCHEMA_VERSION};
use std::path::PathBuf;

fn temp_dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("dz-test-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn fresh_database_is_migrated_to_current_schema() {
    let db = Database::open_in_memory().unwrap();
    assert_eq!(db.schema_version().unwrap(), SCHEMA_VERSION);
}

#[test]
fn reopening_a_database_keeps_data_and_version() {
    let dir = temp_dir("reopen");
    let path = dir.join("dizero.db");
    {
        let out = Database::open(&path).unwrap();
        assert!(out.recovered_from.is_none());
        out.db
            .with(|c| {
                c.execute("INSERT INTO app_settings(key, value, updated_at) VALUES ('probe', '1', 0)", [])?;
                Ok(())
            })
            .unwrap();
    }
    let out = Database::open(&path).unwrap();
    assert_eq!(out.db.schema_version().unwrap(), SCHEMA_VERSION);
    let v: String = out
        .db
        .with(|c| Ok(c.query_row("SELECT value FROM app_settings WHERE key='probe'", [], |r| r.get(0))?))
        .unwrap();
    assert_eq!(v, "1");
}

#[test]
fn corrupt_database_file_is_quarantined_and_replaced() {
    let dir = temp_dir("corrupt");
    let path = dir.join("dizero.db");
    std::fs::write(&path, b"this is definitely not a sqlite database, just garbage bytes......").unwrap();
    let out = Database::open(&path).unwrap();
    let quarantined = out.recovered_from.expect("corruption should be reported");
    assert!(quarantined.exists(), "corrupt file must be preserved for diagnosis");
    assert_eq!(out.db.schema_version().unwrap(), SCHEMA_VERSION);
    assert!(out.db.integrity_ok().unwrap());
}

#[test]
fn backup_produces_an_openable_copy() {
    let dir = temp_dir("backup");
    let out = Database::open(&dir.join("dizero.db")).unwrap();
    let dest = dir.join("backup.db");
    out.db.backup_to(&dest).unwrap();
    let copy = Database::open(&dest).unwrap();
    assert_eq!(copy.db.schema_version().unwrap(), SCHEMA_VERSION);
}

#[test]
fn reports_size_and_vacuums() {
    let db = Database::open_in_memory().unwrap();
    assert!(db.size_bytes().unwrap() > 0);
    db.vacuum().unwrap();
}
