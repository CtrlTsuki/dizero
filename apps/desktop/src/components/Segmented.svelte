<script lang="ts" generics="T extends string">
  let {
    value,
    options,
    label,
    onchange,
  }: { value: T; options: { value: T; label: string }[]; label: string; onchange: (v: T) => void } = $props();

  function onKey(e: KeyboardEvent) {
    if (e.key !== "ArrowRight" && e.key !== "ArrowLeft") return;
    e.preventDefault();
    const i = options.findIndex((o) => o.value === value);
    const next = options[(i + (e.key === "ArrowRight" ? 1 : -1) + options.length) % options.length];
    if (next) onchange(next.value);
  }
</script>

<div class="seg" role="radiogroup" aria-label={label} tabindex="-1" onkeydown={onKey}>
  {#each options as o (o.value)}
    <button
      role="radio"
      aria-checked={o.value === value}
      tabindex={o.value === value ? 0 : -1}
      onclick={() => onchange(o.value)}>{o.label}</button
    >
  {/each}
</div>

<style>
  .seg {
    display: inline-flex;
    padding: 3px;
    gap: 2px;
    border: 1px solid var(--dz-border);
    border-radius: 8px;
    background: var(--dz-input);
  }
  button {
    height: 28px;
    padding: 0 12px;
    border-radius: 6px;
    color: var(--dz-text-secondary);
    font-size: var(--dz-text-sm);
    font-weight: 560;
    white-space: nowrap;
  }
  button:hover {
    color: var(--dz-text-primary);
  }
  button[aria-checked="true"] {
    background: var(--dz-surface-elevated);
    color: var(--dz-text-primary);
    box-shadow: 0 0 0 1px var(--dz-border-strong);
  }
</style>
