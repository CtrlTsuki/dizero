//! SQLite persistence: connection management, versioned migrations,
//! integrity checking and recovery.
//!
//! One connection guarded by a mutex. DiZero's write volume is tiny (a chat
//! client's local cache), so a pool would add memory and complexity without
//! measurable benefit. WAL mode keeps reads cheap and crash-safe.

use crate::error::{CoreError, Result};
use crate::util::now_ms;
use rusqlite::{Connection, OpenFlags, Transaction};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub const SCHEMA_VERSION: i64 = 1;

/// Ordered migrations. Index `i` upgrades the schema from version `i` to `i + 1`.
/// Never edit a shipped migration; append a new one instead.
const MIGRATIONS: &[&str] = &[
    // v1 — initial schema
    r#"
    CREATE TABLE app_settings (
        key        TEXT PRIMARY KEY,
        value      TEXT NOT NULL,
        updated_at INTEGER NOT NULL
    );

    CREATE TABLE integrations (
        id         TEXT PRIMARY KEY,
        provider   TEXT NOT NULL,
        label      TEXT NOT NULL,
        enabled    INTEGER NOT NULL DEFAULT 1,
        config     TEXT NOT NULL DEFAULT '{}',
        created_at INTEGER NOT NULL
    );

    CREATE TABLE communities (
        id             TEXT PRIMARY KEY,
        integration_id TEXT NOT NULL REFERENCES integrations(id) ON DELETE CASCADE,
        name           TEXT NOT NULL,
        initials       TEXT NOT NULL,
        accent         TEXT NOT NULL,
        description    TEXT,
        position       INTEGER NOT NULL DEFAULT 0
    );

    CREATE TABLE conversations (
        id           TEXT PRIMARY KEY,
        community_id TEXT NOT NULL REFERENCES communities(id) ON DELETE CASCADE,
        name         TEXT NOT NULL,
        kind         TEXT NOT NULL,
        category     TEXT,
        topic        TEXT,
        position     INTEGER NOT NULL DEFAULT 0,
        last_read_at INTEGER NOT NULL DEFAULT 0
    );
    CREATE INDEX idx_conversations_community ON conversations(community_id, position);

    CREATE TABLE users (
        id             TEXT PRIMARY KEY,
        integration_id TEXT NOT NULL REFERENCES integrations(id) ON DELETE CASCADE,
        display_name   TEXT NOT NULL,
        username       TEXT NOT NULL,
        avatar_color   TEXT NOT NULL,
        presence       TEXT NOT NULL DEFAULT 'offline',
        is_bot         INTEGER NOT NULL DEFAULT 0,
        is_self        INTEGER NOT NULL DEFAULT 0
    );

    CREATE TABLE community_members (
        community_id TEXT NOT NULL REFERENCES communities(id) ON DELETE CASCADE,
        user_id      TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
        role         TEXT,
        PRIMARY KEY (community_id, user_id)
    );

    CREATE TABLE messages (
        id              TEXT PRIMARY KEY,
        conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
        author_id       TEXT NOT NULL,
        content         TEXT NOT NULL,
        created_at      INTEGER NOT NULL,
        edited_at       INTEGER,
        attachments     TEXT NOT NULL DEFAULT '[]',
        mentions_self   INTEGER NOT NULL DEFAULT 0
    );
    CREATE INDEX idx_messages_conversation_time ON messages(conversation_id, created_at, id);

    -- Full-text search index over message content (external-content FTS5).
    CREATE VIRTUAL TABLE message_fts USING fts5(
        content,
        content = 'messages',
        content_rowid = 'rowid',
        tokenize = 'unicode61 remove_diacritics 2'
    );
    CREATE TRIGGER messages_ai AFTER INSERT ON messages BEGIN
        INSERT INTO message_fts(rowid, content) VALUES (new.rowid, new.content);
    END;
    CREATE TRIGGER messages_ad AFTER DELETE ON messages BEGIN
        INSERT INTO message_fts(message_fts, rowid, content) VALUES ('delete', old.rowid, old.content);
    END;
    CREATE TRIGGER messages_au AFTER UPDATE OF content ON messages BEGIN
        INSERT INTO message_fts(message_fts, rowid, content) VALUES ('delete', old.rowid, old.content);
        INSERT INTO message_fts(rowid, content) VALUES (new.rowid, new.content);
    END;

    CREATE TABLE plugin_registry (
        id           TEXT PRIMARY KEY,
        version      TEXT NOT NULL,
        source       TEXT NOT NULL,          -- 'bundled' | 'local'
        install_path TEXT NOT NULL,
        manifest     TEXT NOT NULL,          -- validated manifest JSON
        integrity    TEXT NOT NULL,          -- sha256 over manifest + code
        enabled      INTEGER NOT NULL DEFAULT 0,
        state        TEXT NOT NULL DEFAULT 'disabled',
        last_error   TEXT,
        installed_at INTEGER NOT NULL,
        updated_at   INTEGER NOT NULL
    );

    CREATE TABLE plugin_permissions (
        plugin_id  TEXT NOT NULL REFERENCES plugin_registry(id) ON DELETE CASCADE,
        permission TEXT NOT NULL,
        granted    INTEGER NOT NULL DEFAULT 0,
        PRIMARY KEY (plugin_id, permission)
    );

    CREATE TABLE plugin_storage (
        plugin_id TEXT NOT NULL,
        key       TEXT NOT NULL,
        value     TEXT NOT NULL,
        PRIMARY KEY (plugin_id, key)
    );
    "#,
];

