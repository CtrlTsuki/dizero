//! IPC commands. Each one validates its input, delegates to `dz-core`, and
//! returns either data or an [`IpcError`] with a stable code.
//!
//! File and folder pickers are opened *here* in Rust, never by the webview,
//! so a compromised renderer cannot read or write arbitrary paths.

use crate::events::{self, MessageDeleted, Typing};
use crate::state::AppState;
use dz_core::demo::{self, DEMO_INTEGRATION_ID, DemoProvider};
use dz_core::error::IpcError;
use dz_core::integrations::Provider;
use dz_core::logging::LogEntry;
use dz_core::models::{Member, Message, MessagePage, SearchHit, WorkspaceSnapshot};
use dz_core::perf::PerfSnapshot;
use dz_core::plugins::host::{CommandOutcome, HostLimits, PluginMessage, PluginRuntimeStats};
use dz_core::plugins::manifest::{PanelPlacement, PluginKind, PluginManifest, ValidationIssue};
use dz_core::plugins::permissions::{Permission, PermissionInfo, catalogue};
use dz_core::plugins::registry::{InstalledPlugin, PluginRegistry, PluginSource, ValidationReport};
use dz_core::plugins::view::ViewNode;
use dz_core::settings::{self, PerfMode, PerfProfile, Settings};
use dz_core::store::{self, StorageStats};
use dz_core::{CoreError, logging};
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

type CmdResult<T> = Result<T, IpcError>;

fn err(e: CoreError) -> IpcError {
    // User errors are expected; plugin failures are already logged under the
    // plugin's own target by the host. Everything else is an app fault.
    if !matches!(
        e,
        CoreError::Invalid(_) | CoreError::NotFound(_) | CoreError::PermissionDenied(_) | CoreError::Plugin(_)
    ) {
        log::error!("command failed: {e}");
    }
    e.into()
}

fn invalid(msg: impl Into<String>) -> IpcError {
    IpcError { code: "invalid".into(), message: msg.into() }
}

/// Run blocking work (plugin calls, dialogs, disk I/O) off the async runtime.
async fn blocking<T: Send + 'static>(
    app: &AppHandle,
    f: impl FnOnce(&AppState) -> Result<T, CoreError> + Send + 'static,
) -> CmdResult<T> {
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || f(&app.state::<AppState>()))
        .await
        .map_err(|e| invalid(format!("background task failed: {e}")))?
        .map_err(err)
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn app_ready(app: AppHandle, state: State<'_, AppState>) -> CmdResult<()> {
    if state.startup_ms.load(Ordering::Relaxed) == 0 {
        let ms = state.started.elapsed().as_millis() as u64;
        state.startup_ms.store(ms.max(1), Ordering::Relaxed);
        log::info!("UI ready {ms} ms after launch");
    }
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.set_focus();
    }
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    version: String,
    tauri_version: String,
    webview_version: Option<String>,
    os: String,
    arch: String,
    build_profile: String,
    data_dir: String,
    log_dir: String,
    database_path: String,
    plugins_dir: String,
    bundled_plugins_dir: String,
    schema_version: i64,
    plugin_api_version: String,
    recovered_database: Option<String>,
    hardware_acceleration_at_launch: bool,
}

#[tauri::command]
pub async fn app_info(app: AppHandle, state: State<'_, AppState>) -> CmdResult<AppInfo> {
    Ok(AppInfo {
        version: app.package_info().version.to_string(),
        tauri_version: tauri::VERSION.to_string(),
        webview_version: tauri::webview_version().ok(),
        os: std::env::consts::OS.into(),
        arch: std::env::consts::ARCH.into(),
        build_profile: if cfg!(debug_assertions) { "debug" } else { "release" }.into(),
        data_dir: state.paths.data_dir.display().to_string(),
        log_dir: state.paths.log_dir.display().to_string(),
        database_path: state.paths.database.display().to_string(),
        plugins_dir: state.paths.plugins_installed.display().to_string(),
        bundled_plugins_dir: state.paths.plugins_bundled.display().to_string(),
        schema_version: state.db.schema_version().map_err(err)?,
        plugin_api_version: dz_core::plugins::API_VERSION.into(),
        recovered_database: state.recovered_database.as_ref().map(|p| p.display().to_string()),
        hardware_acceleration_at_launch: state.hardware_acceleration_at_launch,
    })
}

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsView {
    settings: Settings,
    profile: PerfProfile,
}

