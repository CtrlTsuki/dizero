import { api, errorMessage, type PanelView, type PluginsOverview, type UiContributions } from "../api";
import type { ViewNode } from "../bindings/ViewNode";
import { toasts } from "./toasts.svelte";

export interface PanelState {
  nodes: ViewNode[];
  error: string | null;
  loading: boolean;
}

export const panelKey = (pluginId: string, panel: string) => `${pluginId}::${panel}`;

class Plugins {
  contributions = $state<UiContributions>({ commands: [], panels: [], themes: [] });
  overview = $state<PluginsOverview | null>(null);
  panels = $state<Record<string, PanelState>>({});
  /** Active dock tab: "members", "search" or a panel key. */
  dockTab = $state<string>("members");

  dockPanels = $derived(this.contributions.panels.filter((p) => p.placement === "dock"));
  sidebarPanels = $derived(this.contributions.panels.filter((p) => p.placement === "sidebar"));

  async loadContributions() {
    try {
      this.contributions = await api.pluginContributions();
      const valid = new Set(this.contributions.panels.map((p) => panelKey(p.pluginId, p.id)));
      if (this.dockTab !== "members" && this.dockTab !== "search" && !valid.has(this.dockTab)) {
        this.dockTab = "members";
      }
      for (const key of Object.keys(this.panels)) if (!valid.has(key)) delete this.panels[key];
    } catch (e) {
      toasts.error("Couldn't load plugin contributions", errorMessage(e));
    }
  }

  async loadOverview() {
    try {
      this.overview = await api.pluginsOverview();
    } catch (e) {
      toasts.error("Couldn't load plugins", errorMessage(e));
    }
  }

  async renderPanel(p: Pick<PanelView, "pluginId" | "id">) {
    const key = panelKey(p.pluginId, p.id);
    const prev = this.panels[key];
    this.panels[key] = { nodes: prev?.nodes ?? [], error: null, loading: !prev };
    try {
      const nodes = await api.pluginRenderPanel(p.pluginId, p.id);
      this.panels[key] = { nodes, error: null, loading: false };
    } catch (e) {
      this.panels[key] = { nodes: prev?.nodes ?? [], error: errorMessage(e), loading: false };
    }
  }

  async action(p: Pick<PanelView, "pluginId" | "id">, action: string, values: Record<string, string>) {
    const key = panelKey(p.pluginId, p.id);
    try {
      const nodes = await api.pluginPanelAction(p.pluginId, p.id, action, values);
      this.panels[key] = { nodes, error: null, loading: false };
    } catch (e) {
      const st = this.panels[key];
      this.panels[key] = { nodes: st?.nodes ?? [], error: errorMessage(e), loading: false };
    }
  }

  /** A plugin asked for a re-render; only panels already on screen refresh. */
  onRefresh(pluginId: string, panel: string) {
    if (this.panels[panelKey(pluginId, panel)]) void this.renderPanel({ pluginId, id: panel });
  }

  async onChanged() {
    await this.loadContributions();
    if (this.overview) await this.loadOverview();
    for (const key of Object.keys(this.panels)) {
      const [pluginId, id] = key.split("::");
      if (pluginId && id) void this.renderPanel({ pluginId, id });
    }
  }

  themeTokens(themeId: string): Record<string, string> {
    return this.contributions.themes.find((t) => t.pluginId === themeId)?.tokens ?? {};
  }
}

export const plugins = new Plugins();
