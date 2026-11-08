<script lang="ts">
  import { untrack } from "svelte";
  import Modal from "../components/Modal.svelte";
  import type { InstalledPlugin } from "../lib/bindings/InstalledPlugin";
  import type { Permission } from "../lib/bindings/Permission";
  import type { PermissionInfo } from "../lib/bindings/PermissionInfo";

  let {
    plugin,
    catalogue,
    onConfirm,
    onCancel,
  }: {
    plugin: InstalledPlugin;
    catalogue: PermissionInfo[];
    onConfirm: (granted: Permission[]) => void;
    onCancel: () => void;
  } = $props();

  // Default when the dialog opens: what was granted before, or everything
  // requested on first review. Deliberately not reactive to later prop changes.
  let chosen = $state<Record<string, boolean>>(
    untrack(() => {
      const anyGranted = plugin.permissions.some((g) => g.granted);
      return Object.fromEntries(plugin.permissions.map((g) => [g.permission, anyGranted ? g.granted : true]));
    }),
  );
  const info = (p: Permission) => catalogue.find((c) => c.permission === p);
  const riskLabel = { low: "Low risk", medium: "Review", high: "High risk" } as const;
  const riskTone = { low: "ok", medium: "warn", high: "bad" } as const;
</script>

<Modal title="Enable {plugin.manifest.name}?" onClose={onCancel} width={520}>
  <p class="desc">{plugin.manifest.description}</p>
  <p class="by">Version {plugin.manifest.version} by {plugin.manifest.author}{plugin.manifest.signature ? "" : ", unsigned"}</p>

  {#if plugin.permissions.length}
    <h3>This plugin asks to</h3>
    <ul>
      {#each plugin.permissions as g (g.permission)}
        {@const i = info(g.permission)}
        <li>
          <label>
            <input type="checkbox" bind:checked={chosen[g.permission]} />
            <span class="perm">
              <span class="perm-head">
                <span class="perm-title">{i?.title ?? g.permission}</span>
                {#if i}<span class="badge" data-tone={riskTone[i.risk]}>{riskLabel[i.risk]}</span>{/if}
              </span>
              <span class="perm-desc">{i?.description ?? ""}</span>
              <code class="perm-key">{g.permission}</code>
            </span>
          </label>
        </li>
      {/each}
    </ul>
    <p class="note">Unticked permissions stay blocked; features that need them will report an error instead. You can change this later.</p>
  {:else}
    <p class="note">This plugin needs no permissions.</p>
  {/if}

  {#snippet footer()}
    <button class="btn btn-ghost" onclick={onCancel}>Cancel</button>
    <button
      class="btn btn-primary"
      onclick={() => onConfirm(plugin.permissions.filter((g) => chosen[g.permission]).map((g) => g.permission))}
      >Enable plugin</button
    >
  {/snippet}
</Modal>

<style>
  .desc {
    color: var(--dz-text-secondary);
  }
  .by {
    margin-top: 4px;
    color: var(--dz-text-muted);
    font-size: var(--dz-text-xs);
  }
  h3 {
    margin: 18px 0 8px;
    font-size: var(--dz-text-sm);
    color: var(--dz-text-secondary);
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 6px;
  }
  label {
    display: flex;
    gap: 12px;
    padding: 10px 12px;
    border: 1px solid var(--dz-border);
    border-radius: var(--dz-r-control);
    background: var(--dz-surface);
    cursor: pointer;
  }
  label:has(input:checked) {
    border-color: var(--dz-accent-muted);
  }
  input[type="checkbox"] {
    width: 16px;
    height: 16px;
    margin-top: 2px;
    accent-color: var(--dz-accent);
    flex: none;
  }
  .perm {
    display: grid;
    gap: 3px;
    min-width: 0;
  }
  .perm-head {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .perm-title {
    font-weight: 600;
  }
  .perm-desc {
    color: var(--dz-text-secondary);
    font-size: var(--dz-text-sm);
  }
  .perm-key {
    color: var(--dz-text-muted);
    font-size: 11px;
  }
  .note {
    margin-top: 12px;
    color: var(--dz-text-muted);
    font-size: var(--dz-text-xs);
  }
</style>
