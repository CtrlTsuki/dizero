<script lang="ts">
  import { onMount } from "svelte";
  import ViewRenderer from "./ViewRenderer.svelte";
  import type { PanelView } from "../../lib/api";
  import { plugins, panelKey } from "../../lib/stores/plugins.svelte";

  let { panel, compact = false }: { panel: PanelView; compact?: boolean } = $props();
  const ps = $derived(plugins.panels[panelKey(panel.pluginId, panel.id)]);

  onMount(() => {
    void plugins.renderPanel(panel);
  });
</script>

{#if !ps || ps.loading}
  <div class="loading" aria-busy="true">
    <div class="skeleton" style:height="12px" style:width="70%"></div>
    <div class="skeleton" style:height="12px" style:width="45%"></div>
  </div>
{:else}
  {#if ps.error}
    <div class="error" role="alert">
      <p>{panel.pluginName} hit an error: {ps.error}</p>
      <button class="btn btn-sm btn-ghost" onclick={() => plugins.renderPanel(panel)}>Try again</button>
    </div>
  {/if}
  <ViewRenderer nodes={ps.nodes} {compact} onAction={(action, values) => plugins.action(panel, action, values)} />
{/if}

<style>
  .loading {
    display: grid;
    gap: 8px;
    padding: 4px 0;
  }
  .error {
    display: grid;
    gap: 8px;
    justify-items: start;
    margin-bottom: 10px;
    padding: 10px;
    border: 1px solid color-mix(in srgb, var(--dz-danger) 40%, transparent);
    border-radius: var(--dz-r-control);
    color: var(--dz-text-secondary);
    font-size: var(--dz-text-sm);
    overflow-wrap: anywhere;
  }
</style>
