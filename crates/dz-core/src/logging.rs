//! Application logging: a `log` backend writing to a size-capped file and an
//! in-memory ring buffer the Developer Center reads.

use crate::settings::LogLevel;
use serde::Serialize;
use std::collections::VecDeque;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LogEntry {
    #[ts(type = "number")]
    pub seq: u64,
    #[ts(type = "number")]
    pub ts: i64,
    pub level: String,
    pub target: String,
    pub message: String,
}

pub struct LogBuffer {
    inner: Mutex<(VecDeque<LogEntry>, u64)>,
    cap: usize,
}

impl LogBuffer {
    pub fn new(cap: usize) -> Self {
        LogBuffer { inner: Mutex::new((VecDeque::with_capacity(cap.min(1024)), 0)), cap }
    }
    pub fn push(&self, level: &str, target: &str, message: &str) {
        let mut g = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        g.1 += 1;
        let seq = g.1;
        if g.0.len() >= self.cap {
            g.0.pop_front();
        }
        g.0.push_back(LogEntry {
            seq,
            ts: crate::util::now_ms(),
            level: level.to_string(),
            target: target.to_string(),
            message: crate::util::truncate_chars(message, 4000),
        });
    }
    /// Entries with `seq > after`, oldest first, at most `limit`.
    pub fn since(&self, after: u64, limit: usize) -> Vec<LogEntry> {
        let g = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let mut out: Vec<LogEntry> = g.0.iter().filter(|e| e.seq > after).cloned().collect();
        if out.len() > limit {
            out.drain(..out.len() - limit);
        }
        out
    }
}

pub fn level_filter(level: LogLevel) -> log::LevelFilter {
    match level {
        LogLevel::Error => log::LevelFilter::Error,
        LogLevel::Warn => log::LevelFilter::Warn,
        LogLevel::Info => log::LevelFilter::Info,
        LogLevel::Debug => log::LevelFilter::Debug,
    }
}

const MAX_LOG_FILE: u64 = 4 * 1024 * 1024;

/// `log` backend. Our own targets log at the configured level; noisy
/// dependencies (tao, wry, tauri internals) are held to warnings.
pub struct DzLogger {
    buffer: Arc<LogBuffer>,
    file: Mutex<Option<(std::fs::File, u64)>>,
    path: PathBuf,
}

fn is_own_target(target: &str) -> bool {
    target.starts_with("dz_core") || target.starts_with("dizero") || target.starts_with("plugin:")
}

impl log::Log for DzLogger {
    fn enabled(&self, meta: &log::Metadata<'_>) -> bool {
        if is_own_target(meta.target()) { meta.level() <= log::max_level() } else { meta.level() <= log::Level::Warn }
    }

    fn log(&self, record: &log::Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let msg = record.args().to_string();
        self.buffer.push(record.level().as_str(), record.target(), &msg);
        let line = format!("{} {:5} [{}] {}\n", crate::util::now_ms(), record.level(), record.target(), msg);
        let mut g = self.file.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((_, written)) = g.as_mut() {
            if *written + line.len() as u64 > MAX_LOG_FILE {
                *g = rotate(&self.path);
            }
        }
        if let Some((f, written)) = g.as_mut() {
            if f.write_all(line.as_bytes()).is_ok() {
                *written += line.len() as u64;
            }
        }
    }

    fn flush(&self) {
        if let Some((f, _)) = self.file.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
            let _ = f.flush();
        }
    }
}

fn rotate(path: &Path) -> Option<(std::fs::File, u64)> {
    let _ = std::fs::rename(path, path.with_extension("log.1"));
    open_log(path)
}

fn open_log(path: &Path) -> Option<(std::fs::File, u64)> {
    let f = std::fs::OpenOptions::new().create(true).append(true).open(path).ok()?;
    let len = f.metadata().map(|m| m.len()).unwrap_or(0);
    Some((f, len))
}

/// Install the global logger. Safe to call once per process; later calls
/// only return the existing buffer's replacement and are ignored by `log`.
pub fn init(log_dir: &Path, level: LogLevel) -> Arc<LogBuffer> {
    let _ = std::fs::create_dir_all(log_dir);
    let path = log_dir.join("dizero.log");
    let mut file = open_log(&path);
    if file.as_ref().is_some_and(|(_, len)| *len > MAX_LOG_FILE) {
        file = rotate(&path);
    }
    let buffer = Arc::new(LogBuffer::new(1000));
    let logger = DzLogger { buffer: buffer.clone(), file: Mutex::new(file), path };
    // The logger lives for the whole process, so leaking it is intended.
    if log::set_logger(Box::leak(Box::new(logger))).is_ok() {
        log::set_max_level(level_filter(level));
    }
    buffer
}

pub fn set_level(level: LogLevel) {
    log::set_max_level(level_filter(level));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_buffer_caps_and_sequences() {
        let b = LogBuffer::new(3);
        for i in 0..5 {
            b.push("INFO", "t", &format!("m{i}"));
        }
        let all = b.since(0, 100);
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].message, "m2");
        assert_eq!(all[2].seq, 5);
        let newer = b.since(4, 100);
        assert_eq!(newer.len(), 1);
        assert_eq!(newer[0].message, "m4");
    }

    #[test]
    fn level_mapping() {
        assert_eq!(level_filter(LogLevel::Debug), log::LevelFilter::Debug);
        assert_eq!(level_filter(LogLevel::Error), log::LevelFilter::Error);
    }
}
