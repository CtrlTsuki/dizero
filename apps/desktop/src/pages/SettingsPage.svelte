<script lang="ts">
  import { onMount } from "svelte";
  import Switch from "../components/Switch.svelte";
  import Segmented from "../components/Segmented.svelte";
  import Modal from "../components/Modal.svelte";
  import { api, errorMessage } from "../lib/api";
  import type { StorageStats } from "../lib/bindings/StorageStats";
  import type { Density } from "../lib/bindings/Density";
  import type { MotionPref } from "../lib/bindings/MotionPref";
  import type { PerfMode } from "../lib/bindings/PerfMode";
  import type { LogLevel } from "../lib/bindings/LogLevel";
  import { app } from "../lib/stores/app.svelte";
  import { workspace } from "../lib/stores/workspace.svelte";
  import { plugins } from "../lib/stores/plugins.svelte";
  import { toasts } from "../lib/stores/toasts.svelte";
  import { formatBytes } from "../lib/format";
  import { BASE_TOKENS } from "../lib/theme";

  const sections = [
    { id: "profile", label: "Profile" },
    { id: "appearance", label: "Appearance" },
    { id: "accessibility", label: "Accessibility" },
    { id: "notifications", label: "Notifications" },
    { id: "performance", label: "Performance" },
    { id: "integrations", label: "Integrations" },
    { id: "plugins", label: "Plugins" },
    { id: "data", label: "Storage and privacy" },
    { id: "shortcuts", label: "Keyboard shortcuts" },
    { id: "advanced", label: "Advanced" },
  ];

  const s = $derived(app.settings);
  let stats = $state<StorageStats | null>(null);
  let confirm = $state<null | { title: string; body: string; action: string; run: () => Promise<void> }>(null);
  let nameDraft = $state(app.settings?.profile.displayName ?? "");

  const accents = ["#9278f5", "#6fb3e8", "#65c99a", "#e8b96c", "#f08a5d", "#e57883", "#d38cf0"];
  const shortcuts: [string, string][] = [
    ["Ctrl K", "Quick switcher: jump to any channel or page"],
    ["Alt ↑ / Alt ↓", "Previous / next channel"],
    ["Ctrl F", "Search messages"],
    ["Ctrl ,", "Open settings"],
    ["Enter", "Send message"],
    ["Shift Enter", "New line"],
    ["/", "Plugin commands in the composer"],
    ["Esc", "Close overlays, return to the workspace"],
  ];

  onMount(() => {
    void api.dataStats().then((x) => (stats = x)).catch(() => {});
    if (!plugins.overview) void plugins.loadOverview();
  });

  function jump(id: string) {
    app.settingsSection = id;
    document.getElementById(`set-${id}`)?.scrollIntoView({ behavior: app.reducedMotion ? "auto" : "smooth", block: "start" });
  }

  $effect(() => {
    const id = app.settingsSection;
    queueMicrotask(() => document.getElementById(`set-${id}`)?.scrollIntoView({ block: "start" }));
  });

  async function saveName() {
    const name = nameDraft.trim();
    if (!name || name === s?.profile.displayName) return;
    await app.update((x) => (x.profile.displayName = name));
    await workspace.refreshSnapshot();
    toasts.success("Display name updated");
  }

  async function guarded(fn: () => Promise<unknown>, ok: string) {
    try {
      await fn();
      toasts.success(ok);
      stats = await api.dataStats();
    } catch (e) {
      toasts.error("That didn't work", errorMessage(e));
    }
  }

  const themes = $derived([
    { id: "dizero-dark", label: "DiZero Dark", tokens: BASE_TOKENS as Record<string, string> },
    ...plugins.contributions.themes.map((t) => ({ id: t.pluginId, label: t.label, tokens: { ...BASE_TOKENS, ...t.tokens } as Record<string, string> })),
  ]);
</script>

