<script lang="ts">
  import { Ellipsis, FileText } from "@lucide/svelte";
  import Avatar from "../Avatar.svelte";
  import RichText from "./RichText.svelte";
  import type { Message } from "../../lib/bindings/Message";
  import { clockTime, fullTimestamp, formatBytes } from "../../lib/format";
  import { menu, type MenuItem } from "../../lib/stores/menu.svelte";
  import { workspace } from "../../lib/stores/workspace.svelte";
  import { toasts } from "../../lib/stores/toasts.svelte";

  let {
    message,
    continued,
    selfName,
    showAvatars,
    highlighted,
  }: { message: Message; continued: boolean; selfName: string; showAvatars: boolean; highlighted: boolean } = $props();

  let row = $state<HTMLElement>();

  $effect(() => {
    if (highlighted) row?.scrollIntoView({ block: "center" });
  });

  function copy(text: string, what: string) {
    navigator.clipboard.writeText(text).then(
      () => toasts.success(`${what} copied`),
      () => toasts.error(`Couldn't copy ${what.toLowerCase()}`),
    );
  }

  function items(): MenuItem[] {
    const list: MenuItem[] = [
      { label: "Copy text", run: () => copy(message.content, "Text") },
      { label: "Copy message ID", run: () => copy(message.id, "Message ID") },
    ];
    if (message.author.isSelf && workspace.capabilities.deleteOwnMessages) {
      list.push({ label: "Delete message", danger: true, run: () => void workspace.remove(message.id) });
    }
    return list;
  }

  function onContext(e: MouseEvent) {
    e.preventDefault();
    menu.show(e.clientX, e.clientY, items());
  }
</script>

<article
  bind:this={row}
  class="row"
  class:continued
  class:mentioned={message.mentionsSelf}
  class:highlighted
  oncontextmenu={onContext}
  aria-label="{message.author.displayName} at {clockTime(message.createdAt)}"
>
  <div class="gutter">
    {#if !continued && showAvatars}
      <Avatar user={message.author} size={36} />
    {:else}
      <time class="hover-time num" datetime={new Date(message.createdAt).toISOString()} title={fullTimestamp(message.createdAt)}
        >{clockTime(message.createdAt)}</time
      >
    {/if}
  </div>
  <div class="body">
    {#if !continued}
      <header>
        <span class="author" style:--tint={message.author.avatarColor}>{message.author.displayName}</span>
        {#if message.author.isBot}<span class="bot">Bot</span>{/if}
        <time class="num" datetime={new Date(message.createdAt).toISOString()} title={fullTimestamp(message.createdAt)}
          >{clockTime(message.createdAt)}</time
        >
      </header>
    {/if}
    <RichText text={message.content} {selfName} />
    {#each message.attachments as a (a.id)}
      <div class="file" title={a.url ? a.name : "Demo attachment: name and size only, there is no file to download"}>
        <FileText size={20} strokeWidth={1.6} />
        <div class="file-meta">
          <span class="file-name">{a.name}</span>
          <span class="file-size num">{formatBytes(a.sizeBytes)}{a.url ? "" : ", demo file (no download)"}</span>
        </div>
      </div>
    {/each}
  </div>
  <div class="actions">
    <button
      class="icon-btn"
      aria-label="More actions for this message"
      onclick={(e) => menu.showAt(e.currentTarget as HTMLElement, items())}
    >
      <Ellipsis size={16} />
    </button>
  </div>
</article>

<style>
  .row {
    position: relative;
    display: grid;
    grid-template-columns: 56px 1fr;
    padding: 2px 48px 2px 16px;
    margin-top: var(--dz-group-gap);
    content-visibility: auto;
    contain-intrinsic-size: auto 52px;
  }
  .row.continued {
    margin-top: var(--dz-row-gap);
    contain-intrinsic-size: auto 26px;
  }
  .row:hover,
  .row:focus-within {
    background: var(--dz-hover);
  }
  .row.mentioned {
    background: color-mix(in srgb, var(--dz-warning) 8%, transparent);
  }
  .row.mentioned:hover {
    background: color-mix(in srgb, var(--dz-warning) 12%, transparent);
  }
  .row.highlighted {
    background: var(--dz-accent-soft);
  }
  .gutter {
    display: flex;
    justify-content: flex-start;
    padding-top: 2px;
  }
  .hover-time {
    width: 40px;
    padding-top: 2px;
    color: var(--dz-text-muted);
    font-size: 11px;
    text-align: right;
    opacity: 0;
  }
  .row:hover .hover-time,
  .row:focus-within .hover-time {
    opacity: 1;
  }
  .body {
    min-width: 0;
  }
  header {
    display: flex;
    align-items: baseline;
    gap: 8px;
    margin-bottom: 1px;
  }
  .author {
    font-weight: 620;
    color: color-mix(in srgb, var(--tint) 35%, var(--dz-text-primary));
  }
  .bot {
    padding: 0 5px;
    border-radius: 4px;
    background: var(--dz-accent-soft);
    color: var(--dz-accent-hover);
    font-size: 10.5px;
    font-weight: 650;
    line-height: 16px;
  }
  header time {
    color: var(--dz-text-muted);
    font-size: var(--dz-text-xs);
  }
  .file {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    margin-top: 6px;
    padding: 10px 14px 10px 12px;
    min-width: 260px;
    max-width: 420px;
    border: 1px solid var(--dz-border);
    border-radius: var(--dz-r-control);
    background: var(--dz-surface);
    color: var(--dz-text-muted);
  }
  .file-meta {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .file-name {
    color: var(--dz-text-primary);
    font-weight: 560;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .file-size {
    font-size: var(--dz-text-xs);
  }
  .actions {
    position: absolute;
    top: -6px;
    right: 12px;
    display: none;
    border: 1px solid var(--dz-border);
    border-radius: var(--dz-r-control);
    background: var(--dz-surface-elevated);
  }
  .actions .icon-btn {
    width: 28px;
    height: 28px;
  }
  .row:hover .actions,
  .row:focus-within .actions {
    display: flex;
  }
</style>
