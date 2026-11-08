<script lang="ts">
  import { Hash, Megaphone, PanelLeft, PanelRight, Search, X } from "@lucide/svelte";
  import { workspace } from "../../lib/stores/workspace.svelte";
  import { plugins } from "../../lib/stores/plugins.svelte";
  import { app } from "../../lib/stores/app.svelte";
  import { tooltip } from "../../lib/tooltip";

  let {
    sidebarCollapsed,
    dockOpen,
    onToggleSidebar,
    onToggleDock,
  }: { sidebarCollapsed: boolean; dockOpen: boolean; onToggleSidebar: () => void; onToggleDock: () => void } = $props();

  let query = $state("");
  let timer: ReturnType<typeof setTimeout> | undefined;
  const conv = $derived(workspace.activeConversation);

  function openSearch() {
    plugins.dockTab = "search";
    if (!dockOpen) onToggleDock();
  }

  function onInput() {
    clearTimeout(timer);
    timer = setTimeout(() => {
      void workspace.runSearch(query);
      if (query.trim()) openSearch();
    }, 220);
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === "Enter") {
      clearTimeout(timer);
      void workspace.runSearch(query);
      openSearch();
    } else if (e.key === "Escape" && query) {
      e.stopPropagation();
      clear();
    }
  }

  function clear() {
    query = "";
    void workspace.runSearch("");
    if (plugins.dockTab === "search") plugins.dockTab = "members";
  }
</script>

<header class="header">
  <button
    class="icon-btn"
    aria-label={sidebarCollapsed ? "Show channel list" : "Hide channel list"}
    aria-pressed={!sidebarCollapsed}
    use:tooltip={{ text: sidebarCollapsed ? "Show channels" : "Hide channels", side: "bottom" }}
    onclick={onToggleSidebar}
  >
    <PanelLeft size={18} strokeWidth={1.8} />
  </button>

  {#if conv}
    <div class="title">
      <span class="glyph" aria-hidden="true">
        {#if conv.kind === "announcement"}<Megaphone size={18} strokeWidth={1.9} />{:else}<Hash size={18} strokeWidth={1.9} />{/if}
      </span>
      <h1>{conv.name}</h1>
      {#if conv.topic}
        <span class="divider" aria-hidden="true"></span>
        <p class="topic" title={conv.topic}>{conv.topic}</p>
      {/if}
    </div>
  {:else}
    <div class="title"><div class="skeleton" style:width="140px" style:height="14px"></div></div>
  {/if}

  <div class="tools">
    {#if workspace.capabilities.search}
      <label class="search">
        <Search size={14} strokeWidth={2} />
        <span class="sr-only">Search messages</span>
        <input
          id="dz-search"
          type="search"
          placeholder="Search"
          autocomplete="off"
          spellcheck="false"
          bind:value={query}
          oninput={onInput}
          onkeydown={onKey}
          onfocus={() => query && openSearch()}
        />
        {#if query}
          <button class="clear" aria-label="Clear search" onclick={clear}><X size={13} strokeWidth={2.2} /></button>
        {:else}
          <kbd aria-hidden="true">Ctrl F</kbd>
        {/if}
      </label>
    {/if}
    <button
      class="icon-btn"
      aria-label={dockOpen ? "Hide side panel" : "Show side panel"}
      aria-pressed={dockOpen}
      use:tooltip={{ text: dockOpen ? "Hide members and panels" : "Show members and panels", side: "bottom" }}
      onclick={onToggleDock}
    >
      <PanelRight size={18} strokeWidth={1.8} />
    </button>
    <button class="switcher" onclick={() => (app.quickSwitcherOpen = true)} aria-label="Open quick switcher">
      <kbd>Ctrl K</kbd>
    </button>
  </div>
</header>

<style>
  .header {
    display: flex;
    align-items: center;
    gap: 8px;
    height: var(--dz-header);
    padding: 0 10px 0 8px;
    border-bottom: 1px solid var(--dz-border);
    flex: none;
    background: var(--dz-surface-secondary);
  }
  .title {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    padding-left: 2px;
  }
  .glyph {
    display: inline-flex;
    color: var(--dz-text-muted);
  }
  h1 {
    font-size: var(--dz-text-md);
    font-weight: 640;
    white-space: nowrap;
  }
  .divider {
    width: 1px;
    height: 18px;
    background: var(--dz-border);
    margin: 0 4px;
  }
  .topic {
    min-width: 0;
    color: var(--dz-text-muted);
    font-size: var(--dz-text-sm);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .tools {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .search {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 220px;
    height: 30px;
    padding: 0 6px 0 9px;
    border: 1px solid var(--dz-border);
    border-radius: var(--dz-r-control);
    background: var(--dz-input);
    color: var(--dz-text-muted);
    transition: border-color var(--dz-fast) var(--dz-ease), width var(--dz-base) var(--dz-ease);
  }
  .search:focus-within {
    border-color: var(--dz-accent);
    box-shadow: 0 0 0 3px var(--dz-accent-soft);
    width: 260px;
  }
  .search input {
    flex: 1;
    min-width: 0;
    border: 0;
    background: none;
    outline: none;
    font-size: var(--dz-text-sm);
    color: var(--dz-text-primary);
  }
  .search input::placeholder {
    color: var(--dz-text-muted);
  }
  .search input::-webkit-search-cancel-button {
    display: none;
  }
  .clear {
    display: inline-grid;
    place-items: center;
    width: 20px;
    height: 20px;
    border-radius: 4px;
    color: var(--dz-text-muted);
  }
  .clear:hover {
    color: var(--dz-text-primary);
    background: var(--dz-hover);
  }
  .switcher {
    display: inline-flex;
    margin-left: 2px;
    padding: 4px;
    border-radius: var(--dz-r-control);
  }
  .switcher:hover kbd {
    color: var(--dz-text-primary);
    border-color: var(--dz-text-muted);
  }
  @media (max-width: 1100px) {
    .topic,
    .divider,
    .switcher {
      display: none;
    }
  }
</style>
