<script lang="ts">
  import { SearchX } from "@lucide/svelte";
  import { workspace } from "../../lib/stores/workspace.svelte";
  import { clockTime, dayLabel } from "../../lib/format";

  /** Split an FTS snippet on its private-use markers into plain/highlight runs. */
  function runs(snippet: string): { hit: boolean; text: string }[] {
    const out: { hit: boolean; text: string }[] = [];
    let hit = false;
    let buf = "";
    for (const ch of snippet) {
      if (ch === "" || ch === "") {
        if (buf) out.push({ hit, text: buf });
        buf = "";
        hit = ch === "";
      } else buf += ch;
    }
    if (buf) out.push({ hit, text: buf });
    return out;
  }
</script>

<div class="results" aria-live="polite">
  {#if workspace.searching && workspace.searchResults.length === 0}
    <p class="state">Searching…</p>
  {:else if workspace.searchQuery && workspace.searchResults.length === 0}
    <div class="empty">
      <SearchX size={22} strokeWidth={1.6} />
      <p>No messages match “{workspace.searchQuery}”.</p>
      <p class="muted">Search covers every conversation stored on this computer.</p>
    </div>
  {:else}
    <p class="state num">{workspace.searchResults.length}{workspace.searchResults.length === 50 ? "+" : ""} result{workspace.searchResults.length === 1 ? "" : "s"}</p>
    <ul>
      {#each workspace.searchResults as hit (hit.message.id)}
        <li>
          <button onclick={() => workspace.openHit(hit)}>
            <span class="meta">
              <span class="where">#{hit.conversationName}</span>
              <span class="when num">{dayLabel(hit.message.createdAt)}, {clockTime(hit.message.createdAt)}</span>
            </span>
            <span class="author">{hit.message.author.displayName}</span>
            <span class="snippet">
              {#each runs(hit.snippet) as r, i (i)}{#if r.hit}<mark>{r.text}</mark>{:else}{r.text}{/if}{/each}
            </span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .results {
    padding: 8px;
  }
  .state {
    padding: 6px 8px 8px;
    color: var(--dz-text-muted);
    font-size: var(--dz-text-xs);
    font-weight: 600;
  }
  .empty {
    display: grid;
    justify-items: center;
    gap: 6px;
    padding: 36px 16px;
    text-align: center;
    color: var(--dz-text-secondary);
    font-size: var(--dz-text-sm);
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 6px;
  }
  li button {
    display: grid;
    gap: 3px;
    width: 100%;
    padding: 9px 10px;
    border: 1px solid var(--dz-border);
    border-radius: var(--dz-r-control);
    background: var(--dz-surface-secondary);
    text-align: left;
  }
  li button:hover {
    border-color: var(--dz-border-strong);
  }
  .meta {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    font-size: var(--dz-text-xs);
    color: var(--dz-text-muted);
  }
  .where {
    font-weight: 600;
  }
  .author {
    font-size: var(--dz-text-sm);
    font-weight: 600;
  }
  .snippet {
    color: var(--dz-text-secondary);
    font-size: var(--dz-text-sm);
    overflow-wrap: anywhere;
  }
  mark {
    background: color-mix(in srgb, var(--dz-warning) 25%, transparent);
    color: var(--dz-text-primary);
    border-radius: 3px;
    padding: 0 1px;
  }
</style>
