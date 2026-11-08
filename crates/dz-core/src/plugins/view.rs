//! Declarative, sanitised view model for plugin UI.
//!
//! Plugins never touch the DOM. They return plain data describing a small
//! set of node types; the host validates it and the Svelte UI renders it
//! with its own components as text (never as HTML).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub const MAX_NODES: usize = 100;
pub const MAX_TEXT: usize = 2000;
pub const MAX_LABEL: usize = 120;
pub const MAX_LIST_ITEMS: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, Default)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Tone {
    #[default]
    Default,
    Muted,
    Accent,
    Success,
    Warning,
    Danger,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, Default)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ButtonVariant {
    #[default]
    Primary,
    Ghost,
    Danger,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ListItem {
    pub id: String,
    pub text: String,
    pub meta: Option<String>,
    /// Optional per-item action (e.g. delete); the item id is sent as `values.itemId`.
    pub action: Option<String>,
    pub action_label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(export)]
pub enum ViewNode {
    Heading {
        text: String,
    },
    Text {
        text: String,
        tone: Tone,
    },
    Stat {
        label: String,
        value: String,
        hint: Option<String>,
    },
    Progress {
        label: String,
        value: f64,
    },
    List {
        items: Vec<ListItem>,
    },
    Button {
        action: String,
        label: String,
        variant: ButtonVariant,
    },
    Input {
        id: String,
        placeholder: String,
        value: String,
        #[serde(rename = "maxLength")]
        max_length: u32,
    },
    Divider,
    Empty {
        text: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Sanitized {
    pub nodes: Vec<ViewNode>,
    pub dropped: usize,
}

use crate::util::truncate_chars;
use serde_json::Value;

fn text_of(v: Option<&Value>, max: usize) -> Option<String> {
    match v? {
        Value::String(s) => Some(truncate_chars(s, max)),
        Value::Number(n) => Some(truncate_chars(&n.to_string(), max)),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

fn ident_of(v: Option<&Value>) -> Option<String> {
    let s = v?.as_str()?;
    let ok = !s.is_empty()
        && s.len() <= 64
        && s.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ':'));
    ok.then(|| s.to_string())
}

fn tone_of(v: Option<&Value>) -> Tone {
    v.and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default()
}

fn node_from(v: &Value) -> Option<ViewNode> {
    let o = v.as_object()?;
    let ty = o.get("type")?.as_str()?;
    Some(match ty {
        "heading" => ViewNode::Heading { text: text_of(o.get("text"), MAX_LABEL)? },
        "text" => ViewNode::Text { text: text_of(o.get("text"), MAX_TEXT)?, tone: tone_of(o.get("tone")) },
        "stat" => ViewNode::Stat {
            label: text_of(o.get("label"), MAX_LABEL)?,
            value: text_of(o.get("value"), MAX_LABEL)?,
            hint: text_of(o.get("hint"), MAX_LABEL),
        },
        "progress" => {
            let value = o.get("value")?.as_f64()?;
            ViewNode::Progress {
                label: text_of(o.get("label"), MAX_LABEL)?,
                value: if value.is_finite() { value.clamp(0.0, 1.0) } else { 0.0 },
            }
        }
        "list" => {
            let items = o.get("items")?.as_array()?;
            let items = items
                .iter()
                .take(MAX_LIST_ITEMS)
                .filter_map(|it| {
                    let io = it.as_object()?;
                    let action = ident_of(io.get("action"));
                    Some(ListItem {
                        id: ident_of(io.get("id"))?,
                        text: text_of(io.get("text"), MAX_TEXT)?,
                        meta: text_of(io.get("meta"), MAX_LABEL),
                        action_label: action.as_ref().and_then(|_| text_of(io.get("actionLabel"), 40)),
                        action,
                    })
                })
                .collect();
            ViewNode::List { items }
        }
        "button" => ViewNode::Button {
            action: ident_of(o.get("action"))?,
            label: text_of(o.get("label"), 40)?,
            variant: o.get("variant").and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default(),
        },
        "input" => {
            let max_length =
                o.get("maxLength").and_then(Value::as_u64).map(|n| n.clamp(1, MAX_TEXT as u64) as u32).unwrap_or(500);
            ViewNode::Input {
                id: ident_of(o.get("id"))?,
                placeholder: text_of(o.get("placeholder"), MAX_LABEL).unwrap_or_default(),
                value: text_of(o.get("value"), max_length as usize).unwrap_or_default(),
                max_length,
            }
        }
        "divider" => ViewNode::Divider,
        "empty" => ViewNode::Empty { text: text_of(o.get("text"), MAX_LABEL)? },
        _ => return None,
    })
}

/// Validate plugin-supplied view data. Unknown node types and malformed
/// nodes are dropped (and counted) rather than rejected wholesale, so one
/// bad node doesn't blank a panel.
pub fn sanitize(value: &Value) -> Sanitized {
    let Some(arr) = value.as_array() else {
        return Sanitized { nodes: Vec::new(), dropped: 0 };
    };
    let mut nodes = Vec::new();
    let mut dropped = 0;
    for v in arr {
        if nodes.len() >= MAX_NODES {
            dropped += 1;
            continue;
        }
        match node_from(v) {
            Some(n) => nodes.push(n),
            None => dropped += 1,
        }
    }
    Sanitized { nodes, dropped }
}
