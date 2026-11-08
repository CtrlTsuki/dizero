<script lang="ts">
  import { onMount } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { api, errorMessage } from "./lib/api";
  import type { Message } from "./lib/bindings/Message";
  import { applyTheme, resolveTheme } from "./lib/theme";
  import { app } from "./lib/stores/app.svelte";
  import { workspace } from "./lib/stores/workspace.svelte";
  import { plugins } from "./lib/stores/plugins.svelte";
  import { toasts } from "./lib/stores/toasts.svelte";
  import { menu } from "./lib/stores/menu.svelte";
  import Rail from "./components/Rail.svelte";
  import Workspace from "./components/workspace/Workspace.svelte";
  import QuickSwitcher from "./components/QuickSwitcher.svelte";
  import Toasts from "./components/Toasts.svelte";
  import ContextMenu from "./components/ContextMenu.svelte";

  // Tool pages are code-split: only the workspace is parsed at launch.
  const pages = {
    plugins: () => import("./pages/PluginsPage.svelte"),
    performance: () => import("./pages/PerformancePage.svelte"),
    developer: () => import("./pages/DeveloperPage.svelte"),
    settings: () => import("./pages/SettingsPage.svelte"),
  } as const;

  let booted = $state(false);
  let bootError = $state<string | null>(null);
  let windowFocused = $state(document.hasFocus());

  // Theme: base tokens → theme plugin → accent override → high contrast.
  $effect(() => {
    const a = app.settings?.appearance;
    if (!a) return;
    const tokens = a.themeId === "dizero-dark" ? {} : plugins.themeTokens(a.themeId);
    applyTheme(resolveTheme({ tokens, accentOverride: a.accent, highContrast: a.highContrast }));
    const root = document.documentElement;
    root.style.setProperty("--dz-font-scale", String(a.fontScale));
    root.dataset.density = a.density;
  });

  $effect(() => {
    document.documentElement.dataset.motion = app.reducedMotion ? "reduce" : "full";
  });

  function notifyIfNeeded(m: Message) {
    const n = app.settings?.notifications;
    if (!n?.inApp) return;
    if (n.mentionsOnly && !m.mentionsSelf) return;
    const conv = workspace.conversations.find((c) => c.id === m.conversationId);
    toasts.push({
      kind: "message",
      title: `${m.author.displayName}${conv ? ` in #${conv.name}` : ""}`,
      body: m.content.length > 140 ? `${m.content.slice(0, 140)}…` : m.content,
      action: { label: "Open", run: () => { app.go("workspace"); void workspace.selectConversation(m.conversationId); } },
    });
  }

  function onKeydown(e: KeyboardEvent) {
    const mod = e.ctrlKey || e.metaKey;
    if (mod && e.key.toLowerCase() === "k") {
      e.preventDefault();
      app.quickSwitcherOpen = !app.quickSwitcherOpen;
    } else if (mod && e.key === ",") {
      e.preventDefault();
      app.go("settings");
    } else if (mod && e.key.toLowerCase() === "f" && app.route === "workspace") {
      e.preventDefault();
      document.getElementById("dz-search")?.focus();
    } else if (e.altKey && (e.key === "ArrowUp" || e.key === "ArrowDown") && app.route === "workspace") {
      e.preventDefault();
      void workspace.step(e.key === "ArrowUp" ? -1 : 1);
    } else if (e.key === "Escape") {
      if (menu.open) menu.close();
      else if (app.quickSwitcherOpen) app.quickSwitcherOpen = false;
      else if (app.route !== "workspace" && !document.querySelector("[role=dialog]")) app.go("workspace");
    }
  }

  onMount(() => {
    const unlisten: UnlistenFn[] = [];
    let disposed = false;

    (async () => {
      try {
        await app.init();
        await Promise.all([workspace.init(), plugins.loadContributions()]);
        const subs = await Promise.all([
          listen<Message>("dz://message-created", (e) => {
            const viewing = app.route === "workspace" && windowFocused;
            const isNew = workspace.onMessageCreated(e.payload, viewing);
            if (isNew) notifyIfNeeded(e.payload);
          }),
          listen<{ id: string; conversationId: string }>("dz://message-deleted", (e) =>
            workspace.onMessageDeleted(e.payload.id, e.payload.conversationId),
          ),
          listen<{ conversationId: string; userId: string; displayName: string; active: boolean }>("dz://typing", (e) =>
            workspace.onTyping(e.payload.conversationId, e.payload.userId, e.payload.displayName, e.payload.active),
          ),
          listen<{ pluginId: string; title: string; body: string }>("dz://plugin-notify", (e) => {
            const name = plugins.overview?.installed.find((p) => p.plugin.manifest.id === e.payload.pluginId)?.plugin.manifest.name;
            toasts.push({ kind: "plugin", title: e.payload.title, body: `${e.payload.body}${name ? `\nFrom ${name}` : ""}` });
          }),
          listen<{ pluginId: string; panel: string }>("dz://plugin-refresh", (e) =>
            plugins.onRefresh(e.payload.pluginId, e.payload.panel),
          ),
          listen<{ pluginId: string; state: string; error: string | null }>("dz://plugin-state", (e) => {
            if (e.payload.state === "faulted") {
              toasts.error("A plugin was stopped after repeated failures", `${e.payload.pluginId}: ${e.payload.error ?? "unknown error"}`);
            }
            if (plugins.overview) void plugins.loadOverview();
          }),
          listen("dz://plugins-changed", () => void plugins.onChanged()),
        ]);
        if (disposed) subs.forEach((u) => u());
        else unlisten.push(...subs);
        booted = true;
      } catch (e) {
        bootError = errorMessage(e);
      } finally {
        // Show the window after the first real paint to avoid a blank flash.
        requestAnimationFrame(() => requestAnimationFrame(() => void api.appReady().catch(() => {})));
      }
    })();

    const focus = () => (windowFocused = true);
    const blur = () => (windowFocused = false);
    window.addEventListener("focus", focus);
    window.addEventListener("blur", blur);
    return () => {
      disposed = true;
      unlisten.forEach((u) => u());
      window.removeEventListener("focus", focus);
      window.removeEventListener("blur", blur);
    };
  });

  // Returning to the window marks the visible conversation read.
  $effect(() => {
    if (windowFocused && app.route === "workspace" && workspace.activeConversationId) {
      const c = workspace.activeConversation;
      if (c && c.unreadCount > 0) workspace.markRead(c.id);
    }
  });
