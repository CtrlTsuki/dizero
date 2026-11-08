<script lang="ts">
  import { onMount, tick } from "svelte";
  import { FolderOpen, ShieldCheck, ScrollText } from "@lucide/svelte";
  import Segmented from "../components/Segmented.svelte";
  import Switch from "../components/Switch.svelte";
  import { api, errorMessage, type FolderValidation } from "../lib/api";
  import type { LogEntry } from "../lib/bindings/LogEntry";
  import { app } from "../lib/stores/app.svelte";
  import { plugins } from "../lib/stores/plugins.svelte";
  import { toasts } from "../lib/stores/toasts.svelte";
  import { clockTime, formatBytes } from "../lib/format";

  let logs = $state<LogEntry[]>([]);
  let level = $state<"all" | "INFO" | "WARN" | "ERROR">("all");
  let pluginsOnly = $state(false);
  let follow = $state(true);
  let box = $state<HTMLDivElement>();
  let validation = $state<FolderValidation | null>(null);
  let validating = $state(false);
  let lastSeq = 0;

  const rank = { ERROR: 0, WARN: 1, INFO: 2, DEBUG: 3, TRACE: 4 } as Record<string, number>;
  const shown = $derived(
    logs.filter(
      (l) => (level === "all" || (rank[l.level] ?? 9) <= (rank[level] ?? 9)) && (!pluginsOnly || l.target.startsWith("plugin:")),
    ),
  );

  async function poll() {
    try {
      const fresh = await api.logsRecent(lastSeq);
      if (!fresh.length) return;
      lastSeq = fresh[fresh.length - 1]?.seq ?? lastSeq;
      logs = [...logs, ...fresh].slice(-1000);
      if (follow) {
        await tick();
        if (box) box.scrollTop = box.scrollHeight;
      }
    } catch {
      /* the log view is best effort */
    }
  }

  onMount(() => {
    void poll();
    if (!plugins.overview) void plugins.loadOverview();
    const t = setInterval(() => document.visibilityState === "visible" && void poll(), 2000);
    return () => clearInterval(t);
  });

  async function validate() {
    validating = true;
    try {
      const v = await api.pluginValidateFolder();
      if (v) validation = v;
    } catch (e) {
      toasts.error("Validation failed", errorMessage(e));
    } finally {
      validating = false;
    }
  }

  const failing = $derived((plugins.overview?.installed ?? []).filter((p) => p.plugin.lastError || p.runtime?.lastError || p.runtime?.state === "faulted"));
</script>

