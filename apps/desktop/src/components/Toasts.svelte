<script lang="ts">
  import { X, Check, Info, CircleExclamationPoint, Puzzle, AtSign } from "@lucide/svelte";
  import { toasts } from "../lib/stores/toasts.svelte";

  const icons = { info: Info, success: Check, error: CircleExclamationPoint, plugin: Puzzle, message: AtSign } as const;
</script>

<div class="stack" role="region" aria-label="Notifications">
  {#each toasts.items as t (t.id)}
    {@const Icon = icons[t.kind]}
    <div class="toast" data-kind={t.kind} role={t.kind === "error" ? "alert" : "status"}>
      <span class="icon"><Icon size={16} strokeWidth={2} /></span>
      <div class="text">
        <p class="title">{t.title}</p>
        {#if t.body}<p class="body">{t.body}</p>{/if}
        {#if t.action}
          <button
            class="action"
            onclick={() => {
              t.action?.run();
              toasts.dismiss(t.id);
            }}>{t.action.label}</button
          >
        {/if}
      </div>
      <button class="close" aria-label="Dismiss notification" onclick={() => toasts.dismiss(t.id)}><X size={14} /></button>
    </div>
  {/each}
</div>

<style>
  .stack {
    position: fixed;
    right: 16px;
    bottom: 16px;
    z-index: 400;
    display: flex;
    flex-direction: column;
    gap: 8px;
    width: 340px;
    pointer-events: none;
  }
  .toast {
    display: flex;
    gap: 10px;
    padding: 12px 10px 12px 12px;
    border-radius: var(--dz-r-float);
    background: var(--dz-surface-elevated);
    box-shadow: var(--dz-shadow-float);
    pointer-events: auto;
    animation: toast-in var(--dz-base) var(--dz-ease);
  }
  @keyframes toast-in {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
  }
  .icon {
    display: inline-flex;
    padding-top: 1px;
    color: var(--dz-text-secondary);
  }
  [data-kind="success"] .icon {
    color: var(--dz-success);
  }
  [data-kind="error"] .icon {
    color: var(--dz-danger);
  }
  [data-kind="plugin"] .icon,
  [data-kind="message"] .icon {
    color: var(--dz-accent-hover);
  }
  .text {
    flex: 1;
    min-width: 0;
  }
  .title {
    font-size: var(--dz-text-sm);
    font-weight: 620;
  }
  .body {
    margin-top: 2px;
    color: var(--dz-text-secondary);
    font-size: var(--dz-text-sm);
    white-space: pre-line;
    overflow-wrap: anywhere;
  }
  .action {
    margin-top: 6px;
    color: var(--dz-accent-hover);
    font-size: var(--dz-text-sm);
    font-weight: 600;
  }
  .action:hover {
    text-decoration: underline;
  }
  .close {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    border-radius: 4px;
    color: var(--dz-text-muted);
  }
  .close:hover {
    background: var(--dz-hover);
    color: var(--dz-text-primary);
  }
</style>
