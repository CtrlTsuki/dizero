//! Provider-agnostic queries over the local communication cache.

use crate::db::Database;
use crate::error::{CoreError, Result};
use crate::models::{
    Attachment, Community, Conversation, ConversationKind, IntegrationInfo, Member, Message, MessagePage, Presence,
    SearchHit, User, WorkspaceSnapshot,
};
use rusqlite::{Connection, OptionalExtension, Row, params};

pub const MAX_MESSAGE_CHARS: usize = 4000;

fn user_from_row(row: &Row<'_>, offset: usize) -> rusqlite::Result<User> {
    Ok(User {
        id: row.get(offset)?,
        display_name: row.get::<_, Option<String>>(offset + 1)?.unwrap_or_else(|| "Unknown user".into()),
        username: row.get::<_, Option<String>>(offset + 2)?.unwrap_or_else(|| "unknown".into()),
        avatar_color: row.get::<_, Option<String>>(offset + 3)?.unwrap_or_else(|| "#6b6a78".into()),
        presence: Presence::parse(&row.get::<_, Option<String>>(offset + 4)?.unwrap_or_default()),
        is_bot: row.get::<_, Option<bool>>(offset + 5)?.unwrap_or(false),
        is_self: row.get::<_, Option<bool>>(offset + 6)?.unwrap_or(false),
    })
}

const MESSAGE_SELECT: &str =
    "SELECT m.id, m.conversation_id, m.content, m.created_at, m.edited_at, m.attachments, m.mentions_self,
        m.author_id, u.display_name, u.username, u.avatar_color, u.presence, u.is_bot, u.is_self
     FROM messages m LEFT JOIN users u ON u.id = m.author_id";

fn message_from_row(row: &Row<'_>) -> rusqlite::Result<Message> {
    let attachments_json: String = row.get(5)?;
    Ok(Message {
        id: row.get(0)?,
        conversation_id: row.get(1)?,
        content: row.get(2)?,
        created_at: row.get(3)?,
        edited_at: row.get(4)?,
        attachments: serde_json::from_str::<Vec<Attachment>>(&attachments_json).unwrap_or_default(),
        mentions_self: row.get(6)?,
        author: user_from_row(row, 7)?,
    })
}

