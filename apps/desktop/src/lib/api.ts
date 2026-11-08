// Typed IPC layer. Every backend call goes through `call`, which turns the
// backend's `{ code, message }` errors into `IpcFailure` exceptions.

import { invoke } from "@tauri-apps/api/core";
import type { CommandOutcome } from "./bindings/CommandOutcome";
import type { InstalledPlugin } from "./bindings/InstalledPlugin";
import type { IpcError } from "./bindings/IpcError";
import type { LogEntry } from "./bindings/LogEntry";
import type { Member } from "./bindings/Member";
import type { Message } from "./bindings/Message";
import type { MessagePage } from "./bindings/MessagePage";
import type { PanelPlacement } from "./bindings/PanelPlacement";
import type { PerfMode } from "./bindings/PerfMode";
import type { PerfProfile } from "./bindings/PerfProfile";
import type { PerfSnapshot } from "./bindings/PerfSnapshot";
import type { Permission } from "./bindings/Permission";
import type { PermissionInfo } from "./bindings/PermissionInfo";
import type { PluginManifest } from "./bindings/PluginManifest";
import type { PluginRuntimeStats } from "./bindings/PluginRuntimeStats";
import type { SearchHit } from "./bindings/SearchHit";
import type { Settings } from "./bindings/Settings";
import type { StorageStats } from "./bindings/StorageStats";
import type { ValidationIssue } from "./bindings/ValidationIssue";
import type { ValidationReport } from "./bindings/ValidationReport";
import type { ViewNode } from "./bindings/ViewNode";
import type { WorkspaceSnapshot } from "./bindings/WorkspaceSnapshot";
import type { SlashCommand } from "./slash";

// Shapes defined in the Tauri shell (not exported by ts-rs).
export interface SettingsView {
  settings: Settings;
  profile: PerfProfile;
}

export interface AppInfo {
  version: string;
  tauriVersion: string;
  webviewVersion: string | null;
  os: string;
  arch: string;
  buildProfile: string;
  dataDir: string;
  logDir: string;
  databasePath: string;
  pluginsDir: string;
  bundledPluginsDir: string;
  schemaVersion: number;
  pluginApiVersion: string;
  recoveredDatabase: string | null;
  hardwareAccelerationAtLaunch: boolean;
}

export interface PluginView {
  plugin: InstalledPlugin;
  runtime: PluginRuntimeStats | null;
}

export interface CatalogView {
  folder: string;
  manifest: PluginManifest | null;
  issues: ValidationIssue[];
  installedVersion: string | null;
  updateAvailable: boolean;
}

export interface PluginsOverview {
  installed: PluginView[];
  catalog: CatalogView[];
  permissions: PermissionInfo[];
  limits: {
    memoryBytes: number;
    callBudgetMs: number;
    activateBudgetMs: number;
    storageQuotaBytes: number;
    maxFailures: number;
  };
}

export interface PanelView {
  pluginId: string;
  pluginName: string;
  id: string;
  title: string;
  placement: PanelPlacement;
  icon: string | null;
}

export interface ThemeView {
  pluginId: string;
  label: string;
  tokens: Record<string, string>;
}

export interface UiContributions {
  commands: SlashCommand[];
  panels: PanelView[];
  themes: ThemeView[];
}

export interface PerfView {
  process: PerfSnapshot;
  plugins: PluginRuntimeStats[];
  storage: StorageStats;
  uptimeMs: number;
  startupMs: number | null;
  pendingReplies: number;
  mode: PerfMode;
  profile: PerfProfile;
}

export interface FolderValidation {
  folder: string;
  report: ValidationReport;
}

export class IpcFailure extends Error {
  constructor(
    public readonly code: string,
    message: string,
  ) {
    super(message);
    this.name = "IpcFailure";
  }
}

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    const err = e as Partial<IpcError> | string;
    if (typeof err === "string") throw new IpcFailure("ipc", err);
    throw new IpcFailure(err?.code ?? "unknown", err?.message ?? String(e));
  }
}

