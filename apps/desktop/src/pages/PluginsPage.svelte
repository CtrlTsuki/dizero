<script lang="ts">
  import { onMount } from "svelte";
  import {
    ChevronRight,
    FolderOpen,
    Layers,
    Palette,
    MessageSquare,
    Notebook,
    Accessibility,
    Activity,
    Plug,
    FlaskConical,
    ShieldCheck,
    RotateCcw,
    Trash,
    RefreshCw,
  } from "@lucide/svelte";
  import Switch from "../components/Switch.svelte";
  import Modal from "../components/Modal.svelte";
  import PermissionReview from "./PermissionReview.svelte";
  import { api, errorMessage, type PluginView } from "../lib/api";
  import type { InstalledPlugin } from "../lib/bindings/InstalledPlugin";
  import type { Permission } from "../lib/bindings/Permission";
  import type { PluginCategory } from "../lib/bindings/PluginCategory";
  import { plugins } from "../lib/stores/plugins.svelte";
  import { toasts } from "../lib/stores/toasts.svelte";
  import { app } from "../lib/stores/app.svelte";
  import { formatBytes, relativeTime } from "../lib/format";

  let tab = $state<"installed" | "available">("installed");
  let expanded = $state<string | null>(null);
  let reviewing = $state<InstalledPlugin | null>(null);
  let confirmUninstall = $state<InstalledPlugin | null>(null);
  let busy = $state<string | null>(null);

  const overview = $derived(plugins.overview);
  const installed = $derived(overview?.installed ?? []);
  const catalog = $derived(overview?.catalog ?? []);

  const categoryIcon: Record<PluginCategory, typeof Layers> = {
    interface: Layers,
    appearance: Palette,
    messaging: MessageSquare,
    productivity: Notebook,
    accessibility: Accessibility,
    media: Activity,
    integration: Plug,
    developer: FlaskConical,
  };

  onMount(() => {
    void plugins.loadOverview();
    const t = setInterval(() => {
      if (document.visibilityState === "visible") void plugins.loadOverview();
    }, 4000);
    return () => clearInterval(t);
  });

  function status(v: PluginView): { tone: string; label: string } {
    const p = v.plugin;
    if (p.state === "integrityFailed") return { tone: "bad", label: "Files changed, blocked" };
    if (p.state === "dependencyMissing") return { tone: "warn", label: "Missing dependency" };
    if (!p.enabled) return { tone: "off", label: "Off" };
    if (p.manifest.kind === "theme") {
      return app.settings?.appearance.themeId === p.manifest.id ? { tone: "accent", label: "Active theme" } : { tone: "ok", label: "Theme available" };
    }
    if (v.runtime?.state === "faulted") return { tone: "bad", label: "Stopped after errors" };
    if (v.runtime?.state === "active") return { tone: "ok", label: "Running" };
    return { tone: "idle", label: "Idle, not loaded" };
  }

  async function run<T>(id: string, fn: () => Promise<T>, ok?: string): Promise<T | undefined> {
    busy = id;
    try {
      const r = await fn();
      if (ok) toasts.success(ok);
      await plugins.onChanged();
      await plugins.loadOverview();
      return r;
    } catch (e) {
      toasts.error("Plugin action failed", errorMessage(e));
      return undefined;
    } finally {
      busy = null;
    }
  }

  // After an install the permission review opens straight away, so the only
  // confirmation the user needs is the one that follows their decision.
  async function afterInstall(p: InstalledPlugin | null | undefined, name: string) {
    if (!p) return;
    tab = "installed";
    expanded = p.manifest.id;
    if (p.enabled) toasts.success(`${name} updated`);
    else reviewing = p;
  }

  async function installBundled(folder: string, name: string) {
    await afterInstall(await run(folder, () => api.pluginInstallBundled(folder)), name);
  }

  async function installFolder() {
    const p = await run("folder", () => api.pluginInstallFolder());
    await afterInstall(p, p?.manifest.name ?? "Plugin");
  }

  async function toggle(v: PluginView, on: boolean) {
    if (on) reviewing = v.plugin;
    else await run(v.plugin.manifest.id, () => api.pluginDisable(v.plugin.manifest.id), `${v.plugin.manifest.name} turned off`);
  }

  async function approve(granted: Permission[]) {
    const p = reviewing;
    reviewing = null;
    if (!p) return;
    await run(p.manifest.id, () => api.pluginApprove(p.manifest.id, granted), `${p.manifest.name} is on`);
  }

  async function uninstall() {
    const p = confirmUninstall;
    confirmUninstall = null;
    if (!p) return;
    if (app.settings?.appearance.themeId === p.manifest.id) await app.update((s) => (s.appearance.themeId = "dizero-dark"));
    await run(p.manifest.id, () => api.pluginUninstall(p.manifest.id), `${p.manifest.name} uninstalled`);
    expanded = null;
  }