</script>

<svelte:window onkeydown={onKeydown} />

{#if bootError}
  <main class="fatal" role="alert">
    <h1>DiZero couldn't start</h1>
    <p>{bootError}</p>
    <p class="muted">Details are in the log file. Restarting usually helps; if it doesn't, the log will say why.</p>
  </main>
{:else}
  <div class="shell" class:booting={!booted}>
    <Rail />
    {#if app.route === "workspace"}
      <Workspace loading={!booted} />
    {:else}
      {#await pages[app.route]()}
        <div class="page-loading" aria-busy="true"></div>
      {:then mod}
        <mod.default />
      {:catch e}
        <main class="fatal" role="alert"><h1>This page couldn't load</h1><p>{errorMessage(e)}</p></main>
      {/await}
    {/if}
  </div>
  {#if app.quickSwitcherOpen}
    <QuickSwitcher />
  {/if}
  <ContextMenu />
  <Toasts />
{/if}

<style>
  .shell {
    display: flex;
    height: 100%;
    min-width: 0;
  }
  .page-loading {
    flex: 1;
    background: var(--dz-surface-secondary);
  }
  .fatal {
    display: grid;
    place-content: center;
    gap: 10px;
    height: 100%;
    padding: 32px;
    max-width: 60ch;
    margin: 0 auto;
  }
  :global(.dz-tooltip) {
    position: fixed;
    top: 0;
    left: 0;
    z-index: 1000;
    max-width: 280px;
    padding: 5px 9px;
    border-radius: 6px;
    background: var(--dz-surface-elevated);
    box-shadow: var(--dz-shadow-float);
    color: var(--dz-text-primary);
    font-size: var(--dz-text-xs);
    font-weight: 540;
    pointer-events: none;
    opacity: 0;
    transition: opacity var(--dz-fast) var(--dz-ease);
  }
  :global(.dz-tooltip[data-open="true"]) {
    opacity: 1;
  }
</style>
