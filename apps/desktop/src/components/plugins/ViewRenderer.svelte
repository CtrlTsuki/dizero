<script lang="ts">
  import type { ViewNode } from "../../lib/bindings/ViewNode";

  let {
    nodes,
    compact = false,
    onAction,
  }: { nodes: ViewNode[]; compact?: boolean; onAction: (action: string, values: Record<string, string>) => void } = $props();

  // Input values live here; the plugin only sees them when an action fires.
  let values = $state<Record<string, string>>({});

  function fire(action: string, extra: Record<string, string> = {}) {
    onAction(action, { ...$state.snapshot(values), ...extra });
    values = {};
  }
</script>

<div class="view" class:compact>
  {#each nodes as n, i (i)}
    {#if n.type === "heading"}
      <h3>{n.text}</h3>
    {:else if n.type === "text"}
      <p class="text" data-tone={n.tone}>{n.text}</p>
    {:else if n.type === "stat"}
      <div class="stat">
        <span class="stat-label">{n.label}</span>
        <span class="stat-value num">{n.value}</span>
        {#if n.hint}<span class="stat-hint">{n.hint}</span>{/if}
      </div>
    {:else if n.type === "progress"}
      <div class="progress">
        <span class="progress-label">{n.label}</span>
        <span class="bar" role="meter" aria-label={n.label} aria-valuemin={0} aria-valuemax={1} aria-valuenow={n.value}>
          <span style:width="{Math.round(n.value * 100)}%"></span>
        </span>
      </div>
    {:else if n.type === "list"}
      <ul class="list">
        {#each n.items as item (item.id)}
          <li>
            <div class="item-main">
              <span class="item-text">{item.text}</span>
              {#if item.meta}<span class="item-meta">{item.meta}</span>{/if}
            </div>
            {#if item.action}
              <button class="btn btn-sm btn-ghost" onclick={() => fire(item.action ?? "", { itemId: item.id })}>
                {item.actionLabel ?? "Select"}
              </button>
            {/if}
          </li>
        {/each}
      </ul>
    {:else if n.type === "button"}
      <button
        class="btn btn-sm"
        class:btn-primary={n.variant === "primary"}
        class:btn-ghost={n.variant === "ghost"}
        class:btn-danger={n.variant === "danger"}
        onclick={() => fire(n.action)}>{n.label}</button
      >
    {:else if n.type === "input"}
      <input
        class="field"
        placeholder={n.placeholder}
        maxlength={n.maxLength}
        aria-label={n.placeholder || n.id}
        value={values[n.id] ?? n.value}
        oninput={(e) => (values[n.id] = (e.currentTarget as HTMLInputElement).value)}
        onkeydown={(e) => {
          if (e.key === "Enter") {
            const next = nodes.slice(i + 1).find((x) => x.type === "button");
            if (next && next.type === "button") fire(next.action);
          }
        }}
      />
    {:else if n.type === "divider"}
      <hr />
    {:else if n.type === "empty"}
      <p class="empty">{n.text}</p>
    {/if}
  {/each}
</div>

<style>
  .view {
    display: flex;
    flex-direction: column;
    gap: 10px;
    align-items: stretch;
  }
  .view.compact {
    gap: 6px;
    font-size: var(--dz-text-sm);
  }
  h3 {
    font-size: var(--dz-text-md);
  }
  .text {
    color: var(--dz-text-primary);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .text[data-tone="muted"] {
    color: var(--dz-text-muted);
    font-size: var(--dz-text-sm);
  }
  .text[data-tone="accent"] {
    color: var(--dz-accent-hover);
  }
  .text[data-tone="success"] {
    color: var(--dz-success);
  }
  .text[data-tone="warning"] {
    color: var(--dz-warning);
  }
  .text[data-tone="danger"] {
    color: var(--dz-danger);
  }
  .stat {
    display: grid;
    grid-template-columns: 1fr auto;
    align-items: baseline;
    column-gap: 8px;
  }
  .stat-label {
    color: var(--dz-text-secondary);
    font-size: var(--dz-text-sm);
  }
  .stat-value {
    font-size: var(--dz-text-lg);
    font-weight: 650;
  }
  .stat-hint {
    grid-column: 1 / -1;
    color: var(--dz-text-muted);
    font-size: var(--dz-text-xs);
  }
  .progress {
    display: grid;
    gap: 4px;
  }
  .progress-label {
    font-size: var(--dz-text-sm);
    color: var(--dz-text-secondary);
  }
  .bar {
    display: block;
    height: 6px;
    border-radius: 3px;
    background: var(--dz-background);
    overflow: hidden;
  }
  .bar span {
    display: block;
    height: 100%;
    border-radius: 3px;
    background: var(--dz-accent);
  }
  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 6px;
  }
  .list li {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 8px 10px;
    border: 1px solid var(--dz-border);
    border-radius: var(--dz-r-control);
    background: var(--dz-surface-secondary);
  }
  .item-main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .item-text {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    font-size: var(--dz-text-sm);
  }
  .item-meta {
    color: var(--dz-text-muted);
    font-size: var(--dz-text-xs);
  }
  hr {
    width: 100%;
    margin: 2px 0;
    border: 0;
    border-top: 1px solid var(--dz-border);
  }
  .empty {
    color: var(--dz-text-muted);
    font-size: var(--dz-text-sm);
  }
  .view .btn {
    align-self: flex-start;
  }
</style>