pub struct Database {
    conn: Mutex<Connection>,
}

pub struct OpenOutcome {
    pub db: Database,
    /// Set when the existing file was unreadable and was moved aside.
    pub recovered_from: Option<PathBuf>,
}

fn configure(conn: &Connection, file_backed: bool) -> Result<()> {
    if file_backed {
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
    }
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "temp_store", "MEMORY")?;
    // Bound the page cache (negative = KiB): 2 MiB is plenty for this workload.
    conn.pragma_update(None, "cache_size", -2048)?;
    conn.busy_timeout(std::time::Duration::from_secs(2))?;
    Ok(())
}

fn migrate(conn: &mut Connection) -> Result<()> {
    let current: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if current > SCHEMA_VERSION {
        return Err(CoreError::Unsupported(format!(
            "database schema v{current} is newer than this build (v{SCHEMA_VERSION})"
        )));
    }
    for (idx, sql) in MIGRATIONS.iter().enumerate().skip(current as usize) {
        let next = idx as i64 + 1;
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", next)?;
        tx.commit()?;
        log::info!("database migrated to schema v{next}");
    }
    Ok(())
}

/// Open + verify a file database. Returns an error if the file is not a
/// usable SQLite database so the caller can quarantine it.
fn open_verified(path: &Path) -> Result<Connection> {
    let mut conn = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    configure(&conn, true)?;
    let check: String = conn.query_row("PRAGMA quick_check", [], |r| r.get(0))?;
    if check != "ok" {
        return Err(CoreError::invalid(format!("integrity check failed: {check}")));
    }
    migrate(&mut conn)?;
    Ok(conn)
}

impl Database {
    /// Open (or create) the database at `path`. If the file is corrupt it is
    /// renamed to `<name>.corrupt-<millis>` and a fresh database is created,
    /// so a damaged cache can never prevent the app from starting.
    pub fn open(path: &Path) -> Result<OpenOutcome> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        match open_verified(path) {
            Ok(conn) => Ok(OpenOutcome { db: Database { conn: Mutex::new(conn) }, recovered_from: None }),
            Err(CoreError::Unsupported(m)) => Err(CoreError::Unsupported(m)),
            Err(first_err) => {
                log::error!("database at {} unusable ({first_err}); quarantining", path.display());
                let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("dizero.db");
                let quarantine = path.with_file_name(format!("{file_name}.corrupt-{}", now_ms()));
                std::fs::rename(path, &quarantine)?;
                for suffix in ["-wal", "-shm"] {
                    let side = path.with_file_name(format!("{file_name}{suffix}"));
                    if side.exists() {
                        let _ = std::fs::rename(
                            &side,
                            quarantine.with_file_name(format!(
                                "{}{suffix}",
                                quarantine.file_name().and_then(|n| n.to_str()).unwrap_or_default()
                            )),
                        );
                    }
                }
                let conn = open_verified(path)?;
                Ok(OpenOutcome { db: Database { conn: Mutex::new(conn) }, recovered_from: Some(quarantine) })
            }
        }
    }

    pub fn open_in_memory() -> Result<Database> {
        let mut conn = Connection::open_in_memory()?;
        configure(&conn, false)?;
        migrate(&mut conn)?;
        Ok(Database { conn: Mutex::new(conn) })
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Connection> {
        // A poisoned lock means a panic happened mid-call; SQLite itself is
        // still consistent (statements are atomic), so keep serving.
        self.conn.lock().unwrap_or_else(|p| p.into_inner())
    }

    pub fn schema_version(&self) -> Result<i64> {
        Ok(self.lock().query_row("PRAGMA user_version", [], |r| r.get(0))?)
    }

    pub fn with<R>(&self, f: impl FnOnce(&Connection) -> Result<R>) -> Result<R> {
        f(&self.lock())
    }

    pub fn with_tx<R>(&self, f: impl FnOnce(&Transaction) -> Result<R>) -> Result<R> {
        let mut conn = self.lock();
        let tx = conn.transaction()?;
        let out = f(&tx)?;
        tx.commit()?;
        Ok(out)
    }

    pub fn integrity_ok(&self) -> Result<bool> {
        let r: String = self.lock().query_row("PRAGMA quick_check", [], |r| r.get(0))?;
        Ok(r == "ok")
    }

    pub fn size_bytes(&self) -> Result<u64> {
        let conn = self.lock();
        let pages: i64 = conn.query_row("PRAGMA page_count", [], |r| r.get(0))?;
        let size: i64 = conn.query_row("PRAGMA page_size", [], |r| r.get(0))?;
        Ok((pages * size).max(0) as u64)
    }

    pub fn vacuum(&self) -> Result<()> {
        self.lock().execute_batch("VACUUM")?;
        Ok(())
    }

    /// Consistent online backup via `VACUUM INTO`.
    pub fn backup_to(&self, dest: &Path) -> Result<()> {
        if dest.exists() {
            return Err(CoreError::invalid("backup destination already exists"));
        }
        let dest_str = dest.to_str().ok_or_else(|| CoreError::invalid("backup path is not valid UTF-8"))?;
        self.lock().execute("VACUUM INTO ?1", [dest_str])?;
        Ok(())
    }

    /// Flush the WAL into the main file. Called on graceful shutdown.
    pub fn checkpoint(&self) {
        if let Err(e) = self.lock().execute_batch("PRAGMA wal_checkpoint(TRUNCATE)") {
            log::warn!("wal checkpoint failed: {e}");
        }
    }
}
