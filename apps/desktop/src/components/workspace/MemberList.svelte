<script lang="ts">
  import Avatar from "../Avatar.svelte";
  import { workspace } from "../../lib/stores/workspace.svelte";

  const members = $derived(workspace.activeCommunityId ? (workspace.members[workspace.activeCommunityId] ?? null) : null);
  const online = $derived((members ?? []).filter((m) => m.user.presence !== "offline"));
  const offline = $derived((members ?? []).filter((m) => m.user.presence === "offline"));
  const label = { online: "Online", idle: "Idle", dnd: "Do not disturb", offline: "Offline" } as const;
</script>

<div class="members">
  {#if members === null}
    {#each [60, 45, 70, 50] as w, i (i)}
      <div class="sk"><div class="skeleton" style:width="32px" style:height="32px" style:border-radius="50%"></div><div class="skeleton" style:width="{w}%" style:height="12px"></div></div>
    {/each}
  {:else}
    {#each [{ title: "Online", list: online }, { title: "Offline", list: offline }] as group (group.title)}
      {#if group.list.length}
        <h3>{group.title} <span class="num">{group.list.length}</span></h3>
        <ul>
          {#each group.list as m (m.user.id)}
            <li class:offline={m.user.presence === "offline"}>
              <Avatar user={m.user} size={32} presence />
              <div class="who">
                <span class="name">
                  {m.user.displayName}
                  {#if m.user.isSelf && m.user.displayName.toLowerCase() !== "you"}<span class="you">(you)</span>{/if}
                </span>
                <span class="sub">{m.role ?? (m.user.isBot ? "Bot" : label[m.user.presence])}</span>
              </div>
            </li>
          {/each}
        </ul>
      {/if}
    {/each}
  {/if}
</div>

<style>
  .members {
    padding: 12px 8px 20px;
    --ring: var(--dz-surface);
  }
  .sk {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 8px;
  }
  h3 {
    padding: 8px 8px 6px;
    color: var(--dz-text-muted);
    font-size: var(--dz-text-xs);
    font-weight: 600;
  }
  h3 .num {
    margin-left: 4px;
  }
  ul {
    list-style: none;
    margin: 0 0 8px;
    padding: 0;
  }
  li {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 5px 8px;
    border-radius: var(--dz-r-control);
  }
  li:hover {
    background: var(--dz-sidebar-hover);
  }
  li.offline {
    opacity: 0.55;
  }
  .who {
    display: flex;
    flex-direction: column;
    min-width: 0;
    line-height: 1.25;
  }
  .name {
    font-size: var(--dz-text-sm);
    font-weight: 560;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .you {
    margin-left: 4px;
    color: var(--dz-text-muted);
    font-weight: 450;
  }
  .sub {
    color: var(--dz-text-muted);
    font-size: var(--dz-text-xs);
  }
</style>
