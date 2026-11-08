<script lang="ts">
  import { onMount } from "svelte";
  import Meter from "../components/Meter.svelte";
  import Segmented from "../components/Segmented.svelte";
  import { api, errorMessage, type PerfView } from "../lib/api";
  import type { PerfMode } from "../lib/bindings/PerfMode";
  import type { ProcessMetrics } from "../lib/bindings/ProcessMetrics";
  import { app } from "../lib/stores/app.svelte";
  import { plugins } from "../lib/stores/plugins.svelte";
  import { toasts } from "../lib/stores/toasts.svelte";
  import { formatBytes, formatDuration } from "../lib/format";

  const MiB = 1024 * 1024;
  const TARGET = 100 * MiB;
  const HISTORY = 90;

  let view = $state<PerfView | null>(null);
  let error = $state<string | null>(null);
  let history = $state<number[]>([]);

  async function sample() {
    try {
      const v = await api.perfSnapshot();
      view = v;
      error = null;
      if (v.process.supported) history = [...history, v.process.totalPrivateWorkingSetBytes ?? v.process.totalWorkingSetBytes].slice(-HISTORY);
    } catch (e) {
      error = errorMessage(e);
    }
  }

  // Sample only while this page is open and the window is visible.
  $effect(() => {
    const interval = app.profile?.metricsIntervalMs ?? 2000;
    void sample();
    const t = setInterval(() => {
      if (document.visibilityState === "visible") void sample();
    }, interval);
    return () => clearInterval(t);
  });

  onMount(() => {
    if (!plugins.overview) void plugins.loadOverview();
  });

  /** Short strip label plus the detail line under it. */
  function role(p: ProcessMetrics, all: ProcessMetrics[], rootPid: number): { name: string; sub: string } {
    if (p.pid === rootPid) return { name: "DiZero core", sub: `PID ${p.pid}` };
    if (p.name.toLowerCase().startsWith("msedgewebview2")) {
      if (p.parentPid === rootPid) return { name: "Browser", sub: `PID ${p.pid}` };
      const helpers = all.filter((x) => x.pid !== rootPid && x.parentPid !== rootPid);
      return { name: `Helper ${helpers.indexOf(p) + 1}`, sub: `PID ${p.pid}` };
    }
    return { name: p.name.replace(/\.exe$/i, ""), sub: `PID ${p.pid}` };
  }

  const rootPid = $derived(view?.process.processes[0]?.pid ?? 0);
  const procs = $derived(view?.process.processes ?? []);
  // Primary metric: private working set (what Task Manager's Memory column
  // shows). Falls back to working set on Windows builds that don't report it.
  const usesPrivate = $derived(view?.process.totalPrivateWorkingSetBytes != null);
  const mem = (p: ProcessMetrics) => p.privateWorkingSetBytes ?? p.workingSetBytes;
  const total = $derived(view?.process.totalPrivateWorkingSetBytes ?? view?.process.totalWorkingSetBytes ?? 0);
  const totalScale = $derived(Math.max(200 * MiB, Math.ceil((total * 1.15) / (50 * MiB)) * 50 * MiB));
  const stripScale = $derived(Math.max(50 * MiB, ...procs.map((p) => Math.ceil((mem(p) * 1.2) / (25 * MiB)) * 25 * MiB)));

  const spark = $derived.by(() => {
    if (history.length < 2) return "";
    const max = Math.max(...history) * 1.08;
    const min = Math.min(...history) * 0.92;
    const span = Math.max(1, max - min);
    return history
      .map((v, i) => `${((i / (HISTORY - 1)) * 100).toFixed(2)},${(36 - ((v - min) / span) * 34).toFixed(2)}`)
      .join(" ");
  });

  const modes: { value: PerfMode; label: string }[] = [
    { value: "eco", label: "Eco" },
    { value: "balanced", label: "Balanced" },
    { value: "performance", label: "Performance" },
  ];

  const pluginName = (id: string) => plugins.overview?.installed.find((p) => p.plugin.manifest.id === id)?.plugin.manifest.name ?? id;

  async function compact() {
    try {
      const s = await api.dataVacuum();
      toasts.success("Database compacted", `Now ${formatBytes(s.databaseBytes)}.`);
      void sample();
    } catch (e) {
      toasts.error("Couldn't compact the database", errorMessage(e));
    }
  }
</script>

