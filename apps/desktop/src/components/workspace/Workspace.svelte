<script lang="ts">
  import Sidebar from "./Sidebar.svelte";
  import ChannelHeader from "./ChannelHeader.svelte";
  import MessageList from "./MessageList.svelte";
  import Composer from "./Composer.svelte";
  import Dock from "./Dock.svelte";
  import { app } from "../../lib/stores/app.svelte";
  import { workspace } from "../../lib/stores/workspace.svelte";

  let { loading = false }: { loading?: boolean } = $props();
  let sidebarCollapsed = $state(false);
  const dockOpen = $derived(app.settings?.appearance.showMemberPanel ?? true);
</script>

<div class="workspace">
  {#if !sidebarCollapsed}
    <Sidebar {loading} />
  {/if}
  <main class="stage" aria-label="Conversation">
    <ChannelHeader
      {sidebarCollapsed}
      {dockOpen}
      onToggleSidebar={() => (sidebarCollapsed = !sidebarCollapsed)}
      onToggleDock={() => app.update((s) => (s.appearance.showMemberPanel = !s.appearance.showMemberPanel))}
    />
    {#if workspace.activeConversationId}
      {#key workspace.activeConversationId}
        <MessageList conversationId={workspace.activeConversationId} />
      {/key}
      <Composer conversationId={workspace.activeConversationId} />
    {:else}
      <div class="stage-loading" aria-busy="true">
        {#each [0.62, 0.4, 0.8, 0.5] as w, i (i)}
          <div class="sk-row">
            <div class="skeleton sk-avatar"></div>
            <div class="sk-lines">
              <div class="skeleton" style:width="{w * 40}%" style:height="12px"></div>
              <div class="skeleton" style:width="{w * 100}%" style:height="12px"></div>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </main>
  {#if dockOpen}
    <Dock />
  {/if}
</div>

<style>
  .workspace {
    flex: 1;
    min-width: 0;
    display: flex;
  }
  .stage {
    position: relative;
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    background: var(--dz-surface-secondary);
  }
  .stage-loading {
    flex: 1;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    gap: 22px;
    padding: 24px;
  }
  .sk-row {
    display: flex;
    gap: 14px;
  }
  .sk-avatar {
    width: 36px;
    height: 36px;
    border-radius: 50%;
  }
  .sk-lines {
    flex: 1;
    display: grid;
    gap: 8px;
  }
</style>
