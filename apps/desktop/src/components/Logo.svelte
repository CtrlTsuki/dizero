<script lang="ts">
  let { size = 28 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 1024 1024" aria-hidden="true">
  <defs>
    <linearGradient id="dz-logo-ring" x1="0.15" y1="0" x2="0.85" y2="1">
      <stop offset="0" stop-color="var(--dz-accent-hover)" />
      <stop offset="1" stop-color="var(--dz-accent)" />
    </linearGradient>
  </defs>
  <rect x="350" y="226" width="324" height="572" rx="162" fill="none" stroke="url(#dz-logo-ring)" stroke-width="84" />
  <line x1="428" y1="664" x2="596" y2="360" stroke="url(#dz-logo-ring)" stroke-width="72" stroke-linecap="round" />
</svg>
