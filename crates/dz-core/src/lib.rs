//! DiZero application core.
//!
//! This crate deliberately has no dependency on Tauri: everything here is
//! plain Rust that can be unit-tested headlessly. The desktop shell in
//! `apps/desktop/src-tauri` wires these services to IPC commands and events.

pub mod db;
pub mod demo;
pub mod error;
pub mod integrations;
pub mod logging;
pub mod models;
pub mod perf;
pub mod plugins;
pub mod settings;
pub mod store;
pub mod util;

pub use error::{CoreError, Result};
