//! Shared application state managed by Tauri.

use dz_core::db::Database;
use dz_core::logging::LogBuffer;
use dz_core::perf::PerfSampler;
use dz_core::plugins::host::PluginHost;
use dz_core::plugins::registry::PluginRegistry;
use dz_core::settings::Settings;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, AtomicUsize};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;

pub struct AppPaths {
    pub data_dir: PathBuf,
    pub log_dir: PathBuf,
    pub database: PathBuf,
    pub plugins_installed: PathBuf,
    pub plugins_bundled: PathBuf,
}

pub struct AppState {
    /// Set once during setup; used for Rust-side dialogs.
    pub app: std::sync::OnceLock<tauri::AppHandle>,
    pub db: Arc<Database>,
    pub registry: PluginRegistry,
    /// `None` after shutdown.
    pub host: RwLock<Option<PluginHost>>,
    pub settings: RwLock<Settings>,
    pub sampler: Mutex<PerfSampler>,
    pub logs: Arc<LogBuffer>,
    pub paths: AppPaths,
    pub started: Instant,
    /// Milliseconds from `run()` to the frontend's first paint.
    pub startup_ms: AtomicU64,
    /// Scheduled demo replies not yet delivered.
    pub pending_replies: Arc<AtomicUsize>,
    pub recovered_database: Option<PathBuf>,
    /// The WebView2 rendering mode this process started with.
    pub hardware_acceleration_at_launch: bool,
}

impl AppState {
    pub fn with_host<R>(&self, f: impl FnOnce(&PluginHost) -> dz_core::Result<R>) -> dz_core::Result<R> {
        let guard = self.host.read().unwrap_or_else(|e| e.into_inner());
        match guard.as_ref() {
            Some(h) => f(h),
            None => Err(dz_core::CoreError::Plugin("plugin host is shut down".into())),
        }
    }

    pub fn settings(&self) -> Settings {
        self.settings.read().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Stop the plugin host (deactivating plugins) and flush the database.
    pub fn shutdown(&self) {
        let host = self.host.write().unwrap_or_else(|e| e.into_inner()).take();
        drop(host);
        self.db.checkpoint();
        log::info!("DiZero shut down cleanly");
        log::logger().flush();
    }
}
