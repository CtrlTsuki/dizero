<script lang="ts">
  import { onMount, type Snippet } from "svelte";

  let {
    title,
    onClose,
    width = 460,
    children,
    footer,
  }: { title: string; onClose: () => void; width?: number; children: Snippet; footer?: Snippet } = $props();

  let dialog = $state<HTMLDivElement>();
  let previous: HTMLElement | null = null;
  const titleId = `modal-${Math.random().toString(36).slice(2, 8)}`;

  const focusables = () =>
    [...(dialog?.querySelectorAll<HTMLElement>("button, input, select, textarea, [tabindex]:not([tabindex='-1'])") ?? [])].filter(
      (el) => !el.hasAttribute("disabled"),
    );

  onMount(() => {
    previous = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    (focusables()[0] ?? dialog)?.focus();
    return () => previous?.focus();
  });

  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.stopPropagation();
      onClose();
    } else if (e.key === "Tab") {
      const els = focusables();
      const first = els[0];
      const last = els[els.length - 1];
      if (!first || !last) return;
      if (e.shiftKey && document.activeElement === first) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && document.activeElement === last) {
        e.preventDefault();
        first.focus();
      }
    }
  }
</script>

<div class="backdrop" role="presentation" onmousedown={(e) => e.target === e.currentTarget && onClose()}>
  <div
    class="dialog"
    bind:this={dialog}
    role="dialog"
    aria-modal="true"
    aria-labelledby={titleId}
    tabindex="-1"
    style:width="{width}px"
    onkeydown={onKey}
  >
    <h2 id={titleId}>{title}</h2>
    <div class="content">{@render children()}</div>
    {#if footer}<div class="footer">{@render footer()}</div>{/if}
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 200;
    display: grid;
    place-items: center;
    padding: 24px;
    background: rgba(5, 5, 8, 0.62);
    animation: dz-fade var(--dz-base) var(--dz-ease);
  }
  .dialog {
    max-width: 100%;
    max-height: calc(100vh - 48px);
    display: flex;
    flex-direction: column;
    border-radius: 12px;
    background: var(--dz-surface-elevated);
    box-shadow: var(--dz-shadow-float);
    animation: dz-rise var(--dz-base) var(--dz-ease);
  }
  h2 {
    padding: 18px 20px 4px;
  }
  .content {
    padding: 8px 20px 16px;
    overflow-y: auto;
  }
  .footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 12px 20px;
    border-top: 1px solid var(--dz-border);
  }
  @keyframes dz-fade {
    from {
      opacity: 0;
    }
  }
  @keyframes dz-rise {
    from {
      opacity: 0;
      transform: translateY(6px) scale(0.985);
    }
  }
</style>
