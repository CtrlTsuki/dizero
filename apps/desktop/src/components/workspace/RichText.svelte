<script lang="ts">
  import { parseMessage, type Inline } from "../../lib/markdown";
  import { confirmLink } from "../../lib/stores/links.svelte";

  let { text, selfName }: { text: string; selfName: string } = $props();
  const blocks = $derived(parseMessage(text, selfName));
</script>

{#snippet inlines(items: Inline[])}
  {#each items as i, idx (idx)}
    {#if i.t === "text"}{i.v}{:else if i.t === "bold"}<strong>{i.v}</strong>{:else if i.t === "italic"}<em>{i.v}</em>{:else if i.t === "code"}<code class="inline">{i.v}</code>{:else if i.t === "link"}<button class="link" onclick={() => confirmLink(i.v)}>{i.v}</button>{:else if i.t === "mention"}<span class="mention" class:self={i.self}>{i.v}</span>{/if}
  {/each}
{/snippet}

<div class="rich">
  {#each blocks as b, idx (idx)}
    {#if b.type === "para"}
      <p>{@render inlines(b.inlines)}</p>
    {:else if b.type === "quote"}
      <blockquote>{@render inlines(b.inlines)}</blockquote>
    {:else}
      <pre class="block" data-lang={b.lang || undefined}><code>{b.text}</code></pre>
    {/if}
  {/each}
</div>

<style>
  .rich {
    min-width: 0;
    color: var(--dz-text-primary);
    overflow-wrap: anywhere;
  }
  p {
    white-space: pre-wrap;
  }
  strong {
    font-weight: 650;
  }
  em {
    font-style: italic;
  }
  .inline {
    padding: 1px 5px;
    border-radius: 4px;
    background: var(--dz-background);
    border: 1px solid var(--dz-border);
    font-size: 0.86em;
  }
  .block {
    margin: 6px 0 2px;
    padding: 10px 12px;
    max-width: 72ch;
    overflow-x: auto;
    border-radius: var(--dz-r-control);
    background: var(--dz-background);
    border: 1px solid var(--dz-border);
    font-size: var(--dz-text-sm);
    line-height: 1.55;
    white-space: pre;
  }
  blockquote {
    margin: 2px 0;
    padding-left: 12px;
    border-left: 1px solid var(--dz-text-muted);
    color: var(--dz-text-secondary);
    white-space: pre-wrap;
  }
  .mention {
    padding: 0 3px;
    border-radius: 4px;
    background: var(--dz-accent-soft);
    color: var(--dz-accent-hover);
    font-weight: 560;
  }
  .mention.self {
    background: color-mix(in srgb, var(--dz-warning) 18%, transparent);
    color: var(--dz-warning);
  }
  .link {
    display: inline;
    color: var(--dz-accent-hover);
    text-decoration: underline;
    text-decoration-color: color-mix(in srgb, var(--dz-accent-hover) 45%, transparent);
    text-underline-offset: 2px;
    text-align: left;
    overflow-wrap: anywhere;
  }
  .link:hover {
    text-decoration-color: currentColor;
  }
</style>
