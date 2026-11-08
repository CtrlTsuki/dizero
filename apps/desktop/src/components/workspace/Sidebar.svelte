<script lang="ts">
  import { Hash, Megaphone, ChevronDown } from "@lucide/svelte";
  import UserPanel from "./UserPanel.svelte";
  import PluginPanel from "../plugins/PluginPanel.svelte";
  import { app } from "../../lib/stores/app.svelte";
  import { workspace } from "../../lib/stores/workspace.svelte";
  import { plugins } from "../../lib/stores/plugins.svelte";

  let { loading = false }: { loading?: boolean } = $props();
  let collapsed = $state<Record<string, boolean>>({});
  let dragWidth = $state<number | null>(null);

  const width = $derived(dragWidth ?? app.settings?.appearance.sidebarWidth ?? 248);
  const community = $derived(workspace.activeCommunity);
  const groups = $derived(community ? workspace.grouped(community.id) : []);

  function persistWidth(w: number) {
    const clamped = Math.round(Math.min(420, Math.max(200, w)));
    void app.update((s) => (s.appearance.sidebarWidth = clamped));
  }

  function startDrag(e: PointerEvent) {
    const handle = e.currentTarget as HTMLElement;
    handle.setPointerCapture(e.pointerId);
    const startX = e.clientX;
    const startW = width;
    const move = (ev: PointerEvent) => (dragWidth = Math.min(420, Math.max(200, startW + ev.clientX - startX)));
    const up = () => {
      handle.removeEventListener("pointermove", move);
      handle.removeEventListener("pointerup", up);
      if (dragWidth !== null) persistWidth(dragWidth);
      dragWidth = null;
    };
    handle.addEventListener("pointermove", move);
    handle.addEventListener("pointerup", up);
  }

  function resizeKey(e: KeyboardEvent) {
    if (e.key === "ArrowLeft" || e.key === "ArrowRight") {
      e.preventDefault();
      persistWidth(width + (e.key === "ArrowRight" ? 16 : -16));
    }
  }

  const hover = (id: string) => {
    if (app.profile?.preloadAdjacent) workspace.prefetch(id);
  };
</script>

