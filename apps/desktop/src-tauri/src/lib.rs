//! DiZero desktop shell: wires `dz-core` services to Tauri IPC and events.

mod commands;
mod events;
mod state;

use dz_core::db::Database;
use dz_core::plugins::host::{HostLimits, PluginHost};
use dz_core::plugins::registry::{DbPluginStorage, PluginRegistry};
use dz_core::{demo, logging, settings, store};
use state::{AppPaths, AppState};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, AtomicUsize};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use tauri::{Manager, RunEvent};

/// Messages kept per conversation in the local cache.
const CACHE_MESSAGES_PER_CONVERSATION: u32 = 5000;

/// `canonicalize` on Windows returns `\\?\C:\…` verbatim paths; show users the
/// ordinary form when it is equivalent.
fn strip_verbatim(p: PathBuf) -> PathBuf {
    match p.to_str().and_then(|s| s.strip_prefix(r"\\?\")) {
        Some(rest) if rest.as_bytes().get(1) == Some(&b':') => PathBuf::from(rest),
        _ => p,
    }
}

fn bundled_plugins_dir(app: &tauri::App) -> PathBuf {
    // During development read straight from the repository so example plugins
    // can be edited without rebuilding; release builds use bundled resources.
    if cfg!(debug_assertions) {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../plugins");
        if let Ok(p) = repo.canonicalize() {
            return strip_verbatim(p);
        }
    }
    app.path().resource_dir().map(|d| d.join("plugins")).unwrap_or_else(|_| PathBuf::from("plugins"))
}

fn setup(app: &mut tauri::App, started: Instant) -> Result<(), Box<dyn std::error::Error>> {
    let data_dir = app.path().app_data_dir()?;
    let log_dir = app.path().app_log_dir()?;
    let logs = logging::init(&log_dir, settings::LogLevel::Info);
    log::info!(
        "DiZero {} starting ({})",
        app.package_info().version,
        if cfg!(debug_assertions) { "debug" } else { "release" }
    );

    let db_path = data_dir.join("dizero.db");
    let opened = Database::open(&db_path)?;
    if let Some(q) = &opened.recovered_from {
        log::error!("database was unreadable and has been moved to {}; started fresh", q.display());
    }
    let db = Arc::new(opened.db);
    let settings = settings::load(&db)?;
    logging::set_level(settings.advanced.log_level);

    demo::seed(&db, &settings.profile.display_name)?;
    let pruned = store::prune(&db, CACHE_MESSAGES_PER_CONVERSATION)?;
    if pruned > 0 {
        log::info!("pruned {pruned} cached message(s) beyond the per-conversation limit");
    }

    let paths = AppPaths {
        plugins_installed: data_dir.join("plugins"),
        plugins_bundled: bundled_plugins_dir(app),
        database: db_path,
        data_dir,
        log_dir,
    };
    let registry = PluginRegistry::new(db.clone(), paths.plugins_installed.clone());
    let host = PluginHost::start(
        Arc::new(DbPluginStorage::new(db.clone())),
        Arc::new(events::TauriSink { app: app.handle().clone() }),
        HostLimits::default(),
    );
    host.set_idle_unload(settings.performance.mode.profile().plugin_idle_unload_secs.map(Duration::from_secs));

    // Plugins are registered dormant; none execute until first used.
    let (requests, problems) = registry.load_requests()?;
    let count = requests.len();
    for req in requests {
        let id = req.manifest.id.clone();
        if let Err(e) = host.register(req) {
            log::warn!("could not register plugin {id}: {e}");
        }
    }
    for (id, reason) in problems {
        log::warn!("plugin {id} not loaded: {reason}");
        let _ = registry.set_last_error(&id, Some(&reason));
    }
    log::info!("{count} plugin(s) registered (dormant until used)");

    // The window is created here rather than in tauri.conf.json because its
    // WebView2 arguments depend on a user setting (hardware acceleration).
    let hardware_acceleration = settings.performance.hardware_acceleration;
    tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::App("index.html".into()))
        .title("DiZero")
        .inner_size(1280.0, 820.0)
        .min_inner_size(940.0, 600.0)
        .center()
        .visible(false)
        .theme(Some(tauri::Theme::Dark))
        .background_color(tauri::window::Color(16, 16, 20, 255))
        .additional_browser_args(&settings::webview_browser_args(hardware_acceleration))
        .build()?;
    log::info!("window created (hardware acceleration {})", if hardware_acceleration { "on" } else { "off" });

    let state = AppState {
        hardware_acceleration_at_launch: hardware_acceleration,
        app: std::sync::OnceLock::new(),
        db,
        registry,
        host: RwLock::new(Some(host)),
        settings: RwLock::new(settings),
        sampler: Mutex::new(dz_core::perf::PerfSampler::new(std::process::id())),
        logs,
        paths,
        started,
        startup_ms: AtomicU64::new(0),
        pending_replies: Arc::new(AtomicUsize::new(0)),
        recovered_database: opened.recovered_from,
    };
    let _ = state.app.set(app.handle().clone());
    app.manage(state);

    // Safety net: if the UI never signals readiness (e.g. a frontend crash),
    // show the window anyway so the user isn't left with an invisible app.
    let handle = app.handle().clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(5));
        if let Some(w) = handle.get_webview_window("main") {
            if !w.is_visible().unwrap_or(true) {
                log::warn!("UI did not report ready within 5 s; showing window");
                let _ = w.show();
            }
        }
    });
    Ok(())
}

pub fn run() {
    let started = Instant::now();
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(move |app| setup(app, started))
        .invoke_handler(tauri::generate_handler![
            commands::app_ready,
            commands::app_info,
            commands::settings_get,
            commands::settings_update,
            commands::settings_reset,
            commands::settings_export,
            commands::settings_import,
            commands::workspace_snapshot,
            commands::messages_list,
            commands::message_send,
            commands::message_delete,
            commands::conversation_mark_read,
            commands::members_list,
            commands::messages_search,
            commands::plugins_overview,
            commands::plugin_contributions,
            commands::plugin_install_bundled,
            commands::plugin_install_folder,
            commands::plugin_approve,
            commands::plugin_disable,
            commands::plugin_set_permission,
            commands::plugin_uninstall,
            commands::plugin_rollback,
            commands::plugin_reset,
            commands::plugin_run_command,
            commands::plugin_render_panel,
            commands::plugin_panel_action,
            commands::plugin_validate_folder,
            commands::perf_snapshot,
            commands::logs_recent,
            commands::logs_open_folder,
            commands::open_link,
            commands::data_stats,
            commands::data_reset_demo,
            commands::data_vacuum,
            commands::data_prune,
            commands::data_backup,
            commands::data_clear_plugin_storage,
        ])
        .build(tauri::generate_context!())
        .expect("failed to build the DiZero application");

    app.run(|handle, event| {
        if let RunEvent::Exit = event {
            if let Some(state) = handle.try_state::<AppState>() {
                state.shutdown();
            }
        }
    });
}