export const api = {
  appReady: () => call<void>("app_ready"),
  appInfo: () => call<AppInfo>("app_info"),

  settingsGet: () => call<SettingsView>("settings_get"),
  settingsUpdate: (settings: Settings) => call<SettingsView>("settings_update", { settings }),
  settingsReset: () => call<SettingsView>("settings_reset"),
  settingsExport: () => call<string | null>("settings_export"),
  settingsImport: () => call<SettingsView | null>("settings_import"),

  workspaceSnapshot: () => call<WorkspaceSnapshot>("workspace_snapshot"),
  messagesList: (conversationId: string, before?: { createdAt: number; id: string }, limit?: number) =>
    call<MessagePage>("messages_list", { conversationId, before: before ?? null, limit: limit ?? null }),
  messageSend: (conversationId: string, content: string) => call<Message>("message_send", { conversationId, content }),
  messageDelete: (messageId: string) => call<void>("message_delete", { messageId }),
  markRead: (conversationId: string) => call<void>("conversation_mark_read", { conversationId }),
  members: (communityId: string) => call<Member[]>("members_list", { communityId }),
  search: (query: string) => call<SearchHit[]>("messages_search", { query }),

  pluginsOverview: () => call<PluginsOverview>("plugins_overview"),
  pluginContributions: () => call<UiContributions>("plugin_contributions"),
  pluginInstallBundled: (folder: string) => call<InstalledPlugin>("plugin_install_bundled", { folder }),
  pluginInstallFolder: () => call<InstalledPlugin | null>("plugin_install_folder"),
  pluginApprove: (pluginId: string, granted: Permission[]) => call<InstalledPlugin>("plugin_approve", { pluginId, granted }),
  pluginDisable: (pluginId: string) => call<InstalledPlugin>("plugin_disable", { pluginId }),
  pluginSetPermission: (pluginId: string, permission: Permission, granted: boolean) =>
    call<InstalledPlugin>("plugin_set_permission", { pluginId, permission, granted }),
  pluginUninstall: (pluginId: string) => call<void>("plugin_uninstall", { pluginId }),
  pluginRollback: (pluginId: string) => call<InstalledPlugin>("plugin_rollback", { pluginId }),
  pluginReset: (pluginId: string) => call<void>("plugin_reset", { pluginId }),
  pluginRunCommand: (pluginId: string, name: string, args: string) =>
    call<CommandOutcome>("plugin_run_command", { pluginId, name, args }),
  pluginRenderPanel: (pluginId: string, panel: string) => call<ViewNode[]>("plugin_render_panel", { pluginId, panel }),
  pluginPanelAction: (pluginId: string, panel: string, action: string, values: Record<string, string>) =>
    call<ViewNode[]>("plugin_panel_action", { pluginId, panel, action, values }),
  pluginValidateFolder: () => call<FolderValidation | null>("plugin_validate_folder"),

  perfSnapshot: () => call<PerfView>("perf_snapshot"),
  logsRecent: (after: number) => call<LogEntry[]>("logs_recent", { after }),
  logsOpenFolder: () => call<void>("logs_open_folder"),
  openLink: (url: string) => call<void>("open_link", { url }),

  dataStats: () => call<StorageStats>("data_stats"),
  dataResetDemo: () => call<WorkspaceSnapshot>("data_reset_demo"),
  dataVacuum: () => call<StorageStats>("data_vacuum"),
  dataPrune: (keepPerConversation: number) => call<number>("data_prune", { keepPerConversation }),
  dataBackup: () => call<string | null>("data_backup"),
  dataClearPluginStorage: (pluginId: string | null) => call<void>("data_clear_plugin_storage", { pluginId }),
};

export function errorMessage(e: unknown): string {
  if (e instanceof IpcFailure) return e.message;
  if (e instanceof Error) return e.message;
  return String(e);
}
