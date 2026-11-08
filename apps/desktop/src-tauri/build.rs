/// Every IPC command the frontend may call. Listing them here makes Tauri
/// generate one `allow-<command>` permission per command, and commands not
/// granted in `capabilities/` are rejected at the IPC layer. Keep in sync with
/// `generate_handler!` in `src/lib.rs`.
const COMMANDS: &[&str] = &[
    "app_ready",
    "app_info",
    "settings_get",
    "settings_update",
    "settings_reset",
    "settings_export",
    "settings_import",
    "workspace_snapshot",
    "messages_list",
    "message_send",
    "message_delete",
    "conversation_mark_read",
    "members_list",
    "messages_search",
    "plugins_overview",
    "plugin_contributions",
    "plugin_install_bundled",
    "plugin_install_folder",
    "plugin_approve",
    "plugin_disable",
    "plugin_set_permission",
    "plugin_uninstall",
    "plugin_rollback",
    "plugin_reset",
    "plugin_run_command",
    "plugin_render_panel",
    "plugin_panel_action",
    "plugin_validate_folder",
    "perf_snapshot",
    "logs_recent",
    "logs_open_folder",
    "open_link",
    "data_stats",
    "data_reset_demo",
    "data_vacuum",
    "data_prune",
    "data_backup",
    "data_clear_plugin_storage",
];

fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new().app_manifest(tauri_build::AppManifest::new().commands(COMMANDS)),
    )
    .expect("failed to run tauri-build");
}
