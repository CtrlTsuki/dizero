use dz_core::plugins::view::{MAX_NODES, MAX_TEXT, ViewNode, sanitize};
use serde_json::json;

#[test]
fn keeps_known_nodes() {
    let out = sanitize(&json!([
        { "type": "heading", "text": "Hello" },
        { "type": "stat", "label": "Messages", "value": 42 },
        { "type": "button", "action": "reset", "label": "Reset", "variant": "danger" },
        { "type": "input", "id": "note", "placeholder": "Write…" },
        { "type": "divider" }
    ]));
    assert_eq!(out.nodes.len(), 5);
    assert_eq!(out.dropped, 0);
    assert!(matches!(&out.nodes[1], ViewNode::Stat { value, .. } if value == "42"));
}

#[test]
fn drops_unknown_and_malformed_nodes() {
    let out = sanitize(&json!([
        { "type": "script", "src": "x" },
        { "type": "html", "html": "<img onerror=alert(1)>" },
        { "type": "button", "label": "no action" },
        "a bare string",
        { "type": "text", "text": "ok" }
    ]));
    assert_eq!(out.nodes.len(), 1);
    assert_eq!(out.dropped, 4);
}

#[test]
fn truncates_long_text() {
    let long = "x".repeat(MAX_TEXT * 2);
    let out = sanitize(&json!([{ "type": "text", "text": long }]));
    match &out.nodes[0] {
        ViewNode::Text { text, .. } => assert_eq!(text.chars().count(), MAX_TEXT),
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn caps_node_count() {
    let many: Vec<_> = (0..MAX_NODES + 50).map(|i| json!({ "type": "text", "text": i.to_string() })).collect();
    let out = sanitize(&serde_json::Value::Array(many));
    assert_eq!(out.nodes.len(), MAX_NODES);
    assert_eq!(out.dropped, 50);
}

#[test]
fn non_array_input_yields_nothing() {
    let out = sanitize(&json!({ "type": "text", "text": "x" }));
    assert!(out.nodes.is_empty());
}

#[test]
fn unknown_tones_and_variants_fall_back_to_defaults() {
    let out = sanitize(&json!([
        { "type": "text", "text": "a", "tone": "blink" },
        { "type": "button", "action": "a", "label": "b", "variant": "rainbow" }
    ]));
    assert_eq!(out.nodes.len(), 2);
}

#[test]
fn list_items_are_sanitized_and_capped() {
    let items: Vec<_> = (0..500).map(|i| json!({ "id": i.to_string(), "text": "item" })).collect();
    let out = sanitize(&json!([{ "type": "list", "items": items }]));
    match &out.nodes[0] {
        ViewNode::List { items } => assert!(items.len() <= 100),
        other => panic!("unexpected {other:?}"),
    }
}
