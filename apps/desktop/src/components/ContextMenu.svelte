<script lang="ts">
  import { tick } from "svelte";
  import { menu } from "../lib/stores/menu.svelte";
  import { pendingLink, openPendingLink } from "../lib/stores/links.svelte";
  import Modal from "./Modal.svelte";

  let el = $state<HTMLDivElement>();
  let pos = $state({ x: 0, y: 0 });

  $effect(() => {
    if (!menu.open) return;
    void menu.x;
    void menu.y;
    tick().then(() => {
      if (!el) return;
      const r = el.getBoundingClientRect();
      pos = {
        x: Math.min(menu.x, window.innerWidth - r.width - 8),
        y: Math.min(menu.y, window.innerHeight - r.height - 8),
      };
      el.querySelector<HTMLButtonElement>("button:not(:disabled)")?.focus();
    });
  });

  function onKey(e: KeyboardEvent) {
    const items = [...(el?.querySelectorAll<HTMLButtonElement>("button:not(:disabled)") ?? [])];
    const i = items.indexOf(document.activeElement as HTMLButtonElement);
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      const next = items[(i + (e.key === "ArrowDown" ? 1 : -1) + items.length) % items.length];
      next?.focus();
    } else if (e.key === "Escape" || e.key === "Tab") {
      e.preventDefault();
      e.stopPropagation();
      menu.close();
    }
  }
</script>

<svelte:window onmousedown={(e) => menu.open && el && !el.contains(e.target as Node) && menu.close()} onblur={() => menu.close()} />

{#if menu.open}
  <div class="menu" bind:this={el} role="menu" tabindex="-1" style:transform="translate({pos.x}px, {pos.y}px)" onkeydown={onKey}>
    {#each menu.items as item, i (i)}
      <button
        role="menuitem"
        class:danger={item.danger}
        disabled={item.disabled}
        onclick={() => {
          menu.close();
          item.run();
        }}>{item.label}</button
      >
    {/each}
  </div>
{/if}

{#if pendingLink.url}
  <Modal title="Open this link?" onClose={() => (pendingLink.url = null)}>
    <p class="link-note">It will open in your default browser.</p>
    <p class="link-url">{pendingLink.url}</p>
    {#snippet footer()}
      <button class="btn btn-ghost" onclick={() => (pendingLink.url = null)}>Cancel</button>
      <button class="btn btn-primary" onclick={openPendingLink}>Open link</button>
    {/snippet}
  </Modal>
{/if}

<style>
  .menu {
    position: fixed;
    top: 0;
    left: 0;
    z-index: 500;
    min-width: 190px;
    padding: 5px;
    border-radius: var(--dz-r-float);
    background: var(--dz-surface-elevated);
    box-shadow: var(--dz-shadow-float);
  }
  button {
    display: block;
    width: 100%;
    padding: 7px 10px;
    border-radius: var(--dz-r-control);
    text-align: left;
    font-size: var(--dz-text-sm);
    color: var(--dz-text-primary);
  }
  button:hover:not(:disabled),
  button:focus-visible {
    background: var(--dz-active);
    outline: none;
  }
  button.danger {
    color: var(--dz-danger);
  }
  button:disabled {
    color: var(--dz-text-muted);
  }
  .link-note {
    color: var(--dz-text-secondary);
    font-size: var(--dz-text-sm);
    margin-bottom: 10px;
  }
  .link-url {
    padding: 10px 12px;
    border-radius: var(--dz-r-control);
    background: var(--dz-background);
    border: 1px solid var(--dz-border);
    font-size: var(--dz-text-sm);
    overflow-wrap: anywhere;
  }
</style>