<section class="page" aria-labelledby="perf-title">
  <header class="page-header">
    <div class="titles">
      <h1 id="perf-title">Performance Center</h1>
      <p>Measured live from Windows for DiZero and every WebView2 process it started.</p>
    </div>
    {#if app.settings}
      <Segmented
        label="Performance mode"
        value={app.settings.performance.mode}
        options={modes}
        onchange={(m) => app.update((s) => (s.performance.mode = m))}
      />
    {/if}
  </header>

  <div class="page-body">
    {#if error}
      <p class="error" role="alert">Couldn't read metrics: {error}</p>
    {/if}

    {#if view && !view.process.supported}
      <p class="error">Process metrics are only implemented for Windows in this build.</p>
    {/if}

    <!-- The mixer -->
    <div class="console" aria-label="Memory by process">
      <div class="strip master">
        <span class="strip-name">Total</span>
        <span class="strip-sub num">{procs.length} process{procs.length === 1 ? "" : "es"}</span>
        <div class="meter-row">
          <Meter value={total} max={totalScale} marker={TARGET} segments={30} height={210} label="Total memory" />
          <div class="scale num" aria-hidden="true">
            <span>{formatBytes(totalScale)}</span>
            <span>{formatBytes(totalScale / 2)}</span>
            <span>0</span>
          </div>
        </div>
        <span class="readout num" class:over={total > TARGET}>{view ? formatBytes(total) : "…"}</span>
        <span class="strip-sub">{usesPrivate ? "private working set" : "working set"}, target {formatBytes(TARGET)}</span>
      </div>

      <div class="strips">
        {#each procs as p (p.pid)}
          {@const r = role(p, procs, rootPid)}
          <div class="strip">
            <span class="strip-name" title={p.name}>{r.name}</span>
            <span class="strip-sub num">{r.sub}</span>
            <Meter value={mem(p)} max={stripScale} segments={20} height={150} label="{r.name} memory" />
            <span class="readout small num">{formatBytes(mem(p))}</span>
            <span class="strip-sub num" title="Working set including shared pages">{formatBytes(p.workingSetBytes)} incl. shared</span>
            <div class="cpu" title="CPU">
              <span class="cpu-bar"><span style:width="{Math.min(100, (p.cpuPercent ?? 0) * 4)}%"></span></span>
              <span class="num">{p.cpuPercent === null ? "…" : `${p.cpuPercent.toFixed(1)}%`}</span>
            </div>
          </div>
        {:else}
          {#each [1, 2, 3, 4] as i (i)}<div class="strip"><div class="skeleton" style:width="14px" style:height="150px"></div></div>{/each}
        {/each}
      </div>
    </div>
    <p class="caption">
      DiZero core is the Rust process. Browser and Helpers are the Microsoft Edge WebView2 processes that draw the interface: the GPU
      process, the page renderer and utility processes.
    </p>

    <div class="readouts">
      <div class="ro">
        <span class="ro-label">Committed</span>
        <span class="ro-value num">{view ? formatBytes(view.process.totalPrivateBytes) : "…"}</span>
        <span class="ro-hint num">Private bytes; {view ? formatBytes(view.process.totalWorkingSetBytes) : "…"} working set incl. shared</span>
      </div>
      <div class="ro">
        <span class="ro-label">CPU</span>
        <span class="ro-value num">{view?.process.totalCpuPercent == null ? "…" : `${view.process.totalCpuPercent.toFixed(1)}%`}</span>
        <span class="ro-hint num">Of {view?.process.logicalCpus ?? "?"} logical CPUs</span>
      </div>
      <div class="ro">
        <span class="ro-label">Startup</span>
        <span class="ro-value num">{view?.startupMs ? formatDuration(view.startupMs) : "…"}</span>
        <span class="ro-hint">Launch to first paint</span>
      </div>
      <div class="ro trend">
        <span class="ro-label">Memory trend</span>
        <svg viewBox="0 0 100 38" preserveAspectRatio="none" aria-label="Working set over the last samples">
          {#if spark}<polyline points={spark} />{/if}
        </svg>
        <span class="ro-hint num">Uptime {view ? formatDuration(view.uptimeMs) : "…"}</span>
      </div>
    </div>

    <div class="grid">
      <section class="panel">
        <h2>What this mode changes</h2>
        {#if view}
          <dl class="table">
            <dt>Messages kept per open conversation</dt><dd class="num">{view.profile.messageWindow}</dd>
            <dt>Messages fetched per page</dt><dd class="num">{view.profile.pageSize}</dd>
            <dt>Animations</dt><dd>{view.profile.animations ? "On" : "Off"}</dd>
            <dt>Prefetch channels on hover</dt><dd>{view.profile.preloadAdjacent ? "On" : "Off"}</dd>
            <dt>Unload idle plugin runtimes</dt><dd>{view.profile.pluginIdleUnloadSecs === null ? "Never" : `After ${formatDuration(view.profile.pluginIdleUnloadSecs * 1000)}`}</dd>
            <dt>Sampling on this page</dt><dd class="num">Every {formatDuration(view.profile.metricsIntervalMs)}</dd>
          </dl>
          <p class="note">Sending, receiving and notifications work the same in every mode.</p>
        {/if}
      </section>

      <section class="panel">
        <h2>Background activity</h2>
        <ul class="activity">
          <li><span class="led" data-tone="ok"></span>Plugin host: one thread, asleep until a plugin is called</li>
          <li>
            <span class="led" data-tone={view?.pendingReplies ? "accent" : "off"}></span>
            Demo replies scheduled: <span class="num">{view?.pendingReplies ?? 0}</span>
          </li>
          <li><span class="led" data-tone="accent"></span>Metrics sampling: only while this page is open</li>
          <li><span class="led" data-tone="off"></span>Network: none. This build has no telemetry and no remote services</li>
        </ul>
      </section>

      <section class="panel wide">
        <h2>Plugins</h2>
        {#if view && view.plugins.length}
          <table>
            <thead>
              <tr><th>Plugin</th><th>State</th><th class="r">JS heap</th><th class="r">Calls</th><th class="r">Avg time</th><th class="r">Failures</th></tr>
            </thead>
            <tbody>
              {#each view.plugins as s (s.id)}
                <tr>
                  <td>{pluginName(s.id)}</td>
                  <td>
                    <span class="led" data-tone={s.state === "active" ? "ok" : s.state === "faulted" ? "bad" : "off"}></span>
                    {s.state === "active" ? "Loaded" : s.state === "faulted" ? "Stopped" : "Not loaded"}
                  </td>
                  <td class="r num">{s.heapBytes === null ? "0 B" : formatBytes(s.heapBytes)}</td>
                  <td class="r num">{s.calls}</td>
                  <td class="r num">{s.calls ? `${(s.totalTimeUs / s.calls / 1000).toFixed(2)} ms` : "–"}</td>
                  <td class="r num" class:bad={s.failures > 0}>{s.failures}</td>
                </tr>
              {/each}
            </tbody>
          </table>
          <p class="note">JS heap is measured by each plugin's QuickJS runtime. Unloaded plugins use no heap.</p>
        {:else}
          <p class="note">No script plugins are enabled. Themes don't run code.</p>
        {/if}
      </section>

      <section class="panel wide">
        <h2>Local storage</h2>
        {#if view}
          <dl class="table">
            <dt>Database file</dt><dd class="num">{formatBytes(view.storage.databaseBytes)}</dd>
            <dt>Cached messages</dt><dd class="num">{view.storage.messages}</dd>
            <dt>Conversations</dt><dd class="num">{view.storage.conversations}</dd>
            <dt>Plugin storage</dt><dd class="num">{formatBytes(view.storage.pluginStorageBytes)}</dd>
          </dl>
          <button class="btn btn-sm btn-ghost" onclick={compact}>Compact database</button>
        {/if}
      </section>
    </div>

    <p class="method">
      How this is measured: DiZero walks its own process tree with the Windows Toolhelp API and reads each process with
      GetProcessMemoryInfo. The headline figure is private working set, the memory only these processes use and the number Task Manager
      shows. Working set additionally counts libraries shared between processes, so summing it across the tree over-counts them.
      Committed is private bytes. CPU is kernel plus user time between samples divided by the number of logical CPUs.
    </p>
  </div>
</section>

<style>
  .titles {
    flex: 1;
    min-width: 0;
  }
  .error {
    margin-bottom: 14px;
    color: var(--dz-danger);
  }
  .console {
    display: flex;
    gap: 2px;
    padding: 18px;
    border: 1px solid var(--dz-border);
    border-radius: var(--dz-r-tile);
    background: linear-gradient(180deg, #121217 0%, var(--dz-background) 100%);
    overflow-x: auto;
  }
  .strips {
    display: flex;
    gap: 2px;
  }
  .strip {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    width: 128px;
    padding: 10px 8px 12px;
    border-radius: 8px;
    background: rgba(255, 255, 255, 0.015);
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.03);
  }
  .strip.master {
    width: 168px;
    margin-right: 14px;
    background: rgba(var(--dz-accent-rgb), 0.05);
    box-shadow: inset 0 0 0 1px rgba(var(--dz-accent-rgb), 0.18);
  }
  .strip-name {
    max-width: 100%;
    font-size: var(--dz-text-sm);
    font-weight: 620;
    text-align: center;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .strip-sub {
    color: var(--dz-text-muted);
    font-size: 11px;
  }
  .meter-row {
    display: flex;
    gap: 8px;
    align-items: stretch;
    margin: 4px 0;
  }
  .scale {
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    color: var(--dz-text-muted);
    font-size: 10px;
  }
  .readout {
    font-size: var(--dz-text-xl);
    font-weight: 650;
    letter-spacing: -0.01em;
  }
  .readout.small {
    font-size: var(--dz-text-md);
  }
  .readout.over {
    color: var(--dz-warning);
  }
  .cpu {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    font-size: 11px;
    color: var(--dz-text-secondary);
  }
  .cpu-bar {
    flex: 1;
    height: 4px;
    border-radius: 2px;
    background: #1a1a20;
    overflow: hidden;
  }
  .cpu-bar span {
    display: block;
    height: 100%;
    background: var(--dz-accent);
  }
  .caption {
    margin-top: 8px;
    color: var(--dz-text-muted);
    font-size: var(--dz-text-xs);
  }
  .readouts {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    margin: 16px 0;
    border: 1px solid var(--dz-border);
    border-radius: var(--dz-r-tile);
    background: var(--dz-surface);
  }
  .ro {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 14px 16px;
    min-width: 0;
  }
  .ro + .ro {
    border-left: 1px solid var(--dz-border);
  }
  .ro-label {
    color: var(--dz-text-muted);
    font-size: var(--dz-text-xs);
    font-weight: 600;
  }
  .ro-value {
    font-size: var(--dz-text-2xl);
    font-weight: 640;
    letter-spacing: -0.02em;
    line-height: 1.15;
  }
  .ro-hint {
    color: var(--dz-text-muted);
    font-size: 11px;
  }
  .trend svg {
    width: 100%;
    height: 38px;
  }
  .trend polyline {
    fill: none;
    stroke: var(--dz-accent);
    stroke-width: 1.5;
    vector-effect: non-scaling-stroke;
  }
  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
  }
  .panel {
    padding: 16px 18px;
    border: 1px solid var(--dz-border);
    border-radius: var(--dz-r-tile);
    background: var(--dz-surface);
  }
  .panel.wide {
    grid-column: span 1;
  }
  .panel h2 {
    margin-bottom: 12px;
    font-size: var(--dz-text-md);
  }
  .table {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 7px 16px;
    margin: 0 0 12px;
    font-size: var(--dz-text-sm);
  }
  .table dt {
    color: var(--dz-text-secondary);
  }
  .table dd {
    margin: 0;
    text-align: right;
    font-weight: 560;
  }
  .note {
    color: var(--dz-text-muted);
    font-size: var(--dz-text-xs);
    margin-top: 8px;
  }
  .activity {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 10px;
    font-size: var(--dz-text-sm);
    color: var(--dz-text-secondary);
  }
  .activity li {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--dz-text-sm);
  }
  th {
    padding: 0 0 8px;
    color: var(--dz-text-muted);
    font-size: var(--dz-text-xs);
    font-weight: 600;
    text-align: left;
  }
  td {
    padding: 7px 0;
    border-top: 1px solid var(--dz-border);
  }
  td .led {
    width: 7px;
    height: 7px;
    margin-right: 6px;
  }
  .r {
    text-align: right;
  }
  .bad {
    color: var(--dz-danger);
  }
  .method {
    max-width: 90ch;
    margin-top: 20px;
    color: var(--dz-text-muted);
    font-size: var(--dz-text-xs);
    line-height: 1.6;
  }
  @media (max-width: 1150px) {
    .readouts {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
    .ro:nth-child(3) {
      border-left: 0;
    }
    .ro:nth-child(n + 3) {
      border-top: 1px solid var(--dz-border);
    }
    .grid {
      grid-template-columns: 1fr;
    }
  }
</style>