fn view(s: Settings) -> SettingsView {
    let profile = s.performance.mode.profile();
    SettingsView { settings: s, profile }
}

/// Persist settings and apply their runtime side effects.
fn apply_settings(state: &AppState, next: Settings) -> Result<Settings, CoreError> {
    let previous = state.settings();
    let saved = settings::save(&state.db, next)?;
    logging::set_level(saved.advanced.log_level);
    if saved.performance.mode != previous.performance.mode {
        let idle = saved.performance.mode.profile().plugin_idle_unload_secs.map(Duration::from_secs);
        state.with_host(|h| {
            h.set_idle_unload(idle);
            Ok(())
        })?;
        log::info!("performance mode set to {:?}", saved.performance.mode);
    }
    if saved.profile.display_name != previous.profile.display_name {
        demo::set_self_name(&state.db, &saved.profile.display_name)?;
    }
    *state.settings.write().unwrap_or_else(|e| e.into_inner()) = saved.clone();
    Ok(saved)
}

#[tauri::command]
pub async fn settings_get(state: State<'_, AppState>) -> CmdResult<SettingsView> {
    Ok(view(state.settings()))
}

#[tauri::command]
pub async fn settings_update(state: State<'_, AppState>, settings: Settings) -> CmdResult<SettingsView> {
    apply_settings(&state, settings).map(view).map_err(err)
}

#[tauri::command]
pub async fn settings_reset(state: State<'_, AppState>) -> CmdResult<SettingsView> {
    // Keep where the user was; reset everything else.
    let defaults = Settings { workspace: state.settings().workspace, ..Settings::default() };
    apply_settings(&state, defaults).map(view).map_err(err)
}

#[tauri::command]
pub async fn settings_export(app: AppHandle) -> CmdResult<Option<String>> {
    blocking(&app, |state| {
        let json = settings::export_json(&state.settings())?;
        let Some(path) = pick_save(state, "dizero-settings.json", "DiZero settings", &["json"]) else {
            return Ok(None);
        };
        std::fs::write(&path, json)?;
        log::info!("settings exported");
        Ok(Some(path.display().to_string()))
    })
    .await
}

#[tauri::command]
pub async fn settings_import(app: AppHandle) -> CmdResult<Option<SettingsView>> {
    blocking(&app, |state| {
        let Some(path) = pick_file(state, "DiZero settings", &["json"]) else { return Ok(None) };
        let meta = std::fs::metadata(&path)?;
        if meta.len() > 256 * 1024 {
            return Err(CoreError::invalid("settings file is too large"));
        }
        let imported = settings::import_json(&std::fs::read_to_string(&path)?)?;
        let saved = apply_settings(state, imported)?;
        log::info!("settings imported");
        Ok(Some(view(saved)))
    })
    .await
}

// Dialog helpers (Rust-side only).
fn app_handle(state: &AppState) -> Option<AppHandle> {
    state.app.get().cloned()
}

fn pick_save(state: &AppState, default_name: &str, filter: &str, ext: &[&str]) -> Option<std::path::PathBuf> {
    let app = app_handle(state)?;
    app.dialog()
        .file()
        .set_file_name(default_name)
        .add_filter(filter, ext)
        .blocking_save_file()
        .and_then(|p| p.into_path().ok())
}

fn pick_file(state: &AppState, filter: &str, ext: &[&str]) -> Option<std::path::PathBuf> {
    let app = app_handle(state)?;
    app.dialog().file().add_filter(filter, ext).blocking_pick_file().and_then(|p| p.into_path().ok())
}

