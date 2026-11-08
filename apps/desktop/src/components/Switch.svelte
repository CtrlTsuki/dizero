<script lang="ts">
  let {
    checked,
    label,
    disabled = false,
    onchange,
  }: { checked: boolean; label: string; disabled?: boolean; onchange: (v: boolean) => void } = $props();
</script>

<button
  class="switch"
  role="switch"
  aria-checked={checked}
  aria-label={label}
  {disabled}
  onclick={() => onchange(!checked)}
>
  <span class="knob"></span>
</button>

<style>
  .switch {
    position: relative;
    width: 36px;
    height: 20px;
    flex: none;
    border-radius: 10px;
    background: var(--dz-border-strong);
    transition: background-color var(--dz-fast) var(--dz-ease);
  }
  .switch[aria-checked="true"] {
    background: var(--dz-accent);
  }
  .knob {
    position: absolute;
    top: 3px;
    left: 3px;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: #fff;
    transition: transform var(--dz-fast) var(--dz-ease);
  }
  .switch[aria-checked="true"] .knob {
    transform: translateX(16px);
  }
  .switch:disabled {
    opacity: 0.45;
  }
</style>
