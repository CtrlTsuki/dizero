<script lang="ts">
  import { onMount } from "svelte";
  import { Hash, Megaphone, Puzzle, Gauge, CodeXml, Settings, Search } from "@lucide/svelte";
  import { app, type Route } from "../lib/stores/app.svelte";
  import { workspace } from "../lib/stores/workspace.svelte";

  type Entry =
    | { kind: "conversation"; id: string; label: string; detail: string; announcement: boolean; unread: number }
    | { kind: "page"; route: Route; label: string; detail: string };

  let query = $state("");
  let selected = $state(0);
  let input = $state<HTMLInputElement>();

  const pages: Entry[] = [
    { kind: "page", route: "plugins", label: "Plugins", detail: "Install and manage plugins" },
    { kind: "page", route: "performance", label: "Performance Center", detail: "Memory, CPU and modes" },
    { kind: "page", route: "developer", label: "Developer Center", detail: "Logs, SDK and validation" },
    { kind: "page", route: "settings", label: "Settings", detail: "Appearance, data and more" },
  ];

  /** Subsequence match score: lower is better, -1 = no match. */
  function score(text: string, q: string): number {
    const t = text.toLowerCase();
    const s = q.toLowerCase().replace(/^#/, "");
    if (!s) return 0;
    const direct = t.indexOf(s);
    if (direct >= 0) return direct;
    let ti = 0;
    let gaps = 0;
    for (const ch of s) {
      const found = t.indexOf(ch, ti);
      if (found < 0) return -1;
      gaps += found - ti;
      ti = found + 1;
    }
    return 100 + gaps;
  }

  const entries = $derived.by(() => {
    const convs: Entry[] = workspace.conversations.map((c) => ({
      kind: "conversation" as const,
      id: c.id,
      label: c.name,
      detail: workspace.communities.find((x) => x.id === c.communityId)?.name ?? "",
      announcement: c.kind === "announcement",
      unread: c.unreadCount,
    }));
    const all = [...convs, ...pages];
    if (!query.trim()) {
      return [...convs.filter((c) => c.kind === "conversation" && c.unread > 0), ...convs.filter((c) => c.kind === "conversation" && c.unread === 0)]
        .slice(0, 8)
        .concat(pages);
    }
    return all
      .map((e) => ({ e, s: Math.min(...[score(e.label, query), score(e.detail, query) + 50].filter((x) => x >= 0), Infinity) }))
      .filter((x) => Number.isFinite(x.s))
      .sort((a, b) => a.s - b.s)
      .slice(0, 12)
      .map((x) => x.e);
  });

  $effect(() => {
    void query;
    selected = 0;
  });

  onMount(() => input?.focus());

  function choose(e: Entry | undefined) {
    if (!e) return;
    app.quickSwitcherOpen = false;
    if (e.kind === "conversation") {
      app.go("workspace");
      void workspace.selectConversation(e.id);
    } else app.go(e.route);
  }

  function onKey(ev: KeyboardEvent) {
    if (ev.key === "ArrowDown" || ev.key === "ArrowUp") {
      ev.preventDefault();
      const d = ev.key === "ArrowDown" ? 1 : -1;
      selected = (selected + d + entries.length) % Math.max(entries.length, 1);
    } else if (ev.key === "Enter") {
      ev.preventDefault();
      choose(entries[selected]);
    } else if (ev.key === "Escape") {
      ev.preventDefault();
      ev.stopPropagation();
      app.quickSwitcherOpen = false;
    }
  }

  const pageIcon = { plugins: Puzzle, performance: Gauge, developer: CodeXml, settings: Settings, workspace: Hash } as const;
</script>

<div class="backdrop" role="presentation" onmousedown={(e) => e.target === e.currentTarget && (app.quickSwitcherOpen = false)}>
  <div class="switcher" role="dialog" aria-modal="true" aria-label="Quick switcher">
    <label class="search">
      <Search size={17} strokeWidth={1.9} />
      <input
        bind:this={input}
        bind:value={query}
        onkeydown={onKey}
        placeholder="Jump to a channel or page"
        aria-label="Jump to a channel or page"
        role="combobox"
        aria-expanded="true"
        aria-controls="qs-list"
        aria-activedescendant={entries[selected] ? `qs-${selected}` : undefined}
        autocomplete="off"
        spellcheck="false"
      />
    </label>
    <ul id="qs-list" role="listbox">
      {#each entries as e, i (e.kind === "conversation" ? e.id : e.route)}
        <li
          id="qs-{i}"
          role="option"
          aria-selected={i === selected}
          class:sel={i === selected}
          onmousemove={() => (selected = i)}
          onmousedown={(ev) => {
            ev.preventDefault();
            choose(e);
          }}
        >
          <span class="icon">
            {#if e.kind === "conversation"}
              {#if e.announcement}<Megaphone size={16} strokeWidth={1.8} />{:else}<Hash size={16} strokeWidth={1.8} />{/if}
            {:else}
              {@const Icon = pageIcon[e.route]}
              <Icon size={16} strokeWidth={1.8} />
            {/if}
          </span>
          <span class="label" class:unread={e.kind === "conversation" && e.unread > 0}>{e.label}</span>
          <span class="detail">{e.detail}</span>
        </li>
      {:else}
        <li class="none">Nothing matches “{query}”.</li>
      {/each}
    </ul>
    <div class="foot">
      <span><kbd>↑</kbd> <kbd>↓</kbd> move</span>
      <span><kbd>Enter</kbd> open</span>
      <span><kbd>Esc</kbd> close</span>
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 300;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding-top: 14vh;
    background: rgba(5, 5, 8, 0.55);
  }
  .switcher {
    width: 560px;
    max-width: calc(100vw - 48px);
    border-radius: 12px;
    background: var(--dz-surface-elevated);
    box-shadow: var(--dz-shadow-float);
    overflow: hidden;
    animation: qs-in var(--dz-base) var(--dz-ease);
  }
  @keyframes qs-in {
    from {
      opacity: 0;
      transform: translateY(-6px);
    }
  }
  .search {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 16px;
    height: 54px;
    border-bottom: 1px solid var(--dz-border);
    color: var(--dz-text-muted);
  }
  .search input {
    flex: 1;
    border: 0;
    outline: none;
    background: none;
    font-size: var(--dz-text-lg);
    color: var(--dz-text-primary);
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 6px;
    max-height: 380px;
    overflow-y: auto;
  }
  li {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 38px;
    padding: 0 10px;
    border-radius: var(--dz-r-control);
    cursor: pointer;
    color: var(--dz-text-secondary);
  }
  li.sel {
    background: var(--dz-active);
    color: var(--dz-text-primary);
  }
  .icon {
    display: inline-flex;
    color: var(--dz-text-muted);
  }
  .label {
    font-weight: 560;
  }
  .label.unread {
    color: var(--dz-text-primary);
    font-weight: 650;
  }
  .detail {
    margin-left: auto;
    color: var(--dz-text-muted);
    font-size: var(--dz-text-xs);
  }
  .none {
    justify-content: center;
    color: var(--dz-text-muted);
    cursor: default;
  }
  .foot {
    display: flex;
    gap: 16px;
    padding: 9px 16px;
    border-top: 1px solid var(--dz-border);
    color: var(--dz-text-muted);
    font-size: var(--dz-text-xs);
  }
</style>
