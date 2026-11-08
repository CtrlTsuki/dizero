<script lang="ts">
  import type { User } from "../lib/bindings/User";

  let { user, size = 36, presence = false }: { user: User; size?: number; presence?: boolean } = $props();

  const initials = $derived(
    user.displayName
      .split(/\s+/)
      .filter(Boolean)
      .slice(0, 2)
      .map((w) => [...w][0]?.toUpperCase() ?? "")
      .join("") || "?",
  );
</script>

<span class="avatar" style:--size="{size}px" style:--tint={user.avatarColor} aria-hidden="true">
  <span class="initials">{initials}</span>
  {#if presence}
    <span class="presence led" data-tone={user.presence}></span>
  {/if}
</span>

<style>
  .avatar {
    position: relative;
    display: inline-grid;
    place-items: center;
    width: var(--size);
    height: var(--size);
    flex: none;
    border-radius: 50%;
    background: color-mix(in srgb, var(--tint) 24%, var(--dz-surface));
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--tint) 40%, transparent);
  }
  .initials {
    color: color-mix(in srgb, var(--tint) 70%, var(--dz-text-primary));
    font-size: calc(var(--size) * 0.36);
    font-weight: 650;
    letter-spacing: -0.02em;
  }
  .presence {
    position: absolute;
    right: -1px;
    bottom: -1px;
    width: calc(var(--size) * 0.3);
    height: calc(var(--size) * 0.3);
    min-width: 9px;
    min-height: 9px;
    outline: 2.5px solid var(--ring, var(--dz-surface));
  }
</style>