</script>

<section class="page" aria-labelledby="plugins-title">
  <header class="page-header">
    <div class="titles">
      <h1 id="plugins-title">Plugins</h1>
      {#if overview}
        <p>
          Every plugin runs in its own sandbox with {formatBytes(overview.limits.memoryBytes)} of memory, {overview.limits.callBudgetMs} ms per
          call and only the permissions you approve.
        </p>
      {/if}
    </div>
    <button class="btn btn-ghost" onclick={installFolder} disabled={busy === "folder"}>
      <FolderOpen size={15} /> Install from folder
    </button>
  </header>

  <div class="tabs" role="tablist" aria-label="Plugin lists">
    <button role="tab" aria-selected={tab === "installed"} onclick={() => (tab = "installed")}>
      Installed <span class="num">{installed.length}</span>
    </button>
    <button role="tab" aria-selected={tab === "available"} onclick={() => (tab = "available")}>
      Available <span class="num">{catalog.length}</span>
    </button>
  </div>

  <div class="page-body">
    {#if !overview}
      {#each [1, 2, 3] as i (i)}<div class="skeleton sk-row"></div>{/each}
    {:else if tab === "installed"}
      {#if installed.length === 0}
        <div class="empty">
          <h2>No plugins installed</h2>
          <p>Start with the examples that ship with DiZero, or install a plugin folder you trust.</p>
          <button class="btn btn-primary" onclick={() => (tab = "available")}>Browse examples</button>
        </div>
      {:else}
        <ul class="list">
          {#each installed as v (v.plugin.manifest.id)}
            {@const p = v.plugin}
            {@const m = p.manifest}
            {@const s = status(v)}
            {@const Icon = categoryIcon[m.category]}
            {@const open = expanded === m.id}
            <li class:open>
              <div class="row">
                <button class="expand" aria-expanded={open} aria-controls="details-{m.id}" onclick={() => (expanded = open ? null : m.id)}>
                  <span class="chev" class:rot={open}><ChevronRight size={16} /></span>
                  <span class="tile"><Icon size={18} strokeWidth={1.7} /></span>
                  <span class="ident">
                    <span class="name">{m.name} <span class="ver num">{m.version}</span></span>
                    <span class="desc">{m.description}</span>
                  </span>
                </button>
                <span class="state">
                  <span class="led" data-tone={s.tone === "idle" ? "off" : s.tone}></span>
                  <span>{s.label}</span>
                  {#if v.runtime?.heapBytes}<span class="heap num">{formatBytes(v.runtime.heapBytes)}</span>{/if}
                </span>
                <Switch
                  checked={p.enabled}
                  label="{p.enabled ? 'Turn off' : 'Turn on'} {m.name}"
                  disabled={busy === m.id || p.state === "integrityFailed"}
                  onchange={(on) => toggle(v, on)}
                />
              </div>

              {#if open}
                <div class="details" id="details-{m.id}">
                  <div class="col">
                    <h3>Permissions</h3>
                    {#if p.permissions.length === 0}
                      <p class="muted small">None requested.</p>
                    {/if}
                    {#each p.permissions as g (g.permission)}
                      {@const info = overview.permissions.find((x) => x.permission === g.permission)}
                      <div class="perm">
                        <div>
                          <span class="perm-title">{info?.title ?? g.permission}</span>
                          <code>{g.permission}</code>
                        </div>
                        <Switch
                          checked={g.granted}
                          label="{g.granted ? 'Revoke' : 'Grant'} {info?.title ?? g.permission}"
                          disabled={busy === m.id}
                          onchange={(on) => run(m.id, () => api.pluginSetPermission(m.id, g.permission, on))}
                        />
                      </div>
                    {/each}

                    {#if m.contributes.commands.length || m.contributes.panels.length}
                      <h3>Adds</h3>
                      <ul class="adds">
                        {#each m.contributes.commands as c (c.name)}<li><code>/{c.name}</code> {c.description}</li>{/each}
                        {#each m.contributes.panels as pn (pn.id)}<li>
                            {pn.placement === "dock" ? "Side panel" : "Sidebar section"}: {pn.title}
                          </li>{/each}
                      </ul>
                    {/if}

                    {#if m.kind === "theme" && m.theme}
                      <h3>Colours</h3>
                      <div class="swatches">
                        {#each Object.entries(m.theme.tokens) as [k, color] (k)}
                          <span class="swatch" style:background={color} title="{k}: {color}"></span>
                        {/each}
                      </div>
                      {#if p.enabled}
                        <button
                          class="btn btn-sm btn-primary"
                          disabled={app.settings?.appearance.themeId === m.id}
                          onclick={() => app.update((st) => (st.appearance.themeId = m.id))}
                          >{app.settings?.appearance.themeId === m.id ? "In use" : "Use this theme"}</button
                        >
                      {/if}
                    {/if}
                  </div>

                  <div class="col">
                    <h3>Diagnostics</h3>
                    <dl>
                      <dt>ID</dt><dd><code>{m.id}</code></dd>
                      <dt>Source</dt><dd>{p.source === "bundled" ? "Bundled example" : "Local folder"}</dd>
                      <dt>Author</dt><dd>{m.author}</dd>
                      <dt>Integrity</dt>
                      <dd class="integrity" title="SHA-256 of every installed file, checked before each load: {p.integrity}">
                        <ShieldCheck size={13} /> <code>{p.integrity.slice(0, 16)}…</code>
                      </dd>
                      <dt>Signature</dt><dd>{m.signature ? "Present, not verified (signing is planned)" : "Unsigned"}</dd>
                      <dt>Updated</dt><dd>{relativeTime(p.updatedAt)}</dd>
                      {#if v.runtime}
                        <dt>Runtime</dt>
                        <dd class="num">
                          {v.runtime.calls} calls, {v.runtime.failures} failed, {v.runtime.activations} loads{#if v.runtime.calls}, avg {(
                              v.runtime.totalTimeUs /
                              v.runtime.calls /
                              1000
                            ).toFixed(2)} ms{/if}
                        </dd>
                      {/if}
                    </dl>

                    {#if p.lastError || v.runtime?.lastError}
                      <div class="error">
                        <p><strong>Last error</strong> {p.lastError ?? v.runtime?.lastError}</p>
                        {#if v.runtime?.state === "faulted" || p.lastError}
                          <button class="btn btn-sm btn-ghost" onclick={() => run(m.id, () => api.pluginReset(m.id), `${m.name} restarted`)}>
                            <RefreshCw size={13} /> Restart plugin
                          </button>
                        {/if}
                      </div>
                    {/if}

                    <div class="actions">
                      {#if p.hasBackup}
                        <button class="btn btn-sm btn-ghost" onclick={() => run(m.id, () => api.pluginRollback(m.id), `${m.name} rolled back`)}>
                          <RotateCcw size={13} /> Roll back to previous version
                        </button>
                      {/if}
                      <button class="btn btn-sm btn-danger" onclick={() => (confirmUninstall = p)}>
                        <Trash size={13} /> Uninstall
                      </button>
                    </div>
                  </div>
                </div>
              {/if}
            </li>
          {/each}
        </ul>
      {/if}
    {:else}
      <p class="lead">Examples bundled with this build. They're reviewed like any other plugin: nothing runs until you enable it.</p>
      <ul class="list">
        {#each catalog as c (c.folder)}
          {#if c.manifest}
            {@const m = c.manifest}
            {@const Icon = categoryIcon[m.category]}
            <li>
              <div class="row">
                <div class="expand static">
                  <span class="tile"><Icon size={18} strokeWidth={1.7} /></span>
                  <span class="ident">
                    <span class="name">{m.name} <span class="ver num">{m.version}</span></span>
                    <span class="desc">{m.description}</span>
                  </span>
                </div>
                <span class="perm-count">{m.permissions.length} permission{m.permissions.length === 1 ? "" : "s"}</span>
                {#if c.installedVersion && !c.updateAvailable}
                  <span class="badge">Installed</span>
                {:else}
                  <button class="btn btn-sm btn-primary" disabled={busy === c.folder} onclick={() => installBundled(c.folder, m.name)}>
                    {c.updateAvailable ? `Update to ${m.version}` : "Install"}
                  </button>
                {/if}
              </div>
            </li>
          {:else}
            <li>
              <div class="row">
                <div class="ident">
                  <span class="name">{c.folder}</span>
                  <span class="desc bad">Invalid: {c.issues.map((i) => `${i.field}: ${i.message}`).join("; ")}</span>
                </div>
              </div>
            </li>
          {/if}
        {/each}
      </ul>
    {/if}
  </div>
</section>

{#if reviewing && overview}
  <PermissionReview plugin={reviewing} catalogue={overview.permissions} onConfirm={approve} onCancel={() => (reviewing = null)} />
{/if}

{#if confirmUninstall}
  <Modal title="Uninstall {confirmUninstall.manifest.name}?" onClose={() => (confirmUninstall = null)}>
    <p class="muted">Its files and everything it stored on this computer will be deleted. This can't be undone.</p>
    {#snippet footer()}
      <button class="btn btn-ghost" onclick={() => (confirmUninstall = null)}>Cancel</button>
      <button class="btn btn-danger" onclick={uninstall}>Uninstall</button>
    {/snippet}
  </Modal>
{/if}

<style>
  .titles {
    flex: 1;
    min-width: 0;
  }
  .tabs {
    display: flex;
    gap: 4px;
    padding: 0 22px;
    border-bottom: 1px solid var(--dz-border);
  }
  .tabs button {
    display: inline-flex;
    gap: 6px;
    padding: 12px 8px;
    color: var(--dz-text-muted);
    font-size: var(--dz-text-sm);
    font-weight: 560;
    border-bottom: 2px solid transparent;
  }
  .tabs button[aria-selected="true"] {
    color: var(--dz-text-primary);
    border-bottom-color: var(--dz-accent);
  }
  .tabs .num {
    color: var(--dz-text-muted);
  }
  .lead {
    margin-bottom: 14px;
    color: var(--dz-text-secondary);
    font-size: var(--dz-text-sm);
  }
  .sk-row {
    height: 64px;
    margin-bottom: 8px;
  }
  .empty {
    display: grid;
    justify-items: start;
    gap: 8px;
    max-width: 52ch;
    padding: 40px 0;
  }
  .empty p {
    color: var(--dz-text-secondary);
  }
  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    border: 1px solid var(--dz-border);
    border-radius: var(--dz-r-tile);
    overflow: hidden;
    background: var(--dz-surface);
  }
  .list > li + li {
    border-top: 1px solid var(--dz-border);
  }
  .row {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 12px 16px 12px 8px;
  }
  .expand {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 12px;
    text-align: left;
    padding: 2px 4px;
    border-radius: var(--dz-r-control);
  }
  .expand.static {
    padding-left: 12px;
  }
  .chev {
    display: inline-flex;
    color: var(--dz-text-muted);
    transition: transform var(--dz-fast) var(--dz-ease);
  }
  .chev.rot {
    transform: rotate(90deg);
  }
  .tile {
    display: grid;
    place-items: center;
    width: 38px;
    height: 38px;
    flex: none;
    border-radius: 10px;
    background: var(--dz-surface-secondary);
    border: 1px solid var(--dz-border);
    color: var(--dz-accent-hover);
  }
  .ident {
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .name {
    font-weight: 620;
  }
  .ver {
    margin-left: 4px;
    color: var(--dz-text-muted);
    font-size: var(--dz-text-xs);
    font-weight: 500;
  }
  .desc {
    color: var(--dz-text-secondary);
    font-size: var(--dz-text-sm);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .desc.bad {
    color: var(--dz-danger);
    white-space: normal;
  }
  .state {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    min-width: 200px;
    color: var(--dz-text-secondary);
    font-size: var(--dz-text-sm);
  }
  .heap {
    color: var(--dz-text-muted);
    font-size: var(--dz-text-xs);
  }
  .perm-count {
    color: var(--dz-text-muted);
    font-size: var(--dz-text-sm);
  }
  .details {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 28px;
    padding: 6px 24px 20px 70px;
  }
  .col {
    min-width: 0;
  }
  h3 {
    margin: 12px 0 8px;
    color: var(--dz-text-muted);
    font-size: var(--dz-text-xs);
    font-weight: 620;
  }
  .perm {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 7px 0;
    border-bottom: 1px solid var(--dz-border);
  }
  .perm-title {
    font-size: var(--dz-text-sm);
    font-weight: 560;
    margin-right: 6px;
  }
  .perm code,
  .adds code {
    color: var(--dz-text-muted);
    font-size: 11px;
  }
  .adds {
    margin: 0;
    padding-left: 18px;
    color: var(--dz-text-secondary);
    font-size: var(--dz-text-sm);
    display: grid;
    gap: 3px;
  }
  .adds code {
    color: var(--dz-text-primary);
    font-size: 12px;
  }
  .swatches {
    display: flex;
    gap: 6px;
    margin-bottom: 10px;
    flex-wrap: wrap;
  }
  .swatch {
    width: 22px;
    height: 22px;
    border-radius: 6px;
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.12);
  }
  dl {
    display: grid;
    grid-template-columns: 96px 1fr;
    gap: 6px 12px;
    margin: 0;
    font-size: var(--dz-text-sm);
  }
  dt {
    color: var(--dz-text-muted);
  }
  dd {
    margin: 0;
    min-width: 0;
    overflow-wrap: anywhere;
  }
  dd code {
    font-size: 12px;
  }
  .integrity {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    color: var(--dz-success);
  }
  .integrity code {
    color: var(--dz-text-secondary);
  }
  .error {
    display: grid;
    gap: 8px;
    justify-items: start;
    margin-top: 14px;
    padding: 10px 12px;
    border: 1px solid color-mix(in srgb, var(--dz-danger) 40%, transparent);
    border-radius: var(--dz-r-control);
    font-size: var(--dz-text-sm);
    color: var(--dz-text-secondary);
    overflow-wrap: anywhere;
  }
  .error strong {
    color: var(--dz-danger);
    margin-right: 4px;
  }
  .actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    margin-top: 16px;
  }
  .small {
    font-size: var(--dz-text-sm);
  }
</style>