<aside class="sidebar" style:width="{width}px" aria-label="Channels">
  <header class="community">
    {#if community}
      <h2 title={community.description ?? community.name}>{community.name}</h2>
      <span class="source">Local demo</span>
    {:else}
      <div class="skeleton" style:width="60%" style:height="14px"></div>
    {/if}
  </header>

  <nav class="channels">
    {#if loading && groups.length === 0}
      {#each [70, 55, 80, 62, 48] as w, i (i)}
        <div class="skeleton sk" style:width="{w}%"></div>
      {/each}
    {/if}
    {#each groups as g (g.category)}
      <section>
        <button
          class="category"
          aria-expanded={!collapsed[g.category]}
          onclick={() => (collapsed[g.category] = !collapsed[g.category])}
        >
          <span class="chev" class:closed={collapsed[g.category]}><ChevronDown size={12} strokeWidth={2.25} /></span>
          {g.category}
        </button>
        {#if !collapsed[g.category]}
          <ul>
            {#each g.items as c (c.id)}
              {@const active = c.id === workspace.activeConversationId}
              <li>
                <button
                  class="channel"
                  class:active
                  class:unread={c.unreadCount > 0 && !active}
                  aria-current={active ? "page" : undefined}
                  onclick={() => workspace.selectConversation(c.id)}
                  onmouseenter={() => hover(c.id)}
                >
                  <span class="glyph" aria-hidden="true">
                    {#if c.kind === "announcement"}
                      <Megaphone size={15} strokeWidth={1.9} />
                    {:else}
                      <Hash size={15} strokeWidth={1.9} />
                    {/if}
                  </span>
                  <span class="name">{c.name}</span>
                  {#if c.mentionCount > 0 && !active}
                    <span class="mention num" aria-label="{c.mentionCount} mentions">{c.mentionCount}</span>
                  {:else if c.unreadCount > 0 && !active}
                    <span class="sr-only">unread</span>
                  {/if}
                </button>
              </li>
            {/each}
          </ul>
        {/if}
      </section>
    {/each}
  </nav>

  {#each plugins.sidebarPanels as p (p.pluginId + p.id)}
    <section class="plugin-section" aria-label={p.title}>
      <h3>{p.title}</h3>
      <PluginPanel panel={p} compact />
    </section>
  {/each}

  <UserPanel />

  <!-- WAI-ARIA window splitter: a focusable separator adjusted with arrow keys. -->
  <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
  <div
    class="resize"
    role="separator"
    aria-orientation="vertical"
    aria-label="Resize channel list"
    aria-valuenow={width}
    aria-valuemin={200}
    aria-valuemax={420}
    tabindex="0"
    onpointerdown={startDrag}
    onkeydown={resizeKey}
  ></div>
</aside>

<style>
  .sidebar {
    position: relative;
    flex: none;
    display: flex;
    flex-direction: column;
    min-height: 0;
    background: var(--dz-surface);
    border-right: 1px solid var(--dz-border);
  }
  .community {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 8px;
    height: var(--dz-header);
    padding: 0 16px;
    border-bottom: 1px solid var(--dz-border);
    flex: none;
    align-content: center;
    flex-wrap: wrap;
  }
  .community h2 {
    font-size: var(--dz-text-md);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    line-height: var(--dz-header);
  }
  .source {
    color: var(--dz-text-muted);
    font-size: var(--dz-text-xs);
    white-space: nowrap;
  }
  .channels {
    flex: 1;
    overflow-y: auto;
    padding: 10px 8px 16px;
  }
  .sk {
    height: 14px;
    margin: 12px 8px;
  }
  section + section {
    margin-top: 14px;
  }
  .category {
    display: flex;
    align-items: center;
    gap: 4px;
    width: 100%;
    padding: 4px 6px;
    color: var(--dz-text-muted);
    font-size: var(--dz-text-xs);
    font-weight: 600;
    letter-spacing: 0.01em;
    border-radius: 4px;
  }
  .category:hover {
    color: var(--dz-text-secondary);
  }
  .chev {
    display: inline-flex;
    transition: transform var(--dz-fast) var(--dz-ease);
  }
  .chev.closed {
    transform: rotate(-90deg);
  }
  ul {
    list-style: none;
    margin: 2px 0 0;
    padding: 0;
    display: grid;
    gap: 1px;
  }
  .channel {
    position: relative;
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    height: 32px;
    padding: 0 8px;
    border-radius: var(--dz-r-control);
    color: var(--dz-text-muted);
    text-align: left;
    transition: background-color var(--dz-fast) var(--dz-ease), color var(--dz-fast) var(--dz-ease);
  }
  .channel:hover {
    background: var(--dz-sidebar-hover);
    color: var(--dz-text-secondary);
  }
  .channel.active {
    background: var(--dz-sidebar-active);
    color: var(--dz-text-primary);
  }
  .channel.unread {
    color: var(--dz-text-primary);
    font-weight: 600;
  }
  /* Unread marker: a lit segment on the list's left edge. */
  .channel.unread::before {
    content: "";
    position: absolute;
    left: -8px;
    top: 50%;
    width: 3px;
    height: 8px;
    transform: translateY(-50%);
    border-radius: 0 3px 3px 0;
    background: var(--dz-text-primary);
  }
  .glyph {
    display: inline-flex;
    opacity: 0.8;
  }
  .name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .mention {
    min-width: 18px;
    height: 18px;
    padding: 0 5px;
    border-radius: 9px;
    background: var(--dz-danger);
    color: #1a0b0e;
    font-size: 11px;
    font-weight: 700;
    line-height: 18px;
    text-align: center;
  }
  .plugin-section {
    margin: 0 8px 8px;
    padding: 10px 12px;
    border: 1px solid var(--dz-border);
    border-radius: var(--dz-r-control);
    background: var(--dz-background);
    max-height: 160px;
    overflow-y: auto;
  }
  .plugin-section h3 {
    color: var(--dz-text-muted);
    font-size: var(--dz-text-xs);
    font-weight: 600;
    margin-bottom: 4px;
  }
  .resize {
    position: absolute;
    top: 0;
    right: -4px;
    width: 8px;
    height: 100%;
    cursor: col-resize;
    z-index: 5;
  }
  .resize:hover,
  .resize:focus-visible {
    background: linear-gradient(90deg, transparent 3px, var(--dz-accent) 3px, var(--dz-accent) 5px, transparent 5px);
    outline: none;
  }
</style>