fn pick_folder(state: &AppState, title: &str) -> Option<std::path::PathBuf> {
    let app = app_handle(state)?;
    app.dialog().file().set_title(title).blocking_pick_folder().and_then(|p| p.into_path().ok())
}

// ---------------------------------------------------------------------------
// Workspace & messages
// ---------------------------------------------------------------------------

fn provider_for(state: &AppState, conversation_id: &str) -> Result<&'static dyn Provider, CoreError> {
    let integration = store::conversation_integration(&state.db, conversation_id)?;
    match integration.as_str() {
        DEMO_INTEGRATION_ID => Ok(&DemoProvider),
        other => Err(CoreError::Unsupported(format!("no provider available for integration `{other}`"))),
    }
}

fn dispatch_to_plugins(state: &AppState, m: &Message) {
    let _ = state.with_host(|h| {
        h.dispatch_message(&PluginMessage::from(m));
        Ok(())
    });
}

#[tauri::command]
pub async fn workspace_snapshot(state: State<'_, AppState>) -> CmdResult<WorkspaceSnapshot> {
    store::workspace_snapshot(&state.db).map_err(err)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cursor {
    created_at: i64,
    id: String,
}

#[tauri::command]
pub async fn messages_list(
    state: State<'_, AppState>,
    conversation_id: String,
    before: Option<Cursor>,
    limit: Option<u32>,
) -> CmdResult<MessagePage> {
    let limit = limit.unwrap_or_else(|| state.settings().performance.mode.profile().page_size);
    store::list_messages(&state.db, &conversation_id, before.map(|c| (c.created_at, c.id)), limit).map_err(err)
}

#[tauri::command]
pub async fn message_send(
    app: AppHandle,
    state: State<'_, AppState>,
    conversation_id: String,
    content: String,
) -> CmdResult<Message> {
    let provider = provider_for(&state, &conversation_id).map_err(err)?;
    let outcome = provider.send_message(&state.db, &conversation_id, &content).map_err(err)?;
    let _ = app.emit(events::MESSAGE_CREATED, &outcome.message);
    dispatch_to_plugins(&state, &outcome.message);

    if let Some(reply) = outcome.reply {
        let author = store::get_user(&state.db, &reply.author_id).map_err(err)?;
        let pending = state.pending_replies.clone();
        pending.fetch_add(1, Ordering::Relaxed);
        let app2 = app.clone();
        let conv = conversation_id.clone();
        tauri::async_runtime::spawn(async move {
            let typing = |active| Typing {
                conversation_id: conv.clone(),
                user_id: author.id.clone(),
                display_name: author.display_name.clone(),
                active,
            };
            tokio_sleep(Duration::from_millis(250)).await;
            let _ = app2.emit(events::TYPING, typing(true));
            tokio_sleep(Duration::from_millis(reply.delay_ms)).await;
            let _ = app2.emit(events::TYPING, typing(false));
            let st = app2.state::<AppState>();
            match demo::deliver_reply(&st.db, &conv, &reply) {
                Ok(m) => {
                    let _ = app2.emit(events::MESSAGE_CREATED, &m);
                    dispatch_to_plugins(&st, &m);
                }
                Err(e) => log::warn!("demo reply failed: {e}"),
            }
            pending.fetch_sub(1, Ordering::Relaxed);
        });
    }
    Ok(outcome.message)
}

async fn tokio_sleep(d: Duration) {
    tokio::time::sleep(d).await;
}

#[tauri::command]
pub async fn message_delete(app: AppHandle, state: State<'_, AppState>, message_id: String) -> CmdResult<()> {
    let msg = store::get_message(&state.db, &message_id).map_err(err)?;
    let provider = provider_for(&state, &msg.conversation_id).map_err(err)?;
    provider.delete_message(&state.db, &message_id).map_err(err)?;
    let _ = app.emit(events::MESSAGE_DELETED, MessageDeleted { id: message_id, conversation_id: msg.conversation_id });
    Ok(())
}

#[tauri::command]
pub async fn conversation_mark_read(state: State<'_, AppState>, conversation_id: String) -> CmdResult<()> {
    store::mark_read(&state.db, &conversation_id).map_err(err)
}

#[tauri::command]
pub async fn members_list(state: State<'_, AppState>, community_id: String) -> CmdResult<Vec<Member>> {
    store::members(&state.db, &community_id).map_err(err)
}

#[tauri::command]
pub async fn messages_search(state: State<'_, AppState>, query: String) -> CmdResult<Vec<SearchHit>> {
    if query.chars().count() > 200 {
        return Err(invalid("search query is too long"));
    }
    store::search(&state.db, &query, 50).map_err(err)
}

// ---------------------------------------------------------------------------
// Plugins
// ---------------------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginView {
    plugin: InstalledPlugin,
    runtime: Option<PluginRuntimeStats>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogView {
    folder: String,
    manifest: Option<PluginManifest>,
    issues: Vec<ValidationIssue>,
    installed_version: Option<String>,
    update_available: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LimitsView {
    memory_bytes: usize,
    call_budget_ms: u64,
    activate_budget_ms: u64,
    storage_quota_bytes: usize,
    max_failures: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginsOverview {
    installed: Vec<PluginView>,
    catalog: Vec<CatalogView>,
    permissions: Vec<PermissionInfo>,
    limits: LimitsView,
}

#[tauri::command]
pub async fn plugins_overview(app: AppHandle) -> CmdResult<PluginsOverview> {
    blocking(&app, |state| {
        let runtime = state.with_host(|h| Ok(h.stats())).unwrap_or_default();
        let installed: Vec<PluginView> = state
            .registry
            .list()?
            .into_iter()
            .map(|p| {
                let rt = runtime.iter().find(|r| r.id == p.manifest.id).cloned();
                PluginView { plugin: p, runtime: rt }
            })
            .collect();
        let catalog = PluginRegistry::scan_catalog(&state.paths.plugins_bundled)
            .into_iter()
            .map(|c| {
                let installed_version = c.manifest.as_ref().and_then(|m| {
                    installed.iter().find(|p| p.plugin.manifest.id == m.id).map(|p| p.plugin.manifest.version.clone())
                });
                let update_available = match (&c.manifest, &installed_version) {
                    (Some(m), Some(v)) => matches!(
                        (semver::Version::parse(&m.version), semver::Version::parse(v)),
                        (Ok(a), Ok(b)) if a > b
                    ),
                    _ => false,
                };
                CatalogView {
                    folder: c.folder,
                    manifest: c.manifest,
                    issues: c.issues,
                    installed_version,
                    update_available,
                }
            })
            .collect();
        let l = HostLimits::default();
        Ok(PluginsOverview {
            installed,
            catalog,
            permissions: catalogue(),
            limits: LimitsView {
                memory_bytes: l.memory_bytes,
                call_budget_ms: l.call_budget.as_millis() as u64,
                activate_budget_ms: l.activate_budget.as_millis() as u64,
                storage_quota_bytes: l.storage_quota_bytes,
                max_failures: l.max_failures,
            },
        })
    })
    .await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandView {
    plugin_id: String,
    plugin_name: String,
    name: String,
    description: String,
    usage: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PanelView {
    plugin_id: String,
    plugin_name: String,
    id: String,
    title: String,
    placement: PanelPlacement,
    icon: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeView {
    plugin_id: String,
    label: String,
    tokens: std::collections::BTreeMap<String, String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Contributions {
    commands: Vec<CommandView>,
    panels: Vec<PanelView>,
    themes: Vec<ThemeView>,
}

/// What enabled plugins contribute to the UI, filtered by granted permissions.
#[tauri::command]
pub async fn plugin_contributions(state: State<'_, AppState>) -> CmdResult<Contributions> {
    let mut out = Contributions { commands: vec![], panels: vec![], themes: vec![] };
    for p in state.registry.list().map_err(err)? {
        if !p.enabled || p.state != dz_core::plugins::registry::RegistryState::Enabled {
            continue;
        }
        let granted = |perm: Permission| p.permissions.iter().any(|g| g.permission == perm && g.granted);
        let m = &p.manifest;
        if m.kind == PluginKind::Theme {
            if let (Some(t), true) = (&m.theme, granted(Permission::UiThemes)) {
                out.themes.push(ThemeView {
                    plugin_id: m.id.clone(),
                    label: t.label.clone(),
                    tokens: t.tokens.clone(),
                });
            }
            continue;
        }
        if granted(Permission::CommandsSlash) {
            for c in &m.contributes.commands {
                out.commands.push(CommandView {
                    plugin_id: m.id.clone(),
                    plugin_name: m.name.clone(),
                    name: c.name.clone(),
                    description: c.description.clone(),
                    usage: c.usage.clone(),
                });
            }
        }
        for panel in &m.contributes.panels {
            let allowed = match panel.placement {
                PanelPlacement::Dock => granted(Permission::UiPanels),
                PanelPlacement::Sidebar => granted(Permission::UiSidebar),
            };
            if allowed {
                out.panels.push(PanelView {
                    plugin_id: m.id.clone(),
                    plugin_name: m.name.clone(),
                    id: panel.id.clone(),
                    title: panel.title.clone(),
                    placement: panel.placement,
                    icon: panel.icon.clone(),
                });
            }
        }
    }
    Ok(out)
}

/// Bring the runtime host in line with the registry for one plugin.
fn sync_host(state: &AppState, id: &str) -> Result<(), CoreError> {
    state.with_host(|h| {
        let _ = h.unregister(id);
        match state.registry.load_request(id) {
            Ok(Some(req)) => {
                h.register(req)?;
                state.registry.set_last_error(id, None)?;
            }
            Ok(None) => {}
            Err(e) => {
                state.registry.set_last_error(id, Some(&e.to_string()))?;
                return Err(e);
            }
        }
        Ok(())
    })
}

fn changed(app: &AppHandle) {
    let _ = app.emit(events::PLUGINS_CHANGED, ());
}

#[tauri::command]
pub async fn plugin_install_bundled(app: AppHandle, folder: String) -> CmdResult<InstalledPlugin> {
    let out = blocking(&app, move |state| {
        // Only folders that the catalog scan actually lists are installable.
        let known = PluginRegistry::scan_catalog(&state.paths.plugins_bundled).into_iter().any(|c| c.folder == folder);
        if !known {
            return Err(CoreError::NotFound(format!("bundled plugin `{folder}`")));
        }
        let p = state.registry.install_from_dir(&state.paths.plugins_bundled.join(&folder), PluginSource::Bundled)?;
        if p.enabled {
            sync_host(state, &p.manifest.id)?;
        }
        Ok(p)
    })
    .await;
    changed(&app);
    out
}

#[tauri::command]
pub async fn plugin_install_folder(app: AppHandle) -> CmdResult<Option<InstalledPlugin>> {
    let out = blocking(&app, |state| {
        let Some(dir) = pick_folder(state, "Choose a DiZero plugin folder") else { return Ok(None) };
        let p = state.registry.install_from_dir(&dir, PluginSource::Local)?;
        if p.enabled {
            sync_host(state, &p.manifest.id)?;
        }
        Ok(Some(p))
    })
    .await;
    changed(&app);
    out
}

#[tauri::command]
pub async fn plugin_approve(app: AppHandle, plugin_id: String, granted: Vec<Permission>) -> CmdResult<InstalledPlugin> {
    let out = blocking(&app, move |state| {
        let p = state.registry.approve_and_enable(&plugin_id, &granted)?;
        sync_host(state, &plugin_id)?;
        log::info!("plugin {plugin_id} enabled with {} permission(s)", granted.len());
        Ok(p)
    })
    .await;
    changed(&app);
    out
}

#[tauri::command]
pub async fn plugin_disable(app: AppHandle, plugin_id: String) -> CmdResult<InstalledPlugin> {
    let out = blocking(&app, move |state| {
        let p = state.registry.disable(&plugin_id)?;
        let _ = state.with_host(|h| h.unregister(&plugin_id));
        Ok(p)
    })
    .await;
    changed(&app);
    out
}

#[tauri::command]
pub async fn plugin_set_permission(
    app: AppHandle,
    plugin_id: String,
    permission: Permission,
    granted: bool,
) -> CmdResult<InstalledPlugin> {
    let out = blocking(&app, move |state| {
        let p = state.registry.set_permission(&plugin_id, permission, granted)?;
        sync_host(state, &plugin_id)?;
        Ok(p)
    })
    .await;
    changed(&app);
    out
}

#[tauri::command]
pub async fn plugin_uninstall(app: AppHandle, plugin_id: String) -> CmdResult<()> {
    let out = blocking(&app, move |state| {
        let _ = state.with_host(|h| h.unregister(&plugin_id));
        state.registry.uninstall(&plugin_id)
    })
    .await;
    changed(&app);
    out
}

#[tauri::command]
pub async fn plugin_rollback(app: AppHandle, plugin_id: String) -> CmdResult<InstalledPlugin> {
    let out = blocking(&app, move |state| {
        let p = state.registry.rollback(&plugin_id)?;
        sync_host(state, &plugin_id)?;
        Ok(p)
    })
    .await;
    changed(&app);
    out
}

#[tauri::command]
pub async fn plugin_reset(app: AppHandle, plugin_id: String) -> CmdResult<()> {
    let out = blocking(&app, move |state| {
        state.registry.set_last_error(&plugin_id, None)?;
        sync_host(state, &plugin_id)
    })
    .await;
    changed(&app);
    out
}

#[tauri::command]
pub async fn plugin_run_command(
    app: AppHandle,
    plugin_id: String,
    name: String,
    args: String,
) -> CmdResult<CommandOutcome> {
    blocking(&app, move |state| state.with_host(|h| h.run_command(&plugin_id, &name, &args))).await
}

#[tauri::command]
pub async fn plugin_render_panel(app: AppHandle, plugin_id: String, panel: String) -> CmdResult<Vec<ViewNode>> {
    blocking(&app, move |state| state.with_host(|h| h.render_panel(&plugin_id, &panel))).await
}

#[tauri::command]
pub async fn plugin_panel_action(
    app: AppHandle,
    plugin_id: String,
    panel: String,
    action: String,
    values: serde_json::Value,
) -> CmdResult<Vec<ViewNode>> {
    if values.to_string().len() > 16 * 1024 {
        return Err(invalid("panel input is too large"));
    }
    blocking(&app, move |state| state.with_host(|h| h.panel_action(&plugin_id, &panel, &action, values))).await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderValidation {
    folder: String,
    report: ValidationReport,
}

#[tauri::command]
pub async fn plugin_validate_folder(app: AppHandle) -> CmdResult<Option<FolderValidation>> {
    blocking(&app, |state| {
        let Some(dir) = pick_folder(state, "Choose a plugin folder to validate") else { return Ok(None) };
        let report = PluginRegistry::validate_dir(&dir);
        log::info!("validated plugin folder: {} issue(s)", report.issues.len());
        Ok(Some(FolderValidation { folder: dir.display().to_string(), report }))
    })
    .await
}

// ---------------------------------------------------------------------------
// Performance, logs, data
// ---------------------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PerfView {
    process: PerfSnapshot,
    plugins: Vec<PluginRuntimeStats>,
    storage: StorageStats,
    uptime_ms: u64,
    startup_ms: Option<u64>,
    pending_replies: usize,
    mode: PerfMode,
    profile: PerfProfile,
}

#[tauri::command]
pub async fn perf_snapshot(app: AppHandle) -> CmdResult<PerfView> {
    blocking(&app, |state| {
        let process = state.sampler.lock().unwrap_or_else(|e| e.into_inner()).sample();
        let plugins = state.with_host(|h| Ok(h.stats())).unwrap_or_default();
        let mode = state.settings().performance.mode;
        let startup = state.startup_ms.load(Ordering::Relaxed);
        Ok(PerfView {
            process,
            plugins,
            storage: store::storage_stats(&state.db)?,
            uptime_ms: state.started.elapsed().as_millis() as u64,
            startup_ms: (startup > 0).then_some(startup),
            pending_replies: state.pending_replies.load(Ordering::Relaxed),
            mode,
            profile: mode.profile(),
        })
    })
    .await
}

#[tauri::command]
pub async fn logs_recent(state: State<'_, AppState>, after: Option<u64>) -> CmdResult<Vec<LogEntry>> {
    Ok(state.logs.since(after.unwrap_or(0), 500))
}

/// Open an http(s) link from a message in the default browser. The UI asks
/// the user to confirm first; this re-validates the scheme regardless.
#[tauri::command]
pub async fn open_link(app: AppHandle, url: String) -> CmdResult<()> {
    let ok_scheme = url.starts_with("https://") || url.starts_with("http://");
    let clean = !url.chars().any(|c| c.is_whitespace() || c.is_control());
    if !ok_scheme || !clean || url.len() > 2048 {
        return Err(invalid("only http and https links can be opened"));
    }
    app.opener().open_url(url, None::<&str>).map_err(|e| invalid(format!("could not open the link: {e}")))
}

#[tauri::command]
pub async fn logs_open_folder(app: AppHandle, state: State<'_, AppState>) -> CmdResult<()> {
    app.opener()
        .open_path(state.paths.log_dir.display().to_string(), None::<&str>)
        .map_err(|e| invalid(format!("could not open the log folder: {e}")))
}

#[tauri::command]
pub async fn data_stats(state: State<'_, AppState>) -> CmdResult<StorageStats> {
    store::storage_stats(&state.db).map_err(err)
}

#[tauri::command]
pub async fn data_reset_demo(state: State<'_, AppState>) -> CmdResult<WorkspaceSnapshot> {
    demo::reset(&state.db, &state.settings().profile.display_name).map_err(err)?;
    log::info!("demo workspace reset");
    store::workspace_snapshot(&state.db).map_err(err)
}

#[tauri::command]
pub async fn data_vacuum(app: AppHandle) -> CmdResult<StorageStats> {
    blocking(&app, |state| {
        state.db.vacuum()?;
        store::storage_stats(&state.db)
    })
    .await
}

#[tauri::command]
pub async fn data_prune(state: State<'_, AppState>, keep_per_conversation: u32) -> CmdResult<usize> {
    if !(50..=100_000).contains(&keep_per_conversation) {
        return Err(invalid("keep between 50 and 100000 messages per conversation"));
    }
    store::prune(&state.db, keep_per_conversation).map_err(err)
}

#[tauri::command]
pub async fn data_backup(app: AppHandle) -> CmdResult<Option<String>> {
    blocking(&app, |state| {
        let name = format!("dizero-backup-{}.db", dz_core::util::now_ms());
        let Some(path) = pick_save(state, &name, "SQLite database", &["db"]) else { return Ok(None) };
        state.db.backup_to(&path)?;
        log::info!("database backed up");
        Ok(Some(path.display().to_string()))
    })
    .await
}

#[tauri::command]
pub async fn data_clear_plugin_storage(app: AppHandle, plugin_id: Option<String>) -> CmdResult<()> {
    blocking(&app, move |state| {
        state.db.with(|c| {
            match &plugin_id {
                Some(id) => c.execute("DELETE FROM plugin_storage WHERE plugin_id = ?1", [id])?,
                None => c.execute("DELETE FROM plugin_storage", [])?,
            };
            Ok(())
        })?;
        // Restart affected runtimes so they don't keep stale in-memory copies.
        let ids: Vec<String> = match plugin_id {
            Some(id) => vec![id],
            None => state.registry.list()?.into_iter().map(|p| p.manifest.id).collect(),
        };
        for id in ids {
            let _ = sync_host(state, &id);
        }
        Ok(())
    })
    .await
}