<section class="page" aria-labelledby="dev-title">
  <header class="page-header">
    <div class="titles">
      <h1 id="dev-title">Developer Center</h1>
      <p>Build and debug DZSDK plugins. Full guide: <code>docs/PLUGIN_DEVELOPMENT.md</code> in the repository.</p>
    </div>
  </header>

  <div class="page-body">
    <div class="grid">
      <section class="panel">
        <h2>Environment</h2>
        {#if app.info}
          <dl>
            <dt>DZSDK API</dt><dd>v{app.info.pluginApiVersion}</dd>
            <dt>DiZero</dt><dd>{app.info.version} ({app.info.buildProfile})</dd>
            <dt>Tauri</dt><dd>{app.info.tauriVersion}</dd>
            <dt>WebView2</dt><dd>{app.info.webviewVersion ?? "unknown"}</dd>
            <dt>Platform</dt><dd>{app.info.os} {app.info.arch}</dd>
            <dt>Installed plugins</dt><dd><code>{app.info.pluginsDir}</code></dd>
            <dt>Bundled examples</dt><dd><code>{app.info.bundledPluginsDir}</code></dd>
            <dt>Logs</dt><dd><code>{app.info.logDir}</code></dd>
          </dl>
        {/if}
        {#if plugins.overview}
          <h3>Sandbox limits per plugin</h3>
          <dl>
            <dt>Memory</dt><dd>{formatBytes(plugins.overview.limits.memoryBytes)}</dd>
            <dt>Time per call</dt><dd>{plugins.overview.limits.callBudgetMs} ms ({plugins.overview.limits.activateBudgetMs} ms to load)</dd>
            <dt>Storage quota</dt><dd>{formatBytes(plugins.overview.limits.storageQuotaBytes)}</dd>
            <dt>Auto-stop after</dt><dd>{plugins.overview.limits.maxFailures} consecutive failures</dd>
          </dl>
        {/if}
      </section>

      <section class="panel">
        <h2>Validate a plugin</h2>
        <p class="muted small">Checks the manifest, permissions, contributions, entry file and folder limits, and lists every problem at once. Nothing is installed or run.</p>
        <button class="btn btn-ghost" onclick={validate} disabled={validating}><FolderOpen size={15} /> Choose folder…</button>
        {#if validation}
          <div class="report" class:ok={validation.report.issues.length === 0}>
            <p class="path"><code>{validation.folder}</code></p>
            {#if validation.report.issues.length === 0 && validation.report.manifest}
              <p class="pass"><ShieldCheck size={15} /> Valid: {validation.report.manifest.name} {validation.report.manifest.version}, {validation.report.fileCount} files, {formatBytes(validation.report.totalBytes)}</p>
            {:else}
              <p class="fail">{validation.report.issues.length} problem{validation.report.issues.length === 1 ? "" : "s"} found</p>
              <ul>
                {#each validation.report.issues as i, idx (idx)}
                  <li><code>{i.field}</code> {i.message}</li>
                {/each}
              </ul>
            {/if}
          </div>
        {/if}

        <h3>Plugin errors</h3>
        {#if failing.length === 0}
          <p class="muted small">No plugin has reported an error.</p>
        {:else}
          <ul class="errors">
            {#each failing as p (p.plugin.manifest.id)}
              <li><strong>{p.plugin.manifest.name}</strong> {p.plugin.lastError ?? p.runtime?.lastError ?? "stopped"}</li>
            {/each}
          </ul>
        {/if}
      </section>
    </div>

    <section class="panel">
      <div class="log-head">
        <h2>Runtime log</h2>
        <Segmented
          label="Minimum level"
          value={level}
          options={[
            { value: "all", label: "All" },
            { value: "INFO", label: "Info" },
            { value: "WARN", label: "Warnings" },
            { value: "ERROR", label: "Errors" },
          ]}
          onchange={(v) => (level = v)}
        />
        <label class="inline"><Switch checked={pluginsOnly} label="Plugins only" onchange={(v) => (pluginsOnly = v)} /> Plugins only</label>
        <label class="inline"><Switch checked={follow} label="Follow new entries" onchange={(v) => (follow = v)} /> Follow</label>
        <button class="btn btn-sm btn-ghost" onclick={() => api.logsOpenFolder().catch((e) => toasts.error("Couldn't open the folder", errorMessage(e)))}>
          <ScrollText size={13} /> Open log folder
        </button>
      </div>
      <!-- Scrollable regions must be keyboard-focusable (WCAG 2.1.1). -->
      <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
      <div class="log" bind:this={box} role="log" aria-label="Runtime log" tabindex="0">
        {#each shown as l (l.seq)}
          <div class="line" data-level={l.level}>
            <span class="t num">{clockTime(l.ts)}:{String(new Date(l.ts).getSeconds()).padStart(2, "0")}</span>
            <span class="lv">{l.level}</span>
            <span class="tg">{l.target}</span>
            <span class="msg">{l.message}</span>
          </div>
        {:else}
          <p class="muted small">No entries at this level yet.</p>
        {/each}
      </div>
    </section>

    <section class="panel">
      <h2>DZSDK at a glance</h2>
      <div class="ref">
        <pre><code>// dizero-plugin.json declares id, permissions and everything the plugin adds.
dz.plugin(&#123;
  activate() &#123; &#125;,                        // optional, runs on first use
  commands: &#123; hello: &#123; run: (args) =&gt; (&#123; replaceWith: `hi $&#123;args&#125;` &#125;) &#125; &#125;,
  events: &#123; messageCreated(msg) &#123; &#125; &#125;,    // needs events.messages
  panels: &#123;
    main: &#123;
      render: () =&gt; [&#123; type: "heading", text: "Hello" &#125;],
      action(id, values) &#123; &#125;,
    &#125;,
  &#125;,
&#125;);</code></pre>
        <ul class="api">
          <li><code>dz.storage.get / set / delete / keys</code> private data, needs <code>storage.local</code></li>
          <li><code>dz.notify(title, body)</code> in-app toast, needs <code>notifications</code></li>
          <li><code>dz.ui.refresh(panelId)</code> re-render a panel, needs <code>ui.panels</code> or <code>ui.sidebar</code></li>
          <li><code>dz.log.info / warn / error</code> writes to this log, always available</li>
          <li>View nodes: heading, text, stat, progress, list, button, input, divider, empty</li>
          <li>Not available by design: network, files, processes, timers, DOM</li>
        </ul>
      </div>
    </section>
  </div>
</section>

<style>
  .titles {
    flex: 1;
  }
  .titles code {
    font-size: 12px;
  }
  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
    margin-bottom: 16px;
  }
  .panel {
    padding: 16px 18px;
    border: 1px solid var(--dz-border);
    border-radius: var(--dz-r-tile);
    background: var(--dz-surface);
    min-width: 0;
  }
  .panel + .panel {
    margin-top: 0;
  }
  section.panel:not(.grid section) {
    margin-bottom: 16px;
  }
  h2 {
    font-size: var(--dz-text-md);
    margin-bottom: 12px;
  }
  h3 {
    margin: 18px 0 8px;
    color: var(--dz-text-muted);
    font-size: var(--dz-text-xs);
    font-weight: 620;
  }
  dl {
    display: grid;
    grid-template-columns: 140px 1fr;
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
  dd code,
  .path code {
    font-size: 12px;
  }
  .small {
    font-size: var(--dz-text-sm);
    margin-bottom: 12px;
  }
  .report {
    margin-top: 12px;
    padding: 10px 12px;
    border: 1px solid color-mix(in srgb, var(--dz-danger) 40%, transparent);
    border-radius: var(--dz-r-control);
    font-size: var(--dz-text-sm);
  }
  .report.ok {
    border-color: color-mix(in srgb, var(--dz-success) 40%, transparent);
  }
  .path {
    margin-bottom: 6px;
    color: var(--dz-text-muted);
    overflow-wrap: anywhere;
  }
  .pass {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--dz-success);
  }
  .fail {
    color: var(--dz-danger);
    font-weight: 600;
  }
  .report ul,
  .errors {
    margin: 6px 0 0;
    padding-left: 18px;
    display: grid;
    gap: 4px;
    color: var(--dz-text-secondary);
    font-size: var(--dz-text-sm);
  }
  .report code {
    color: var(--dz-text-primary);
    font-size: 12px;
    margin-right: 4px;
  }
  .errors strong {
    margin-right: 6px;
    color: var(--dz-danger);
  }
  .log-head {
    display: flex;
    align-items: center;
    gap: 14px;
    flex-wrap: wrap;
    margin-bottom: 12px;
  }
  .log-head h2 {
    margin: 0 auto 0 0;
  }
  .inline {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    font-size: var(--dz-text-sm);
    color: var(--dz-text-secondary);
  }
  .log {
    height: 320px;
    overflow: auto;
    padding: 8px 10px;
    border-radius: var(--dz-r-control);
    background: var(--dz-background);
    border: 1px solid var(--dz-border);
    font-family: var(--dz-font-code);
    font-size: 12px;
    line-height: 1.6;
  }
  .line {
    display: grid;
    grid-template-columns: 64px 48px 190px 1fr;
    gap: 10px;
  }
  .t,
  .tg {
    color: var(--dz-text-muted);
  }
  .tg {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .lv {
    color: var(--dz-text-secondary);
  }
  .line[data-level="WARN"] .lv {
    color: var(--dz-warning);
  }
  .line[data-level="ERROR"] .lv,
  .line[data-level="ERROR"] .msg {
    color: var(--dz-danger);
  }
  .msg {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .ref {
    display: grid;
    grid-template-columns: minmax(0, 1.3fr) minmax(0, 1fr);
    gap: 20px;
  }
  .ref pre {
    margin: 0;
    padding: 12px 14px;
    overflow-x: auto;
    border-radius: var(--dz-r-control);
    background: var(--dz-background);
    border: 1px solid var(--dz-border);
    font-size: 12px;
    line-height: 1.6;
  }
  .api {
    margin: 0;
    padding-left: 18px;
    display: grid;
    gap: 8px;
    font-size: var(--dz-text-sm);
    color: var(--dz-text-secondary);
  }
  .api code {
    color: var(--dz-text-primary);
    font-size: 12px;
  }
  @media (max-width: 1150px) {
    .grid,
    .ref {
      grid-template-columns: 1fr;
    }
  }
</style>