<section class="page settings" aria-labelledby="settings-title">
  <nav class="toc" aria-label="Settings sections">
    <h1 id="settings-title">Settings</h1>
    {#each sections as sec (sec.id)}
      <button class:active={app.settingsSection === sec.id} onclick={() => jump(sec.id)}>{sec.label}</button>
    {/each}
  </nav>

  {#if s}
    <div class="content">
      <section id="set-profile">
        <h2>Profile</h2>
        <div class="row">
          <div class="label"><label for="display-name">Display name</label><p>Shown on your messages in the local demo.</p></div>
          <input
            id="display-name"
            class="field narrow"
            maxlength="32"
            bind:value={nameDraft}
            onblur={saveName}
            onkeydown={(e) => e.key === "Enter" && saveName()}
          />
        </div>
      </section>

      <section id="set-appearance">
        <h2>Appearance</h2>
        <div class="row column">
          <div class="label"><span>Theme</span><p>Install theme plugins to add more.</p></div>
          <div class="themes" role="radiogroup" aria-label="Theme">
            {#each themes as t (t.id)}
              <button class="theme" role="radio" aria-checked={s.appearance.themeId === t.id} onclick={() => app.update((x) => (x.appearance.themeId = t.id))}>
                <span class="preview" style:background={t.tokens.background}>
                  <span class="pv-side" style:background={t.tokens.surface}></span>
                  <span class="pv-main" style:background={t.tokens.surfaceSecondary}>
                    <span class="pv-line" style:background={t.tokens.textMuted}></span>
                    <span class="pv-line short" style:background={t.tokens.accent}></span>
                  </span>
                </span>
                <span class="theme-name">{t.label}</span>
              </button>
            {/each}
            <button class="theme more" onclick={() => app.go("plugins")}>Get themes</button>
          </div>
        </div>
        <div class="row">
          <div class="label"><span>Accent colour</span><p>Overrides the theme's accent.</p></div>
          <div class="accents">
            {#each accents as a (a)}
              <button
                class="accent"
                style:background={a}
                aria-label="Accent {a}"
                aria-pressed={s.appearance.accent === a}
                onclick={() => app.update((x) => (x.appearance.accent = a))}
              ></button>
            {/each}
            <label class="custom" title="Custom colour">
              <input
                type="color"
                value={s.appearance.accent ?? "#9278f5"}
                onchange={(e) => app.update((x) => (x.appearance.accent = (e.currentTarget as HTMLInputElement).value))}
              />
              <span class="sr-only">Custom accent colour</span>
            </label>
            <button class="btn btn-sm btn-ghost" disabled={!s.appearance.accent} onclick={() => app.update((x) => (x.appearance.accent = null))}>Use theme accent</button>
          </div>
        </div>
        <div class="row">
          <div class="label"><span>Density</span><p>Spacing between messages.</p></div>
          <Segmented
            label="Density"
            value={s.appearance.density}
            options={[{ value: "compact", label: "Compact" }, { value: "cozy", label: "Cozy" }, { value: "comfortable", label: "Comfortable" }] as { value: Density; label: string }[]}
            onchange={(v) => app.update((x) => (x.appearance.density = v))}
          />
        </div>
        <div class="row">
          <div class="label"><label for="font-scale">Text size</label><p class="num">{Math.round(s.appearance.fontScale * 100)}%</p></div>
          <input
            id="font-scale"
            type="range"
            min="0.85"
            max="1.3"
            step="0.05"
            value={s.appearance.fontScale}
            onchange={(e) => app.update((x) => (x.appearance.fontScale = Number((e.currentTarget as HTMLInputElement).value)))}
          />
        </div>
        <div class="row">
          <div class="label"><span>Show avatars</span><p>Turn off for a denser, text-only history.</p></div>
          <Switch checked={s.appearance.showAvatars} label="Show avatars" onchange={(v) => app.update((x) => (x.appearance.showAvatars = v))} />
        </div>
        <div class="row">
          <div class="label"><span>Side panel</span><p>Members, search results and plugin panels.</p></div>
          <Switch checked={s.appearance.showMemberPanel} label="Show side panel" onchange={(v) => app.update((x) => (x.appearance.showMemberPanel = v))} />
        </div>
      </section>

      <section id="set-accessibility">
        <h2>Accessibility</h2>
        <div class="row">
          <div class="label"><span>Motion</span><p>"System" follows Windows' animation setting. Eco mode also turns animation off.</p></div>
          <Segmented
            label="Motion"
            value={s.appearance.motion}
            options={[{ value: "system", label: "System" }, { value: "reduce", label: "Reduce" }, { value: "full", label: "Full" }] as { value: MotionPref; label: string }[]}
            onchange={(v) => app.update((x) => (x.appearance.motion = v))}
          />
        </div>
        <div class="row">
          <div class="label"><span>High contrast</span><p>Raises text and border contrast to at least WCAG AAA for body text.</p></div>
          <Switch checked={s.appearance.highContrast} label="High contrast" onchange={(v) => app.update((x) => (x.appearance.highContrast = v))} />
        </div>
        <div class="row">
          <div class="label"><span>Keyboard</span><p>Every control is reachable with Tab; focus is always visible.</p></div>
          <button class="btn btn-sm btn-ghost" onclick={() => jump("shortcuts")}>View shortcuts</button>
        </div>
      </section>

      <section id="set-notifications">
        <h2>Notifications</h2>
        <div class="row">
          <div class="label"><span>In-app notifications</span><p>A toast for new messages in conversations you aren't viewing.</p></div>
          <Switch checked={s.notifications.inApp} label="In-app notifications" onchange={(v) => app.update((x) => (x.notifications.inApp = v))} />
        </div>
        <div class="row">
          <div class="label"><span>Only when mentioned</span><p>Ignore everything except messages that mention you.</p></div>
          <Switch
            checked={s.notifications.mentionsOnly}
            disabled={!s.notifications.inApp}
            label="Only when mentioned"
            onchange={(v) => app.update((x) => (x.notifications.mentionsOnly = v))}
          />
        </div>
        <p class="planned">Windows system notifications are planned for when real integrations arrive (Phase 3).</p>
      </section>

      <section id="set-performance">
        <h2>Performance</h2>
        <div class="row">
          <div class="label"><span>Mode</span><p>Eco saves memory and battery; Performance keeps more history in memory.</p></div>
          <Segmented
            label="Performance mode"
            value={s.performance.mode}
            options={[{ value: "eco", label: "Eco" }, { value: "balanced", label: "Balanced" }, { value: "performance", label: "Performance" }] as { value: PerfMode; label: string }[]}
            onchange={(v) => app.update((x) => (x.performance.mode = v))}
          />
        </div>
        <div class="row">
          <div class="label">
            <span>Hardware acceleration</span>
            <p>
              Uses the GPU to draw the interface. In our tests, turning it off used about 15 MiB less memory but about 18% more CPU while
              scrolling. Takes effect the next time DiZero starts.
            </p>
            {#if app.info && app.info.hardwareAccelerationAtLaunch !== s.performance.hardwareAcceleration}
              <p class="restart">Restart DiZero to apply this change.</p>
            {/if}
          </div>
          <Switch
            checked={s.performance.hardwareAcceleration}
            label="Hardware acceleration"
            onchange={(v) => app.update((x) => (x.performance.hardwareAcceleration = v))}
          />
        </div>
        <div class="row">
          <div class="label"><span>Live measurements</span><p>Memory and CPU for every DiZero process.</p></div>
          <button class="btn btn-sm btn-ghost" onclick={() => app.go("performance")}>Open Performance Center</button>
        </div>
      </section>

      <section id="set-integrations">
        <h2>Integrations</h2>
        {#each workspace.integrations as i (i.id)}
          <div class="row">
            <div class="label">
              <span>{i.label}</span>
              <p>{i.description}</p>
            </div>
            <span class="badge" data-tone={i.status === "connected" ? "ok" : i.status === "planned" ? "accent" : "warn"}>
              {i.status === "connected" ? "Connected" : i.status === "planned" ? "Planned" : "Disconnected"}
            </span>
          </div>
        {/each}
      </section>

      <section id="set-plugins">
        <h2>Plugins</h2>
        <div class="row">
          <div class="label"><span>Developer mode</span><p>Show runtime details and debug logs for plugins.</p></div>
          <Switch checked={s.plugins.developerMode} label="Developer mode" onchange={(v) => app.update((x) => (x.plugins.developerMode = v))} />
        </div>
        <div class="row">
          <div class="label"><span>Plugin data</span><p>Everything plugins have stored on this computer: {stats ? formatBytes(stats.pluginStorageBytes) : "…"}.</p></div>
          <button
            class="btn btn-sm btn-danger"
            onclick={() =>
              (confirm = {
                title: "Clear all plugin data?",
                body: "Notes, counters and other data saved by plugins will be deleted. Plugins stay installed.",
                action: "Clear plugin data",
                run: () => guarded(() => api.dataClearPluginStorage(null), "Plugin data cleared"),
              })}>Clear plugin data</button
          >
        </div>
        <div class="row">
          <div class="label"><span>Manage plugins</span><p>Install, enable and review permissions.</p></div>
          <button class="btn btn-sm btn-ghost" onclick={() => app.go("plugins")}>Open Plugins</button>
        </div>
      </section>

      <section id="set-data">
        <h2>Storage and privacy</h2>
        <p class="lead">Everything DiZero stores stays on this computer. This build sends nothing over the network: no telemetry, no analytics, no accounts.</p>
        <div class="row">
          <div class="label"><span>Local database</span><p class="num">{stats ? `${formatBytes(stats.databaseBytes)}, ${stats.messages} messages in ${stats.conversations} conversations` : "…"}</p></div>
          <button class="btn btn-sm btn-ghost" onclick={() => guarded(() => api.dataVacuum(), "Database compacted")}>Compact</button>
        </div>
        <div class="row">
          <div class="label"><span>Trim message cache</span><p>Keep only the newest 1,000 messages in each conversation.</p></div>
          <button class="btn btn-sm btn-ghost" onclick={() => guarded(async () => { await api.dataPrune(1000); await workspace.refreshSnapshot(); }, "Message cache trimmed")}>Trim</button>
        </div>
        <div class="row">
          <div class="label"><span>Back up</span><p>Save a consistent copy of the database file.</p></div>
          <button
            class="btn btn-sm btn-ghost"
            onclick={async () => {
              try {
                const p = await api.dataBackup();
                if (p) toasts.success("Backup saved", p);
              } catch (e) {
                toasts.error("Backup failed", errorMessage(e));
              }
            }}>Back up…</button
          >
        </div>
        <div class="row">
          <div class="label"><span>Reset demo workspace</span><p>Restore the demo communities and delete messages you sent in them.</p></div>
          <button
            class="btn btn-sm btn-danger"
            onclick={() =>
              (confirm = {
                title: "Reset the demo workspace?",
                body: "Messages you sent in the demo will be deleted and the original conversations restored. Plugins and settings are kept.",
                action: "Reset demo",
                run: () =>
                  guarded(async () => {
                    await api.dataResetDemo();
                    workspace.convs = {};
                    await workspace.init();
                  }, "Demo workspace reset"),
              })}>Reset demo</button
          >
        </div>
      </section>

      <section id="set-shortcuts">
        <h2>Keyboard shortcuts</h2>
        <dl class="keys">
          {#each shortcuts as [k, d] (k)}
            <dt><kbd>{k}</kbd></dt>
            <dd>{d}</dd>
          {/each}
        </dl>
        <p class="planned">Custom key bindings are planned.</p>
      </section>

      <section id="set-advanced">
        <h2>Advanced</h2>
        <div class="row">
          <div class="label"><label for="log-level">Log level</label><p>How much detail goes to the log file.</p></div>
          <select
            id="log-level"
            class="field narrow"
            value={s.advanced.logLevel}
            onchange={(e) => app.update((x) => (x.advanced.logLevel = (e.currentTarget as HTMLSelectElement).value as LogLevel))}
          >
            <option value="error">Errors only</option>
            <option value="warn">Warnings</option>
            <option value="info">Information</option>
            <option value="debug">Debug</option>
          </select>
        </div>
        <div class="row">
          <div class="label"><span>Settings file</span><p>Move your preferences between computers.</p></div>
          <div class="btns">
            <button
              class="btn btn-sm btn-ghost"
              onclick={async () => {
                try {
                  const p = await api.settingsExport();
                  if (p) toasts.success("Settings exported", p);
                } catch (e) {
                  toasts.error("Export failed", errorMessage(e));
                }
              }}>Export…</button
            >
            <button
              class="btn btn-sm btn-ghost"
              onclick={async () => {
                try {
                  const v = await api.settingsImport();
                  if (v) {
                    app.applyView(v);
                    toasts.success("Settings imported");
                  }
                } catch (e) {
                  toasts.error("Import failed", errorMessage(e));
                }
              }}>Import…</button
            >
          </div>
        </div>
        <div class="row">
          <div class="label"><span>Reset settings</span><p>Restore every setting on this page to its default.</p></div>
          <button
            class="btn btn-sm btn-danger"
            onclick={() =>
              (confirm = {
                title: "Reset all settings?",
                body: "Appearance, notifications, performance and other preferences return to their defaults. Messages and plugins are kept.",
                action: "Reset settings",
                run: async () => {
                  try {
                    app.applyView(await api.settingsReset());
                    nameDraft = app.settings?.profile.displayName ?? "";
                    await workspace.refreshSnapshot();
                    toasts.success("Settings reset");
                  } catch (e) {
                    toasts.error("Reset failed", errorMessage(e));
                  }
                },
              })}>Reset settings</button
          >
        </div>
        {#if app.info}
          <dl class="about">
            <dt>DiZero</dt><dd>{app.info.version} ({app.info.buildProfile})</dd>
            <dt>Tauri</dt><dd>{app.info.tauriVersion}</dd>
            <dt>WebView2</dt><dd>{app.info.webviewVersion ?? "unknown"}</dd>
            <dt>Database schema</dt><dd>v{app.info.schemaVersion}</dd>
            <dt>Data folder</dt><dd><code>{app.info.dataDir}</code></dd>
          </dl>
        {/if}
      </section>
    </div>
  {/if}
</section>

{#if confirm}
  <Modal title={confirm.title} onClose={() => (confirm = null)}>
    <p class="muted">{confirm.body}</p>
    {#snippet footer()}
      <button class="btn btn-ghost" onclick={() => (confirm = null)}>Cancel</button>
      <button
        class="btn btn-danger"
        onclick={async () => {
          const c = confirm;
          confirm = null;
          await c?.run();
        }}>{confirm?.action}</button
      >
    {/snippet}
  </Modal>
{/if}

<style>
  .settings {
    flex-direction: row;
  }
  .toc {
    width: 220px;
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 22px 12px;
    border-right: 1px solid var(--dz-border);
    background: var(--dz-surface);
    overflow-y: auto;
  }
  .toc h1 {
    padding: 0 10px 14px;
  }
  .toc button {
    padding: 7px 10px;
    border-radius: var(--dz-r-control);
    color: var(--dz-text-secondary);
    text-align: left;
    font-size: var(--dz-text-sm);
    font-weight: 540;
  }
  .toc button:hover {
    background: var(--dz-sidebar-hover);
    color: var(--dz-text-primary);
  }
  .toc button.active {
    background: var(--dz-sidebar-active);
    color: var(--dz-text-primary);
  }
  .content {
    flex: 1;
    overflow-y: auto;
    padding: 8px 40px 80px;
  }
  .content > section {
    max-width: 760px;
    padding: 26px 0 10px;
    border-bottom: 1px solid var(--dz-border);
    scroll-margin-top: 8px;
  }
  .content h2 {
    margin-bottom: 10px;
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 24px;
    padding: 12px 0;
  }
  .row + .row {
    border-top: 1px solid color-mix(in srgb, var(--dz-border) 60%, transparent);
  }
  .row.column {
    flex-direction: column;
    align-items: stretch;
    gap: 12px;
  }
  .label {
    min-width: 0;
  }
  .label > span,
  .label > label {
    font-weight: 560;
  }
  .label p {
    margin-top: 2px;
    color: var(--dz-text-muted);
    font-size: var(--dz-text-sm);
    max-width: 60ch;
  }
  .narrow {
    width: 220px;
  }
  .label p.restart {
    color: var(--dz-warning);
    font-weight: 560;
  }
  .lead {
    color: var(--dz-text-secondary);
    font-size: var(--dz-text-sm);
    max-width: 64ch;
    margin-bottom: 6px;
  }
  .planned {
    margin: 4px 0 12px;
    color: var(--dz-text-muted);
    font-size: var(--dz-text-xs);
  }
  .themes {
    display: flex;
    gap: 12px;
    flex-wrap: wrap;
  }
  .theme {
    display: flex;
    flex-direction: column;
    gap: 6px;
    width: 140px;
    padding: 6px;
    border: 1px solid var(--dz-border);
    border-radius: 10px;
    text-align: left;
  }
  .theme[aria-checked="true"] {
    border-color: var(--dz-accent);
    box-shadow: 0 0 0 1px var(--dz-accent);
  }
  .theme.more {
    justify-content: center;
    align-items: center;
    min-height: 98px;
    border-style: dashed;
    color: var(--dz-text-muted);
    font-size: var(--dz-text-sm);
  }
  .preview {
    display: flex;
    gap: 4px;
    height: 64px;
    padding: 4px;
    border-radius: 6px;
  }
  .pv-side {
    width: 30%;
    border-radius: 3px;
  }
  .pv-main {
    flex: 1;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    gap: 4px;
    padding: 6px;
    border-radius: 3px;
  }
  .pv-line {
    height: 4px;
    width: 80%;
    border-radius: 2px;
    opacity: 0.7;
  }
  .pv-line.short {
    width: 45%;
    opacity: 1;
  }
  .theme-name {
    padding: 0 2px 2px;
    font-size: var(--dz-text-sm);
    font-weight: 560;
  }
  .accents {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .accent {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.15);
  }
  .accent[aria-pressed="true"] {
    outline: 2px solid var(--dz-text-primary);
    outline-offset: 2px;
  }
  .custom {
    position: relative;
    width: 24px;
    height: 24px;
    border-radius: 50%;
    background: conic-gradient(#e57883, #e8b96c, #65c99a, #6fb3e8, #9278f5, #e57883);
    cursor: pointer;
  }
  .custom input {
    position: absolute;
    inset: 0;
    opacity: 0;
    cursor: pointer;
  }
  input[type="range"] {
    width: 220px;
    accent-color: var(--dz-accent);
  }
  .btns {
    display: flex;
    gap: 8px;
  }
  .keys {
    display: grid;
    grid-template-columns: 160px 1fr;
    gap: 10px 16px;
    margin: 8px 0;
    font-size: var(--dz-text-sm);
  }
  .keys dd {
    margin: 0;
    color: var(--dz-text-secondary);
  }
  .about {
    display: grid;
    grid-template-columns: 160px 1fr;
    gap: 6px 16px;
    margin: 18px 0 8px;
    font-size: var(--dz-text-sm);
  }
  .about dt {
    color: var(--dz-text-muted);
  }
  .about dd {
    margin: 0;
    overflow-wrap: anywhere;
  }
  .about code {
    font-size: 12px;
  }
</style>
