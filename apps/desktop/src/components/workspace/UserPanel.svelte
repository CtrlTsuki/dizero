<script lang="ts">
  import { Settings } from "@lucide/svelte";
  import Avatar from "../Avatar.svelte";
  import { app } from "../../lib/stores/app.svelte";
  import { workspace } from "../../lib/stores/workspace.svelte";
  import { tooltip } from "../../lib/tooltip";

  const modeLabel = { eco: "Eco", balanced: "Balanced", performance: "Performance" } as const;
  const modeTone = { eco: "ok", balanced: "accent", performance: "warn" } as const;
  const mode = $derived(app.settings?.performance.mode ?? "balanced");
</script>

<div class="user">
  {#if workspace.self}
    <Avatar user={workspace.self} size={34} presence />
    <div class="who">
      <span class="name">{workspace.self.displayName}</span>
      <span class="status">Online</span>
    </div>
  {/if}
  <button
    class="mode"
    aria-label="Performance mode: {modeLabel[mode]}. Open Performance Center"
    use:tooltip={{ text: "Performance mode", side: "top" }}
    onclick={() => app.go("performance")}
  >
    <span class="led" data-tone={modeTone[mode]}></span>
    {modeLabel[mode]}
  </button>
  <button class="icon-btn" aria-label="Settings" use:tooltip={{ text: "Settings", side: "top" }} onclick={() => app.go("settings")}>
    <Settings size={17} strokeWidth={1.8} />
  </button>
</div>

<style>
  .user {
    display: flex;
    align-items: center;
    gap: 9px;
    height: 56px;
    padding: 0 8px 0 12px;
    border-top: 1px solid var(--dz-border);
    background: color-mix(in srgb, var(--dz-background) 55%, var(--dz-surface));
    flex: none;
    --ring: var(--dz-surface);
  }
  .who {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    line-height: 1.25;
  }
  .name {
    font-size: var(--dz-text-sm);
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .status {
    color: var(--dz-text-muted);
    font-size: var(--dz-text-xs);
  }
  .mode {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 26px;
    padding: 0 8px;
    border: 1px solid var(--dz-border);
    border-radius: 999px;
    color: var(--dz-text-secondary);
    font-size: var(--dz-text-xs);
    font-weight: 560;
  }
  .mode:hover {
    border-color: var(--dz-border-strong);
    color: var(--dz-text-primary);
  }
  .mode .led {
    width: 6px;
    height: 6px;
  }
</style>
