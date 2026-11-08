<script lang="ts">
  import { Users, Search, Puzzle } from "@lucide/svelte";
  import MemberList from "./MemberList.svelte";
  import SearchResults from "./SearchResults.svelte";
  import PluginPanel from "../plugins/PluginPanel.svelte";
  import PanelIcon from "../plugins/PanelIcon.svelte";
  import { plugins, panelKey } from "../../lib/stores/plugins.svelte";
  import { workspace } from "../../lib/stores/workspace.svelte";
  import { app } from "../../lib/stores/app.svelte";
  import { tooltip } from "../../lib/tooltip";

  const tabs = $derived([
    { key: "members", label: "Members" },
    ...(workspace.searchQuery ? [{ key: "search", label: "Search" }] : []),
    ...plugins.dockPanels.map((p) => ({ key: panelKey(p.pluginId, p.id), label: p.title })),
  ]);
  const activePanel = $derived(plugins.dockPanels.find((p) => panelKey(p.pluginId, p.id) === plugins.dockTab));

  $effect(() => {
    if (!tabs.some((t) => t.key === plugins.dockTab)) plugins.dockTab = "members";
  });

  function onTabKey(e: KeyboardEvent) {
    if (e.key !== "ArrowRight" && e.key !== "ArrowLeft") return;
    e.preventDefault();
    const i = tabs.findIndex((t) => t.key === plugins.dockTab);
    const next = tabs[(i + (e.key === "ArrowRight" ? 1 : -1) + tabs.length) % tabs.length];
    if (next) {
      plugins.dockTab = next.key;
      document.getElementById(`dock-tab-${next.key}`)?.focus();
    }
  }
</script>

<aside class="dock" aria-label="Side panel">
  <div class="tabs" role="tablist" aria-label="Side panel views" tabindex="-1" onkeydown={onTabKey}>
    {#each tabs as t (t.key)}
      {@const panel = plugins.dockPanels.find((p) => panelKey(p.pluginId, p.id) === t.key)}
      <button
        id="dock-tab-{t.key}"
        role="tab"
        class="tab"
        aria-selected={plugins.dockTab === t.key}
        aria-controls="dock-panel"
        tabindex={plugins.dockTab === t.key ? 0 : -1}
        aria-label={t.label}
        use:tooltip={{ text: panel ? `${t.label} (${panel.pluginName})` : t.label, side: "bottom" }}
        onclick={() => (plugins.dockTab = t.key)}
      >
        {#if t.key === "members"}<Users size={16} strokeWidth={1.8} />{:else if t.key === "search"}<Search
            size={16}
            strokeWidth={1.8}
          />{:else}<PanelIcon name={panel?.icon ?? null} size={16} />{/if}
      </button>
    {/each}
    <span class="tab-title">{tabs.find((t) => t.key === plugins.dockTab)?.label ?? ""}</span>
  </div>

  <div class="body" id="dock-panel" role="tabpanel">
    {#if plugins.dockTab === "members"}
      <MemberList />
    {:else if plugins.dockTab === "search"}
      <SearchResults />
    {:else if activePanel}
      {#key plugins.dockTab}
        <div class="panel-wrap">
          <PluginPanel panel={activePanel} />
          <p class="from"><Puzzle size={12} /> {activePanel.pluginName}</p>
        </div>
      {/key}
    {/if}
  </div>

  {#if plugins.dockPanels.length === 0 && plugins.dockTab === "members"}
    <button class="discover" onclick={() => app.go("plugins")}>
      <Puzzle size={14} />
      Add panels with plugins
    </button>
  {/if}
</aside>

<style>
  .dock {
    width: var(--dz-dock);
    flex: none;
    display: flex;
    flex-direction: column;
    min-height: 0;
    background: var(--dz-surface);
    border-left: 1px solid var(--dz-border);
  }
  .tabs {
    display: flex;
    align-items: center;
    gap: 2px;
    height: var(--dz-header);
    padding: 0 8px;
    border-bottom: 1px solid var(--dz-border);
    flex: none;
  }
  .tab {
    display: grid;
    place-items: center;
    width: 32px;
    height: 30px;
    border-radius: var(--dz-r-control);
    color: var(--dz-text-muted);
  }
  .tab:hover {
    color: var(--dz-text-primary);
    background: var(--dz-sidebar-hover);
  }
  .tab[aria-selected="true"] {
    color: var(--dz-text-primary);
    background: var(--dz-sidebar-active);
    box-shadow: inset 0 -2px 0 var(--dz-accent);
  }
  .tab-title {
    margin-left: auto;
    padding-right: 6px;
    color: var(--dz-text-muted);
    font-size: var(--dz-text-xs);
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }
  .panel-wrap {
    padding: 16px;
  }
  .from {
    display: flex;
    align-items: center;
    gap: 5px;
    margin-top: 18px;
    color: var(--dz-text-muted);
    font-size: var(--dz-text-xs);
  }
  .discover {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    margin: 8px;
    height: 32px;
    border: 1px dashed var(--dz-border-strong);
    border-radius: var(--dz-r-control);
    color: var(--dz-text-muted);
    font-size: var(--dz-text-xs);
    font-weight: 560;
    flex: none;
  }
  .discover:hover {
    color: var(--dz-text-primary);
    border-color: var(--dz-text-muted);
  }
  @media (max-width: 1080px) {
    .dock {
      width: 232px;
    }
  }
</style>
