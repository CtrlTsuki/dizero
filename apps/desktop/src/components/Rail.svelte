<script lang="ts">
  import { Puzzle, Gauge, CodeXml, Settings } from "@lucide/svelte";
  import Logo from "./Logo.svelte";
  import { app, type Route } from "../lib/stores/app.svelte";
  import { workspace } from "../lib/stores/workspace.svelte";
  import { tooltip } from "../lib/tooltip";

  const tools: { route: Route; label: string; icon: typeof Puzzle; shortcut?: string }[] = [
    { route: "plugins", label: "Plugins", icon: Puzzle },
    { route: "performance", label: "Performance Center", icon: Gauge },
    { route: "developer", label: "Developer Center", icon: CodeXml },
    { route: "settings", label: "Settings (Ctrl+,)", icon: Settings },
  ];

  function openCommunity(id: string) {
    app.go("workspace");
    void workspace.selectCommunity(id);
  }
</script>

<nav class="rail" aria-label="Communities and tools">
  <button
    class="home"
    class:active={app.route === "workspace"}
    aria-label="Workspace"
    use:tooltip={{ text: "Workspace", side: "right" }}
    onclick={() => app.go("workspace")}
  >
    <Logo size={34} />
  </button>

  <div class="seam" role="presentation"></div>

  <ul class="communities">
    {#each workspace.communities as c (c.id)}
      {@const counts = workspace.communityUnread(c.id)}
      {@const active = app.route === "workspace" && workspace.activeCommunityId === c.id}
      <li>
        <button
          class="tile"
          class:active
          style:--tint={c.accent}
          aria-label="{c.name}{counts.mentions ? `, ${counts.mentions} mentions` : counts.unread ? ', unread messages' : ''}"
          aria-current={active ? "page" : undefined}
          use:tooltip={{ text: c.name, side: "right" }}
          onclick={() => openCommunity(c.id)}
        >
          {c.initials}
        </button>
        {#if counts.mentions > 0}
          <span class="mention num" aria-hidden="true">{counts.mentions}</span>
        {:else if counts.unread > 0 && !active}
          <span class="unread led" data-tone="accent" aria-hidden="true"></span>
        {/if}
      </li>
    {/each}
  </ul>

  <ul class="tools">
    {#each tools as t (t.route)}
      <li>
        <button
          class="tool"
          class:active={app.route === t.route}
          aria-label={t.label}
          aria-current={app.route === t.route ? "page" : undefined}
          use:tooltip={{ text: t.label, side: "right" }}
          onclick={() => app.go(t.route)}
        >
          <t.icon size={19} strokeWidth={1.75} />
        </button>
      </li>
    {/each}
  </ul>
</nav>

<style>
  .rail {
    width: var(--dz-rail);
    flex: none;
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 12px 0;
    background: var(--dz-background);
    border-right: 1px solid var(--dz-border);
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
  }
  li {
    position: relative;
  }
  .home {
    display: grid;
    place-items: center;
    width: 44px;
    height: 44px;
    border-radius: var(--dz-r-tile);
    transition: background-color var(--dz-fast) var(--dz-ease);
  }
  .home:hover,
  .home.active {
    background: var(--dz-surface);
  }
  .seam {
    width: 28px;
    height: 1px;
    margin: 10px 0;
    background: var(--dz-border);
  }
  .communities {
    flex: 1;
    overflow-y: auto;
    padding: 2px 0;
    width: 100%;
  }
  .tile {
    display: grid;
    place-items: center;
    width: 44px;
    height: 44px;
    border-radius: var(--dz-r-tile);
    background: var(--dz-surface);
    box-shadow: inset 0 0 0 1px var(--dz-border);
    color: var(--dz-text-secondary);
    font-size: var(--dz-text-sm);
    font-weight: 650;
    letter-spacing: 0.01em;
    transition: background-color var(--dz-fast) var(--dz-ease), color var(--dz-fast) var(--dz-ease),
      box-shadow var(--dz-fast) var(--dz-ease);
  }
  .tile:hover {
    color: var(--dz-text-primary);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--tint) 55%, transparent);
  }
  .tile.active {
    background: color-mix(in srgb, var(--tint) 20%, var(--dz-surface));
    color: var(--dz-text-primary);
    box-shadow: inset 0 0 0 1.5px color-mix(in srgb, var(--tint) 75%, transparent);
  }
  /* Active marker: a short bar on the rail's left edge, like a selected channel strip. */
  li:has(.tile.active)::before,
  li:has(.tool.active)::before {
    content: "";
    position: absolute;
    left: -10px;
    top: 50%;
    width: 3px;
    height: 20px;
    transform: translateY(-50%);
    border-radius: 0 3px 3px 0;
    background: var(--dz-text-primary);
  }
  .unread {
    position: absolute;
    top: -1px;
    right: -1px;
    outline: 3px solid var(--dz-background);
  }
  .mention {
    position: absolute;
    right: -5px;
    bottom: -3px;
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
    outline: 3px solid var(--dz-background);
  }
  .tools {
    padding-top: 10px;
    border-top: 1px solid var(--dz-border);
    width: 40px;
  }
  .tool {
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    border-radius: 10px;
    color: var(--dz-text-muted);
    transition: background-color var(--dz-fast) var(--dz-ease), color var(--dz-fast) var(--dz-ease);
  }
  .tool:hover {
    color: var(--dz-text-primary);
    background: var(--dz-surface);
  }
  .tool.active {
    color: var(--dz-accent-hover);
    background: var(--dz-accent-soft);
  }
</style>
