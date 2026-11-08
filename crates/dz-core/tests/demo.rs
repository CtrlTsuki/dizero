use dz_core::CoreError;
use dz_core::db::Database;
use dz_core::demo::{self, DemoProvider, NOVA_ID};
use dz_core::integrations::Provider;
use dz_core::store;

fn seeded() -> Database {
    let db = Database::open_in_memory().unwrap();
    demo::seed(&db, "Ada").unwrap();
    db
}

fn first_conversation(db: &Database) -> String {
    let snap = store::workspace_snapshot(db).unwrap();
    snap.conversations.iter().find(|c| c.name == "general").expect("#general exists").id.clone()
}

#[test]
fn seeding_is_idempotent() {
    let db = seeded();
    let before = store::workspace_snapshot(&db).unwrap();
    demo::seed(&db, "Ada").unwrap();
    let after = store::workspace_snapshot(&db).unwrap();
    assert_eq!(before.communities.len(), after.communities.len());
    assert_eq!(before.conversations.len(), after.conversations.len());
    assert!(before.communities.len() >= 3);
}

#[test]
fn snapshot_includes_self_user_and_unread_counts() {
    let db = seeded();
    let snap = store::workspace_snapshot(&db).unwrap();
    assert!(snap.self_user.is_self);
    assert_eq!(snap.self_user.display_name, "Ada");
    assert!(snap.conversations.iter().any(|c| c.unread_count > 0), "demo should start with unread activity");
    assert!(snap.integrations.iter().any(|i| i.id == demo::DEMO_INTEGRATION_ID));
}

#[test]
fn sending_persists_a_message_from_self() {
    let db = seeded();
    let conv = first_conversation(&db);
    let provider = DemoProvider;
    let out = provider.send_message(&db, &conv, "  hello world  ").unwrap();
    assert_eq!(out.message.content, "hello world");
    assert!(out.message.author.is_self);
    let page = store::list_messages(&db, &conv, None, 200).unwrap();
    assert_eq!(page.messages.last().unwrap().id, out.message.id);
}

#[test]
fn rejects_empty_and_oversized_messages() {
    let db = seeded();
    let conv = first_conversation(&db);
    let provider = DemoProvider;
    assert!(matches!(provider.send_message(&db, &conv, "   "), Err(CoreError::Invalid(_))));
    let huge = "a".repeat(4001);
    assert!(matches!(provider.send_message(&db, &conv, &huge), Err(CoreError::Invalid(_))));
}

#[test]
fn sending_to_unknown_conversation_fails() {
    let db = seeded();
    assert!(matches!(DemoProvider.send_message(&db, "nope", "hi"), Err(CoreError::NotFound(_))));
}

#[test]
fn messages_paginate_oldest_first() {
    let db = seeded();
    let conv = first_conversation(&db);
    for i in 0..30 {
        DemoProvider.send_message(&db, &conv, &format!("msg {i}")).unwrap();
    }
    let page = store::list_messages(&db, &conv, None, 10).unwrap();
    assert_eq!(page.messages.len(), 10);
    assert!(page.has_more);
    assert_eq!(page.messages.last().unwrap().content, "msg 29");
    assert!(page.messages.windows(2).all(|w| (w[0].created_at, &w[0].id) <= (w[1].created_at, &w[1].id)));
    let first = &page.messages[0];
    let older = store::list_messages(&db, &conv, Some((first.created_at, first.id.clone())), 10).unwrap();
    assert_eq!(older.messages.last().unwrap().content, "msg 19");
}

#[test]
fn nova_replies_when_mentioned() {
    let db = seeded();
    let conv = first_conversation(&db);
    let out = DemoProvider.send_message(&db, &conv, "@Nova what can plugins do?").unwrap();
    let reply = out.reply.expect("Nova should plan a reply");
    assert!(reply.delay_ms > 0);
    let msg = demo::deliver_reply(&db, &conv, &reply).unwrap();
    assert_eq!(msg.author.id, NOVA_ID);
    assert!(msg.author.is_bot);
    assert!(msg.content.to_lowercase().contains("plugin"));
}

#[test]
fn no_reply_without_mention() {
    let db = seeded();
    let conv = first_conversation(&db);
    assert!(DemoProvider.send_message(&db, &conv, "just chatting").unwrap().reply.is_none());
}

#[test]
fn search_finds_messages_and_marks_matches() {
    let db = seeded();
    let conv = first_conversation(&db);
    DemoProvider.send_message(&db, &conv, "the quantum flux capacitor is warm").unwrap();
    let hits = store::search(&db, "flux capac", 20).unwrap();
    assert_eq!(hits.len(), 1);
    assert!(hits[0].snippet.contains('\u{E000}'));
    assert_eq!(hits[0].conversation_name, "general");
}

#[test]
fn search_tolerates_fts_syntax_in_queries() {
    let db = seeded();
    for q in ["\"", "AND OR NOT", "(((", "a*b:c", "NEAR(x y)", "", "   "] {
        assert!(store::search(&db, q, 10).is_ok(), "query {q:?} must not error");
    }
}

#[test]
fn can_delete_own_message_but_not_others() {
    let db = seeded();
    let conv = first_conversation(&db);
    let mine = DemoProvider.send_message(&db, &conv, "oops").unwrap().message;
    DemoProvider.delete_message(&db, &mine.id).unwrap();
    let page = store::list_messages(&db, &conv, None, 200).unwrap();
    assert!(!page.messages.iter().any(|m| m.id == mine.id));
    let other = page.messages.iter().find(|m| !m.author.is_self).unwrap();
    assert!(matches!(DemoProvider.delete_message(&db, &other.id), Err(CoreError::PermissionDenied(_))));
}

#[test]
fn mark_read_clears_unread() {
    let db = seeded();
    let snap = store::workspace_snapshot(&db).unwrap();
    let unread = snap.conversations.iter().find(|c| c.unread_count > 0).unwrap();
    store::mark_read(&db, &unread.id).unwrap();
    let snap = store::workspace_snapshot(&db).unwrap();
    let c = snap.conversations.iter().find(|c| c.id == unread.id).unwrap();
    assert_eq!(c.unread_count, 0);
    assert_eq!(c.mention_count, 0);
}

#[test]
fn prune_keeps_only_newest_messages() {
    let db = seeded();
    let conv = first_conversation(&db);
    for i in 0..20 {
        DemoProvider.send_message(&db, &conv, &format!("n{i}")).unwrap();
    }
    store::prune(&db, 5).unwrap();
    let page = store::list_messages(&db, &conv, None, 100).unwrap();
    assert_eq!(page.messages.len(), 5);
    assert_eq!(page.messages.last().unwrap().content, "n19");
}

#[test]
fn reset_restores_the_seed() {
    let db = seeded();
    let conv = first_conversation(&db);
    DemoProvider.send_message(&db, &conv, "temporary").unwrap();
    demo::reset(&db, "Ada").unwrap();
    assert!(store::search(&db, "temporary", 10).unwrap().is_empty());
    assert!(store::workspace_snapshot(&db).unwrap().communities.len() >= 3);
}

#[test]
fn members_are_listed_per_community() {
    let db = seeded();
    let snap = store::workspace_snapshot(&db).unwrap();
    let members = store::members(&db, &snap.communities[0].id).unwrap();
    assert!(members.len() >= 4);
    assert!(members.iter().any(|m| m.user.id == NOVA_ID));
}

#[test]
fn renaming_self_updates_the_profile() {
    let db = seeded();
    demo::set_self_name(&db, "Grace").unwrap();
    assert_eq!(store::workspace_snapshot(&db).unwrap().self_user.display_name, "Grace");
}
