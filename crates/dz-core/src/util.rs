//! Small shared helpers: clocks and sortable identifiers.

use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Milliseconds since the Unix epoch.
pub fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

static COUNTER: AtomicU32 = AtomicU32::new(0);

/// Lexicographically sortable id: 12 hex digits of milliseconds followed by a
/// 6-hex-digit process-local counter. Ids created later sort later, which lets
/// the message store use the id as a stable tiebreaker.
pub fn new_id(prefix: &str) -> String {
    let ms = now_ms().max(0) as u64;
    let n = COUNTER.fetch_add(1, Ordering::Relaxed) & 0x00FF_FFFF;
    format!("{prefix}_{ms:012x}{n:06x}")
}

/// Lowercase hex encoding.
pub fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(DIGITS[(b >> 4) as usize] as char);
        s.push(DIGITS[(b & 0xf) as usize] as char);
    }
    s
}

/// Truncate to at most `max` characters on a char boundary.
pub fn truncate_chars(s: &str, max: usize) -> String {
    match s.char_indices().nth(max) {
        Some((idx, _)) => s[..idx].to_string(),
        None => s.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_sortable() {
        let a = new_id("m");
        let b = new_id("m");
        assert_ne!(a, b);
        assert!(a < b, "{a} should sort before {b}");
        assert!(a.starts_with("m_"));
    }

    #[test]
    fn truncate_respects_char_boundaries() {
        assert_eq!(truncate_chars("héllo", 2), "hé");
        assert_eq!(truncate_chars("hi", 10), "hi");
    }

    #[test]
    fn hex_encodes() {
        assert_eq!(hex(&[0x00, 0xab, 0xff]), "00abff");
    }
}
