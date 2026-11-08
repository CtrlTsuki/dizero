<script lang="ts">
  // Segmented LED meter, vertical like a mixer channel. Segments light from
  // the bottom; colour shifts from green to amber to red past the marker.
  let {
    value,
    max,
    marker = null,
    segments = 24,
    height = 180,
    label,
  }: { value: number; max: number; marker?: number | null; segments?: number; height?: number; label: string } = $props();

  const lit = $derived(Math.min(segments, Math.round((Math.max(0, value) / max) * segments)));
  const markerSeg = $derived(marker === null ? null : Math.min(segments, Math.round((marker / max) * segments)));

  function tone(i: number): string {
    // i counts from the bottom (0) to the top (segments - 1).
    if (markerSeg !== null) {
      if (i < markerSeg) return "ok";
      if (i < markerSeg + Math.max(2, Math.round(segments * 0.15))) return "warn";
      return "bad";
    }
    const f = i / segments;
    return f < 0.65 ? "ok" : f < 0.85 ? "warn" : "bad";
  }
</script>

<div class="meter" style:height="{height}px" role="meter" aria-label={label} aria-valuemin={0} aria-valuemax={max} aria-valuenow={value}>
  {#each Array.from({ length: segments }, (_, k) => segments - 1 - k) as i (i)}
    <span class="seg" data-tone={tone(i)} class:on={i < lit} class:mark={markerSeg !== null && i === markerSeg - 1}></span>
  {/each}
</div>

<style>
  .meter {
    display: flex;
    flex-direction: column;
    gap: 2px;
    width: 14px;
    padding: 3px;
    border-radius: 4px;
    background: #08080b;
    box-shadow: inset 0 0 0 1px var(--dz-border);
  }
  .seg {
    flex: 1;
    border-radius: 1.5px;
    background: #1a1a20;
    transition: background-color 180ms linear, box-shadow 180ms linear;
  }
  .seg.mark {
    box-shadow: 0 -1.5px 0 0 var(--dz-text-muted);
  }
  .seg.on[data-tone="ok"] {
    background: var(--dz-success);
    box-shadow: 0 0 5px color-mix(in srgb, var(--dz-success) 50%, transparent);
  }
  .seg.on[data-tone="warn"] {
    background: var(--dz-warning);
    box-shadow: 0 0 5px color-mix(in srgb, var(--dz-warning) 50%, transparent);
  }
  .seg.on[data-tone="bad"] {
    background: var(--dz-danger);
    box-shadow: 0 0 5px color-mix(in srgb, var(--dz-danger) 50%, transparent);
  }
  .seg.on.mark {
    box-shadow: 0 -1.5px 0 0 var(--dz-text-primary);
  }
</style>