fn integrations(conn: &Connection) -> Result<Vec<IntegrationInfo>> {
    let mut stmt = conn.prepare_cached("SELECT id FROM integrations WHERE enabled = 1 ORDER BY created_at")?;
    let ids: Vec<String> = stmt.query_map([], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?;
    let mut out = Vec::new();
    for id in ids {
        if id == crate::demo::DEMO_INTEGRATION_ID {
            use crate::integrations::Provider;
            out.push(crate::demo::DemoProvider.info());
        }
    }
    out.extend(crate::integrations::planned_integrations());
    Ok(out)
}

fn self_user_conn(conn: &Connection) -> Result<User> {
    conn.prepare_cached(
        "SELECT id, display_name, username, avatar_color, presence, is_bot, is_self FROM users WHERE is_self = 1 LIMIT 1",
    )?
    .query_row([], |r| user_from_row(r, 0))
    .optional()?
    .ok_or_else(|| CoreError::NotFound("self user (no integration seeded)".into()))
}

pub fn workspace_snapshot(db: &Database) -> Result<WorkspaceSnapshot> {
    db.with(|conn| {
        let communities = conn
            .prepare_cached(
                "SELECT id, integration_id, name, initials, accent, description, position FROM communities ORDER BY position, name",
            )?
            .query_map([], |r| {
                Ok(Community {
                    id: r.get(0)?,
                    integration_id: r.get(1)?,
                    name: r.get(2)?,
                    initials: r.get(3)?,
                    accent: r.get(4)?,
                    description: r.get(5)?,
                    position: r.get(6)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let conversations = conn
            .prepare_cached(
                "SELECT c.id, c.community_id, c.name, c.kind, c.category, c.topic, c.position,
                    (SELECT COUNT(*) FROM messages m JOIN users u ON u.id = m.author_id
                       WHERE m.conversation_id = c.id AND m.created_at > c.last_read_at AND u.is_self = 0),
                    (SELECT COUNT(*) FROM messages m
                       WHERE m.conversation_id = c.id AND m.created_at > c.last_read_at AND m.mentions_self = 1)
                 FROM conversations c JOIN communities co ON co.id = c.community_id
                 ORDER BY co.position, c.position, c.name",
            )?
            .query_map([], |r| {
                Ok(Conversation {
                    id: r.get(0)?,
                    community_id: r.get(1)?,
                    name: r.get(2)?,
                    kind: ConversationKind::parse(&r.get::<_, String>(3)?),
                    category: r.get(4)?,
                    topic: r.get(5)?,
                    position: r.get(6)?,
                    unread_count: r.get(7)?,
                    mention_count: r.get(8)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(WorkspaceSnapshot {
            integrations: integrations(conn)?,
            communities,
            conversations,
            self_user: self_user_conn(conn)?,
        })
    })
}

pub fn list_messages(
    db: &Database,
    conversation_id: &str,
    before: Option<(i64, String)>,
    limit: u32,
) -> Result<MessagePage> {
    let limit = limit.clamp(1, 500) as i64;
    db.with(|conn| {
        let mut rows: Vec<Message> = match &before {
            None => conn
                .prepare_cached(&format!(
                    "{MESSAGE_SELECT} WHERE m.conversation_id = ?1 ORDER BY m.created_at DESC, m.id DESC LIMIT ?2"
                ))?
                .query_map(params![conversation_id, limit + 1], message_from_row)?
                .collect::<rusqlite::Result<_>>()?,
            Some((ts, id)) => conn
                .prepare_cached(&format!(
                    "{MESSAGE_SELECT} WHERE m.conversation_id = ?1 AND (m.created_at < ?2 OR (m.created_at = ?2 AND m.id < ?3))
                     ORDER BY m.created_at DESC, m.id DESC LIMIT ?4"
                ))?
                .query_map(params![conversation_id, ts, id, limit + 1], message_from_row)?
                .collect::<rusqlite::Result<_>>()?,
        };
        let has_more = rows.len() as i64 > limit;
        rows.truncate(limit as usize);
        rows.reverse();
        Ok(MessagePage { messages: rows, has_more })
    })
}

pub fn get_message(db: &Database, id: &str) -> Result<Message> {
    db.with(|conn| {
        conn.prepare_cached(&format!("{MESSAGE_SELECT} WHERE m.id = ?1"))?
            .query_row([id], message_from_row)
            .optional()?
            .ok_or_else(|| CoreError::NotFound(format!("message {id}")))
    })
}

pub fn get_user(db: &Database, id: &str) -> Result<User> {
    db.with(|conn| {
        conn.prepare_cached(
            "SELECT id, display_name, username, avatar_color, presence, is_bot, is_self FROM users WHERE id = ?1",
        )?
        .query_row([id], |r| user_from_row(r, 0))
        .optional()?
        .ok_or_else(|| CoreError::NotFound(format!("user {id}")))
    })
}

pub fn self_user(db: &Database) -> Result<User> {
    db.with(self_user_conn)
}

pub struct NewMessage<'a> {
    pub conversation_id: &'a str,
    pub author_id: &'a str,
    pub content: &'a str,
    pub created_at: i64,
}

/// Validate and normalise outgoing message text.
pub fn normalize_content(content: &str) -> Result<String> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Err(CoreError::invalid("message is empty"));
    }
    if trimmed.chars().count() > MAX_MESSAGE_CHARS {
        return Err(CoreError::invalid(format!("message exceeds {MAX_MESSAGE_CHARS} characters")));
    }
    Ok(trimmed.to_string())
}

fn mentions(content: &str, self_name: &str) -> bool {
    let lower = content.to_lowercase();
    lower.contains("@everyone") || lower.contains(&format!("@{}", self_name.to_lowercase()))
}

pub fn insert_message(db: &Database, m: NewMessage<'_>) -> Result<Message> {
    let id = crate::util::new_id("msg");
    db.with(|conn| {
        let exists: bool = conn
            .prepare_cached("SELECT 1 FROM conversations WHERE id = ?1")?
            .query_row([m.conversation_id], |_| Ok(true))
            .optional()?
            .unwrap_or(false);
        if !exists {
            return Err(CoreError::NotFound(format!("conversation {}", m.conversation_id)));
        }
        let me = self_user_conn(conn)?;
        let mentions_self = m.author_id != me.id && mentions(m.content, &me.display_name);
        conn.prepare_cached(
            "INSERT INTO messages(id, conversation_id, author_id, content, created_at, attachments, mentions_self)
             VALUES (?1, ?2, ?3, ?4, ?5, '[]', ?6)",
        )?
        .execute(params![id, m.conversation_id, m.author_id, m.content, m.created_at, mentions_self])?;
        Ok(())
    })?;
    get_message(db, &id)
}

pub fn delete_message(db: &Database, id: &str) -> Result<()> {
    let n = db.with(|c| Ok(c.prepare_cached("DELETE FROM messages WHERE id = ?1")?.execute([id])?))?;
    if n == 0 {
        return Err(CoreError::NotFound(format!("message {id}")));
    }
    Ok(())
}

pub fn conversation_integration(db: &Database, conversation_id: &str) -> Result<String> {
    db.with(|c| {
        c.prepare_cached(
            "SELECT co.integration_id FROM conversations c JOIN communities co ON co.id = c.community_id WHERE c.id = ?1",
        )?
        .query_row([conversation_id], |r| r.get(0))
        .optional()?
        .ok_or_else(|| CoreError::NotFound(format!("conversation {conversation_id}")))
    })
}

pub fn mark_read(db: &Database, conversation_id: &str) -> Result<()> {
    db.with(|c| {
        // Mark read up to the newest message (or now), whichever is later.
        c.prepare_cached(
            "UPDATE conversations SET last_read_at = MAX(?2,
                COALESCE((SELECT MAX(created_at) FROM messages WHERE conversation_id = ?1), 0))
             WHERE id = ?1",
        )?
        .execute(params![conversation_id, crate::util::now_ms()])?;
        Ok(())
    })
}

pub fn members(db: &Database, community_id: &str) -> Result<Vec<Member>> {
    db.with(|c| {
        Ok(c.prepare_cached(
            "SELECT u.id, u.display_name, u.username, u.avatar_color, u.presence, u.is_bot, u.is_self, cm.role
             FROM community_members cm JOIN users u ON u.id = cm.user_id
             WHERE cm.community_id = ?1
             ORDER BY CASE u.presence WHEN 'online' THEN 0 WHEN 'idle' THEN 1 WHEN 'dnd' THEN 2 ELSE 3 END, u.display_name",
        )?
        .query_map([community_id], |r| Ok(Member { user: user_from_row(r, 0)?, role: r.get(7)? }))?
        .collect::<rusqlite::Result<Vec<_>>>()?)
    })
}

/// Turn free text into a safe FTS5 query: each word becomes a quoted prefix
/// term, so FTS operators typed by the user are treated as plain text.
pub fn fts_query(input: &str) -> Option<String> {
    let terms: Vec<String> = input
        .split_whitespace()
        .map(|w| w.chars().filter(|c| c.is_alphanumeric()).collect::<String>())
        .filter(|w| !w.is_empty())
        .take(8)
        .map(|w| format!("\"{w}\"*"))
        .collect();
    (!terms.is_empty()).then(|| terms.join(" "))
}

pub fn search(db: &Database, query: &str, limit: u32) -> Result<Vec<SearchHit>> {
    let Some(q) = fts_query(query) else { return Ok(Vec::new()) };
    let limit = limit.clamp(1, 100);
    let found: Vec<(String, String, String, String)> = db.with(|c| {
        Ok(c.prepare_cached(
            "SELECT m.id, snippet(message_fts, 0, char(57344), char(57345), '…', 18), cv.name, cv.community_id
             FROM message_fts
             JOIN messages m ON m.rowid = message_fts.rowid
             JOIN conversations cv ON cv.id = m.conversation_id
             WHERE message_fts MATCH ?1
             ORDER BY m.created_at DESC LIMIT ?2",
        )?
        .query_map(params![q, limit], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
        .collect::<rusqlite::Result<_>>()?)
    })?;
    found
        .into_iter()
        .map(|(id, snippet, conversation_name, community_id)| {
            Ok(SearchHit { message: get_message(db, &id)?, community_id, conversation_name, snippet })
        })
        .collect()
}

/// Bound the cache: keep only the newest `max_per_conversation` messages in
/// each conversation. Returns how many rows were removed.
pub fn prune(db: &Database, max_per_conversation: u32) -> Result<usize> {
    db.with(|c| {
        Ok(c.execute(
            "DELETE FROM messages WHERE rowid IN (
                SELECT rowid FROM (
                    SELECT rowid, ROW_NUMBER() OVER (PARTITION BY conversation_id ORDER BY created_at DESC, id DESC) AS rn
                    FROM messages
                ) WHERE rn > ?1
            )",
            [max_per_conversation],
        )?)
    })
}

#[derive(Debug, Clone, serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StorageStats {
    #[ts(type = "number")]
    pub database_bytes: u64,
    #[ts(type = "number")]
    pub messages: i64,
    #[ts(type = "number")]
    pub conversations: i64,
    #[ts(type = "number")]
    pub plugin_storage_bytes: i64,
}

pub fn storage_stats(db: &Database) -> Result<StorageStats> {
    let database_bytes = db.size_bytes()?;
    db.with(|c| {
        Ok(StorageStats {
            database_bytes,
            messages: c.query_row("SELECT COUNT(*) FROM messages", [], |r| r.get(0))?,
            conversations: c.query_row("SELECT COUNT(*) FROM conversations", [], |r| r.get(0))?,
            plugin_storage_bytes: c.query_row(
                "SELECT COALESCE(SUM(LENGTH(key) + LENGTH(value)), 0) FROM plugin_storage",
                [],
                |r| r.get(0),
            )?,
        })
    })
}

#[cfg(test)]
mod tests {
    use super::fts_query;

    #[test]
    fn fts_query_quotes_terms() {
        assert_eq!(fts_query("hello wor").as_deref(), Some("\"hello\"* \"wor\"*"));
        assert_eq!(fts_query("\"a\" OR b").as_deref(), Some("\"a\"* \"OR\"* \"b\"*"));
        assert_eq!(fts_query("((( )))"), None);
    }
}
