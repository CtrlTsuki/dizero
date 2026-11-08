//! Sandboxed plugin runtime host (QuickJS-NG via rquickjs).
//!
//! # Isolation model
//! * Every plugin gets its **own QuickJS runtime** (separate heap, GC and
//!   memory limit). Plugins cannot see each other's objects.
//! * The sandbox contains only ECMAScript built-ins plus the `dz` API object.
//!   There is no filesystem, network, process, timer or DOM access: QuickJS's
//!   `std`/`os` modules are never loaded.
//! * Native host functions are installed only for permissions the user
//!   granted, and each one re-checks its grant.
//! * Every call into a plugin runs under a wall-clock budget enforced by the
//!   QuickJS interrupt handler, so infinite loops are terminated.
//! * All runtimes live on one dedicated host thread. A misbehaving plugin can
//!   at worst consume its own time budget; it cannot block the UI thread or
//!   crash the app. Rust panics inside a call are caught and fault the plugin.
//! * Plugins are lazy: registered plugins stay `Dormant` (no memory) until
//!   first use, and can be unloaded again when idle.

use super::manifest::PluginManifest;
use super::permissions::Permission;
use super::view::{ViewNode, sanitize};
use crate::error::{CoreError, Result};
use rquickjs::{Context, Ctx, Function, Object, Runtime};
use serde::{Deserialize, Serialize};
use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use ts_rs::TS;

/// Plugin-private key/value persistence.
pub trait PluginStorage: Send + Sync {
    fn get(&self, plugin_id: &str, key: &str) -> Result<Option<String>>;
    fn set(&self, plugin_id: &str, key: &str, value: &str) -> Result<()>;
    fn delete(&self, plugin_id: &str, key: &str) -> Result<()>;
    fn keys(&self, plugin_id: &str) -> Result<Vec<String>>;
    /// Bytes used (keys + values) by one plugin.
    fn usage_bytes(&self, plugin_id: &str) -> Result<usize>;
}

/// Where host-side effects go (the desktop shell forwards these to the UI).
pub trait HostSink: Send + Sync {
    fn notify(&self, plugin_id: &str, title: &str, body: &str);
    fn refresh(&self, plugin_id: &str, panel: &str);
    fn log(&self, plugin_id: &str, level: log::Level, msg: &str);
    fn state_changed(&self, plugin_id: &str, state: RuntimeState, error: Option<&str>);
}

/// In-memory storage, used by tests and the plugin validator.
#[derive(Default)]
pub struct MemoryPluginStorage {
    data: Mutex<HashMap<(String, String), String>>,
}

impl PluginStorage for MemoryPluginStorage {
    fn get(&self, p: &str, k: &str) -> Result<Option<String>> {
        Ok(self.data.lock().unwrap_or_else(|e| e.into_inner()).get(&(p.into(), k.into())).cloned())
    }
    fn set(&self, p: &str, k: &str, v: &str) -> Result<()> {
        self.data.lock().unwrap_or_else(|e| e.into_inner()).insert((p.into(), k.into()), v.into());
        Ok(())
    }
    fn delete(&self, p: &str, k: &str) -> Result<()> {
        self.data.lock().unwrap_or_else(|e| e.into_inner()).remove(&(p.into(), k.into()));
        Ok(())
    }
    fn keys(&self, p: &str) -> Result<Vec<String>> {
        let data = self.data.lock().unwrap_or_else(|e| e.into_inner());
        let mut keys: Vec<String> = data.keys().filter(|(pid, _)| pid == p).map(|(_, k)| k.clone()).collect();
        keys.sort();
        Ok(keys)
    }
    fn usage_bytes(&self, p: &str) -> Result<usize> {
        let data = self.data.lock().unwrap_or_else(|e| e.into_inner());
        Ok(data.iter().filter(|((pid, _), _)| pid == p).map(|((_, k), v)| k.len() + v.len()).sum())
    }
}

#[derive(Debug, Clone)]
pub struct HostLimits {
    pub memory_bytes: usize,
    pub call_budget: Duration,
    pub activate_budget: Duration,
    pub storage_quota_bytes: usize,
    pub max_failures: u32,
}

impl Default for HostLimits {
    fn default() -> Self {
        HostLimits {
            memory_bytes: 16 * 1024 * 1024,
            call_budget: Duration::from_millis(200),
            activate_budget: Duration::from_millis(750),
            storage_quota_bytes: 512 * 1024,
            max_failures: 5,
        }
    }
}

pub struct LoadRequest {
    pub manifest: PluginManifest,
    pub source: String,
    pub granted: BTreeSet<Permission>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum RuntimeState {
    /// Registered; no runtime allocated yet (lazy).
    Dormant,
    Active,
    /// Disabled after repeated failures until reset.
    Faulted,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PluginRuntimeStats {
    pub id: String,
    pub state: RuntimeState,
    /// QuickJS heap bytes for this plugin's runtime; `None` when not loaded.
    #[ts(type = "number | null")]
    pub heap_bytes: Option<i64>,
    #[ts(type = "number")]
    pub calls: u64,
    #[ts(type = "number")]
    pub failures: u64,
    #[ts(type = "number")]
    pub total_time_us: u64,
    #[ts(type = "number")]
    pub activations: u64,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct CommandOutcome {
    /// Replace the composer text with this value.
    pub replace_with: Option<String>,
    /// A local-only note shown to the user; never sent.
    pub ephemeral: Option<String>,
}

// ---------------------------------------------------------------------------
// Public handle
// ---------------------------------------------------------------------------

type Reply<T> = mpsc::Sender<Result<T>>;

enum Job {
    Register(Box<LoadRequest>, Reply<()>),
    Unregister(String, Reply<()>),
    Reset(String, Reply<()>),
    Command { id: String, name: String, args: String, reply: Reply<CommandOutcome> },
    Render { id: String, panel: String, reply: Reply<Vec<ViewNode>> },
    Action { id: String, panel: String, action: String, values: String, reply: Reply<Vec<ViewNode>> },
    Message(String),
    SetIdle(Option<Duration>),
    Stats(Reply<Vec<PluginRuntimeStats>>),
    Flush(Reply<()>),
    Shutdown,
}

pub struct PluginHost {
    tx: mpsc::Sender<Job>,
    thread: Option<JoinHandle<()>>,
    reply_timeout: Duration,
}

impl PluginHost {
    pub fn start(storage: Arc<dyn PluginStorage>, sink: Arc<dyn HostSink>, limits: HostLimits) -> PluginHost {
        let (tx, rx) = mpsc::channel::<Job>();
        // Generous upper bound for a caller waiting on a reply: a call can
        // take at most its budget, plus a lazy activation.
        let reply_timeout = limits.activate_budget + limits.call_budget * 4 + Duration::from_secs(5);
        let thread = std::thread::Builder::new()
            .name("dz-plugin-host".into())
            .stack_size(8 * 1024 * 1024)
            .spawn(move || {
                let mut worker = Worker { slots: BTreeMap::new(), storage, sink, limits, idle_after: None };
                worker.run(rx);
            })
            .expect("failed to spawn plugin host thread");
        PluginHost { tx, thread: Some(thread), reply_timeout }
    }

    fn request<T>(&self, make: impl FnOnce(Reply<T>) -> Job) -> Result<T> {
        let (rtx, rrx) = mpsc::channel();
        self.tx.send(make(rtx)).map_err(|_| CoreError::Plugin("plugin host is not running".into()))?;
        match rrx.recv_timeout(self.reply_timeout) {
            Ok(r) => r,
            Err(RecvTimeoutError::Timeout) => Err(CoreError::Plugin("plugin host did not respond in time".into())),
            Err(RecvTimeoutError::Disconnected) => Err(CoreError::Plugin("plugin host stopped unexpectedly".into())),
        }
    }

    pub fn register(&self, req: LoadRequest) -> Result<()> {
        self.request(|r| Job::Register(Box::new(req), r))
    }
    pub fn unregister(&self, id: &str) -> Result<()> {
        self.request(|r| Job::Unregister(id.into(), r))
    }
    /// Clear a faulted plugin back to `Dormant` so it can be retried.
    pub fn reset(&self, id: &str) -> Result<()> {
        self.request(|r| Job::Reset(id.into(), r))
    }
    pub fn run_command(&self, id: &str, name: &str, args: &str) -> Result<CommandOutcome> {
        self.request(|reply| Job::Command { id: id.into(), name: name.into(), args: args.into(), reply })
    }
    pub fn render_panel(&self, id: &str, panel: &str) -> Result<Vec<ViewNode>> {
        self.request(|reply| Job::Render { id: id.into(), panel: panel.into(), reply })
    }
    pub fn panel_action(
        &self,
        id: &str,
        panel: &str,
        action: &str,
        values: serde_json::Value,
    ) -> Result<Vec<ViewNode>> {
        let values = serde_json::to_string(&values)?;
        self.request(|reply| Job::Action { id: id.into(), panel: panel.into(), action: action.into(), values, reply })
    }
    /// Fire-and-forget delivery of a message event (JSON of [`PluginMessage`]).
    pub fn dispatch_message_json(&self, json: &str) {
        let _ = self.tx.send(Job::Message(json.into()));
    }
    pub fn dispatch_message(&self, msg: &PluginMessage) {
        if let Ok(json) = serde_json::to_string(msg) {
            self.dispatch_message_json(&json);
        }
    }
    pub fn set_idle_unload(&self, after: Option<Duration>) {
        let _ = self.tx.send(Job::SetIdle(after));
    }
    pub fn stats(&self) -> Vec<PluginRuntimeStats> {
        self.request(Job::Stats).unwrap_or_default()
    }
    /// Block until every job queued before this call has been processed.
    pub fn flush(&self) {
        let _ = self.request(Job::Flush);
    }
}

impl Drop for PluginHost {
    fn drop(&mut self) {
        let _ = self.tx.send(Job::Shutdown);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

/// The message shape plugins receive. Deliberately minimal: no attachment
/// URLs, no provider ids beyond opaque strings.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PluginMessage {
    pub id: String,
    pub conversation_id: String,
    pub author_id: String,
    pub author_name: String,
    pub is_self: bool,
    pub is_bot: bool,
    pub content: String,
    #[ts(type = "number")]
    pub created_at: i64,
}

impl From<&crate::models::Message> for PluginMessage {
    fn from(m: &crate::models::Message) -> Self {
        PluginMessage {
            id: m.id.clone(),
            conversation_id: m.conversation_id.clone(),
            author_id: m.author.id.clone(),
            author_name: m.author.display_name.clone(),
            is_self: m.author.is_self,
            is_bot: m.author.is_bot,
            content: m.content.clone(),
            created_at: m.created_at,
        }
    }
}

// ---------------------------------------------------------------------------
// Host thread
// ---------------------------------------------------------------------------

const MAX_SOURCE_BYTES: usize = 2 * 1024 * 1024;
const MAX_RESULT_BYTES: usize = 256 * 1024;
const MAX_STORAGE_VALUE: usize = 64 * 1024;

/// JavaScript prelude evaluated before plugin code. It wraps the native host
/// functions (installed as `__dz_host` and then deleted from the global
/// scope), exposes the frozen `dz` API, and installs a non-writable
/// dispatcher the host uses to call into the plugin.
const PRELUDE: &str = r#"
(function () {
  "use strict";
  const H = globalThis.__dz_host;
  delete globalThis.__dz_host;
  const unwrap = (raw) => { const r = JSON.parse(raw); if (!r.ok) throw new Error(r.e); return r.v; };
  const denied = (perm) => () => { throw new Error("permission denied: `" + perm + "` was not granted to this plugin"); };
  const fmt = (args) => args.map((a) => {
    if (typeof a === "string") return a;
    try { return JSON.stringify(a); } catch (_) { return String(a); }
  }).join(" ");
  let definition = null;
  const dz = {
    apiVersion: "1",
    plugin(def) {
      if (definition !== null) throw new Error("dz.plugin() can only be called once");
      if (typeof def !== "object" || def === null) throw new TypeError("dz.plugin() expects an object");
      definition = def;
    },
    log: Object.freeze({
      info: (...a) => { H.log(2, fmt(a)); },
      warn: (...a) => { H.log(1, fmt(a)); },
      error: (...a) => { H.log(0, fmt(a)); },
    }),
    storage: Object.freeze(H.sget ? {
      get: (k) => { const v = unwrap(H.sget(String(k))); return v === null ? undefined : JSON.parse(v); },
      set: (k, v) => {
        if (v === undefined) throw new TypeError("storage values must be JSON-serialisable");
        unwrap(H.sset(String(k), JSON.stringify(v)));
      },
      delete: (k) => { unwrap(H.sdel(String(k))); },
      keys: () => unwrap(H.skeys()),
    } : { get: denied("storage.local"), set: denied("storage.local"), delete: denied("storage.local"), keys: denied("storage.local") }),
    notify: H.notify
      ? (title, body) => { unwrap(H.notify(String(title), body === undefined ? "" : String(body))); }
      : denied("notifications"),
    ui: Object.freeze({
      refresh: H.refresh ? (panel) => { unwrap(H.refresh(String(panel))); } : denied("ui.panels"),
    }),
  };
  Object.defineProperty(globalThis, "dz", { value: Object.freeze(dz), writable: false, configurable: false, enumerable: false });
  const out = (v) => JSON.stringify(v === undefined ? null : v);
  Object.defineProperty(globalThis, "__dz_call", {
    writable: false, configurable: false, enumerable: false,
    value: function (kind, a, b, c) {
      const d = definition;
      if (d === null) throw new Error("plugin never called dz.plugin({...})");
      switch (kind) {
        case "activate": return out(typeof d.activate === "function" ? d.activate() : null);
        case "deactivate": return out(typeof d.deactivate === "function" ? d.deactivate() : null);
        case "command": {
          const cmd = d.commands && Object.prototype.hasOwnProperty.call(d.commands, a) ? d.commands[a] : null;
          if (!cmd || typeof cmd.run !== "function") throw new Error("command `" + a + "` is not implemented");
          return out(cmd.run(b));
        }
        case "event": {
          const h = d.events && Object.prototype.hasOwnProperty.call(d.events, a) ? d.events[a] : null;
          if (typeof h === "function") h(JSON.parse(b));
          return "null";
        }
        case "render": {
          const p = d.panels && Object.prototype.hasOwnProperty.call(d.panels, a) ? d.panels[a] : null;
          if (!p || typeof p.render !== "function") throw new Error("panel `" + a + "` is not implemented");
          return out(p.render());
        }
        case "action": {
          const p = d.panels && Object.prototype.hasOwnProperty.call(d.panels, a) ? d.panels[a] : null;
          if (!p || typeof p.action !== "function") throw new Error("panel `" + a + "` has no action handler");
          p.action(b, JSON.parse(c));
          return out(typeof p.render === "function" ? p.render() : []);
        }
        default: throw new Error("unknown host call " + kind);
      }
    },
  });
})();
"#;

struct Loaded {
    // Field order matters: the context must drop before its runtime.
    ctx: Context,
    rt: Runtime,
    deadline: Rc<Cell<Option<Instant>>>,
}

struct Slot {
    manifest: PluginManifest,
    source: String,
    granted: BTreeSet<Permission>,
    loaded: Option<Loaded>,
    state: RuntimeState,
    calls: u64,
    failures: u64,
    consecutive_failures: u32,
    total_time_us: u64,
    activations: u64,
    last_error: Option<String>,
    last_used: Instant,
}

struct Worker {
    slots: BTreeMap<String, Slot>,
    storage: Arc<dyn PluginStorage>,
    sink: Arc<dyn HostSink>,
    limits: HostLimits,
    idle_after: Option<Duration>,
}

fn envelope_ok(v: serde_json::Value) -> String {
    serde_json::json!({ "ok": true, "v": v }).to_string()
}
fn envelope_err(e: impl std::fmt::Display) -> String {
    serde_json::json!({ "ok": false, "e": e.to_string() }).to_string()
}

/// Sliding one-second-window rate limiter for host functions.
#[derive(Clone)]
struct RateLimit {
    window: Rc<Cell<(Instant, u32)>>,
    per_window: u32,
    window_len: Duration,
}

impl RateLimit {
    fn new(per_window: u32, window_len: Duration) -> Self {
        RateLimit { window: Rc::new(Cell::new((Instant::now(), 0))), per_window, window_len }
    }
    fn allow(&self) -> bool {
        let (start, n) = self.window.get();
        if start.elapsed() >= self.window_len {
            self.window.set((Instant::now(), 1));
            true
        } else if n < self.per_window {
            self.window.set((start, n + 1));
            true
        } else {
            false
        }
    }
}

fn describe_error(ctx: &Ctx<'_>, err: rquickjs::Error) -> String {
    if let rquickjs::Error::Exception = err {
        let v = ctx.catch();
        if let Some(ex) = v.as_exception() {
            return ex.message().unwrap_or_else(|| "unknown exception".into());
        }
        if let Some(s) = v.as_string().and_then(|s| s.to_string().ok()) {
            return s;
        }
        if v.is_null() || v.is_undefined() {
            return "uncatchable exception (out of memory or interrupted)".into();
        }
        return format!("{v:?}");
    }
    err.to_string()
}

impl Worker {
    fn run(&mut self, rx: mpsc::Receiver<Job>) {
        loop {
            let job = match self.next_idle_wait() {
                Some(wait) => match rx.recv_timeout(wait) {
                    Ok(j) => Some(j),
                    Err(RecvTimeoutError::Timeout) => None,
                    Err(RecvTimeoutError::Disconnected) => break,
                },
                None => match rx.recv() {
                    Ok(j) => Some(j),
                    Err(_) => break,
                },
            };
            if let Some(job) = job {
                if matches!(job, Job::Shutdown) {
                    break;
                }
                self.handle(job);
            }
            self.unload_idle();
        }
        // Graceful shutdown: give active plugins a chance to clean up.
        let ids: Vec<String> = self.slots.keys().cloned().collect();
        for id in ids {
            self.deactivate(&id);
        }
    }

    fn handle(&mut self, job: Job) {
        match job {
            Job::Register(req, reply) => {
                let _ = reply.send(self.register(*req));
            }
            Job::Unregister(id, reply) => {
                self.deactivate(&id);
                let r = match self.slots.remove(&id) {
                    Some(_) => Ok(()),
                    None => Err(CoreError::NotFound(format!("plugin {id}"))),
                };
                let _ = reply.send(r);
            }
            Job::Reset(id, reply) => {
                let r = match self.slots.get_mut(&id) {
                    Some(slot) => {
                        slot.loaded = None;
                        slot.state = RuntimeState::Dormant;
                        slot.consecutive_failures = 0;
                        Ok(())
                    }
                    None => Err(CoreError::NotFound(format!("plugin {id}"))),
                };
                if r.is_ok() {
                    self.sink.state_changed(&id, RuntimeState::Dormant, None);
                }
                let _ = reply.send(r);
            }
            Job::Command { id, name, args, reply } => {
                let _ = reply.send(self.command(&id, &name, &args));
            }
            Job::Render { id, panel, reply } => {
                let _ = reply.send(self.render(&id, &panel, None));
            }
            Job::Action { id, panel, action, values, reply } => {
                let _ = reply.send(self.render(&id, &panel, Some((&action, &values))));
            }
            Job::Message(json) => self.message(&json),
            Job::SetIdle(after) => self.idle_after = after,
            Job::Stats(reply) => {
                let _ = reply.send(Ok(self.stats()));
            }
            Job::Flush(reply) => {
                let _ = reply.send(Ok(()));
            }
            Job::Shutdown => {}
        }
    }

    fn register(&mut self, req: LoadRequest) -> Result<()> {
        if req.source.len() > MAX_SOURCE_BYTES {
            return Err(CoreError::invalid("plugin code exceeds the 2 MiB limit"));
        }
        let id = req.manifest.id.clone();
        self.deactivate(&id);
        self.slots.insert(
            id,
            Slot {
                manifest: req.manifest,
                source: req.source,
                granted: req.granted,
                loaded: None,
                state: RuntimeState::Dormant,
                calls: 0,
                failures: 0,
                consecutive_failures: 0,
                total_time_us: 0,
                activations: 0,
                last_error: None,
                last_used: Instant::now(),
            },
        );
        Ok(())
    }

    fn stats(&self) -> Vec<PluginRuntimeStats> {
        self.slots
            .iter()
            .map(|(id, s)| PluginRuntimeStats {
                id: id.clone(),
                state: s.state,
                heap_bytes: s.loaded.as_ref().map(|l| l.rt.memory_usage().malloc_size),
                calls: s.calls,
                failures: s.failures,
                total_time_us: s.total_time_us,
                activations: s.activations,
                last_error: s.last_error.clone(),
            })
            .collect()
    }

    fn next_idle_wait(&self) -> Option<Duration> {
        let after = self.idle_after?;
        self.slots
            .values()
            .filter(|s| s.loaded.is_some())
            .map(|s| (s.last_used + after).saturating_duration_since(Instant::now()))
            .min()
            .map(|d| d.max(Duration::from_millis(10)))
    }

    fn unload_idle(&mut self) {
        let Some(after) = self.idle_after else { return };
        let idle: Vec<String> = self
            .slots
            .iter()
            .filter(|(_, s)| s.loaded.is_some() && s.last_used.elapsed() >= after)
            .map(|(id, _)| id.clone())
            .collect();
        for id in idle {
            log::debug!("unloading idle plugin runtime {id}");
            self.deactivate(&id);
        }
    }

    /// Call `deactivate` (best effort) and free the runtime.
    fn deactivate(&mut self, id: &str) {
        let budget = self.limits.call_budget;
        let Some(slot) = self.slots.get_mut(id) else { return };
        if let Some(loaded) = slot.loaded.take() {
            loaded.deadline.set(Some(Instant::now() + budget));
            let _ = catch_unwind(AssertUnwindSafe(|| {
                loaded.ctx.with(|ctx| {
                    if let Ok(f) = ctx.globals().get::<_, Function>("__dz_call") {
                        let _ = f.call::<_, String>(("deactivate", "", "", ""));
                    }
                })
            }));
            drop(loaded);
            if slot.state == RuntimeState::Active {
                slot.state = RuntimeState::Dormant;
                self.sink.state_changed(id, RuntimeState::Dormant, None);
            }
        }
    }

    fn build_runtime(&self, slot: &Slot) -> std::result::Result<Loaded, String> {
        let rt = Runtime::new().map_err(|e| e.to_string())?;
        rt.set_memory_limit(self.limits.memory_bytes);
        rt.set_max_stack_size(512 * 1024);
        rt.set_gc_threshold((self.limits.memory_bytes / 4).max(256 * 1024));
        let deadline: Rc<Cell<Option<Instant>>> = Rc::new(Cell::new(None));
        let d = deadline.clone();
        rt.set_interrupt_handler(Some(Box::new(move || d.get().is_some_and(|t| Instant::now() >= t))));
        let ctx = Context::full(&rt).map_err(|e| e.to_string())?;

        deadline.set(Some(Instant::now() + self.limits.activate_budget));
        let id = slot.manifest.id.clone();
        let granted = slot.granted.clone();
        let storage = self.storage.clone();
        let sink = self.sink.clone();
        let quota = self.limits.storage_quota_bytes;
        let source = slot.source.clone();

        let result = catch_unwind(AssertUnwindSafe(|| {
            ctx.with(|ctx| -> std::result::Result<(), String> {
                install_host(&ctx, &id, &granted, storage, sink, quota).map_err(|e| describe_error(&ctx, e))?;
                ctx.eval::<(), _>(PRELUDE).map_err(|e| describe_error(&ctx, e))?;
                ctx.eval::<(), _>(source.as_str()).map_err(|e| describe_error(&ctx, e))?;
                let call: Function = ctx.globals().get("__dz_call").map_err(|e| describe_error(&ctx, e))?;
                call.call::<_, String>(("activate", "", "", "")).map_err(|e| describe_error(&ctx, e))?;
                Ok(())
            })
        }));
        let timed_out = deadline.get().is_some_and(|t| Instant::now() >= t);
        deadline.set(None);
        match result {
            Ok(Ok(())) => Ok(Loaded { ctx, rt, deadline }),
            Ok(Err(_)) if timed_out => {
                Err(format!("activation timed out after {} ms", self.limits.activate_budget.as_millis()))
            }
            Ok(Err(e)) => Err(format!("activation failed: {e}")),
            Err(_) => Err("activation crashed (host panic)".into()),
        }
    }

    /// Run one dispatcher call against a plugin, loading it lazily, enforcing
    /// the time budget and maintaining failure accounting.
    fn call(&mut self, id: &str, kind: &str, a: &str, b: &str, c: &str) -> Result<serde_json::Value> {
        let max_failures = self.limits.max_failures;
        let budget = self.limits.call_budget;
        let slot_state = match self.slots.get(id) {
            None => return Err(CoreError::NotFound(format!("plugin {id} is not loaded"))),
            Some(s) => s.state,
        };
        if slot_state == RuntimeState::Faulted {
            return Err(CoreError::Plugin(format!("plugin {id} is disabled after repeated failures")));
        }

        let started = Instant::now();
        // Lazy activation.
        let needs_load = self.slots.get(id).is_some_and(|s| s.loaded.is_none());
        if needs_load {
            let built = self.build_runtime(&self.slots[id]);
            let slot = self.slots.get_mut(id).expect("slot exists");
            match built {
                Ok(l) => {
                    slot.loaded = Some(l);
                    slot.state = RuntimeState::Active;
                    slot.activations += 1;
                    self.sink.state_changed(id, RuntimeState::Active, None);
                }
                Err(e) => return Err(self.record_failure(id, e, started, max_failures)),
            }
        }

        let slot = self.slots.get_mut(id).expect("slot exists");
        let loaded = slot.loaded.as_ref().expect("loaded above");
        loaded.deadline.set(Some(Instant::now() + budget));
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            loaded.ctx.with(|ctx| -> std::result::Result<String, String> {
                let f: Function = ctx.globals().get("__dz_call").map_err(|e| describe_error(&ctx, e))?;
                f.call::<_, String>((kind, a, b, c)).map_err(|e| describe_error(&ctx, e))
            })
        }));
        // Drain microtasks (promise callbacks) under the same budget.
        if outcome.is_ok() {
            while loaded.rt.is_job_pending() {
                if loaded.rt.execute_pending_job().is_err() {
                    break;
                }
            }
        }
        let timed_out = loaded.deadline.get().is_some_and(|t| Instant::now() >= t);
        loaded.deadline.set(None);
        slot.last_used = Instant::now();

        match outcome {
            Err(_) => {
                slot.loaded = None;
                let msg = "plugin call crashed the host binding (panic contained)".to_string();
                // A host panic is never the plugin's "fault" to retry: fault immediately.
                slot.consecutive_failures = max_failures.saturating_sub(1);
                Err(self.record_failure(id, msg, started, max_failures))
            }
            Ok(Err(e)) => {
                let msg = if timed_out {
                    // State after an interrupt is unknown; discard the runtime.
                    slot.loaded = None;
                    format!("timed out after {} ms", budget.as_millis())
                } else {
                    if e.contains("out of memory") || e.contains("uncatchable") {
                        slot.loaded = None;
                    }
                    e
                };
                if slot.loaded.is_none() && slot.state == RuntimeState::Active {
                    slot.state = RuntimeState::Dormant;
                }
                Err(self.record_failure(id, msg, started, max_failures))
            }
            Ok(Ok(raw)) => {
                slot.calls += 1;
                slot.total_time_us += started.elapsed().as_micros() as u64;
                slot.consecutive_failures = 0;
                if raw.len() > MAX_RESULT_BYTES {
                    return Err(self.record_failure(id, "result exceeded 256 KiB".into(), started, max_failures));
                }
                serde_json::from_str(&raw).map_err(|e| CoreError::Plugin(format!("plugin returned invalid data: {e}")))
            }
        }
    }

    fn record_failure(&mut self, id: &str, msg: String, started: Instant, max_failures: u32) -> CoreError {
        let slot = self.slots.get_mut(id).expect("slot exists");
        slot.calls += 1;
        slot.failures += 1;
        slot.consecutive_failures += 1;
        slot.total_time_us += started.elapsed().as_micros() as u64;
        slot.last_error = Some(msg.clone());
        self.sink.log(id, log::Level::Error, &msg);
        if slot.consecutive_failures >= max_failures {
            slot.loaded = None;
            slot.state = RuntimeState::Faulted;
            self.sink.state_changed(id, RuntimeState::Faulted, Some(&msg));
        }
        CoreError::Plugin(msg)
    }

    fn command(&mut self, id: &str, name: &str, args: &str) -> Result<CommandOutcome> {
        let declared = self
            .slots
            .get(id)
            .ok_or_else(|| CoreError::NotFound(format!("plugin {id}")))?
            .manifest
            .contributes
            .commands
            .iter()
            .any(|c| c.name == name);
        if !declared {
            return Err(CoreError::NotFound(format!("plugin {id} does not declare command `{name}`")));
        }
        let args = crate::util::truncate_chars(args, 4000);
        let v = self.call(id, "command", name, &args, "")?;
        if v.is_null() {
            return Ok(CommandOutcome::default());
        }
        let mut out: CommandOutcome =
            serde_json::from_value(v).map_err(|e| CoreError::Plugin(format!("invalid command result: {e}")))?;
        out.replace_with = out.replace_with.map(|s| crate::util::truncate_chars(&s, 4000));
        out.ephemeral = out.ephemeral.map(|s| crate::util::truncate_chars(&s, 500));
        Ok(out)
    }

    fn render(&mut self, id: &str, panel: &str, action: Option<(&str, &str)>) -> Result<Vec<ViewNode>> {
        let declared = self
            .slots
            .get(id)
            .ok_or_else(|| CoreError::NotFound(format!("plugin {id}")))?
            .manifest
            .contributes
            .panels
            .iter()
            .any(|p| p.id == panel);
        if !declared {
            return Err(CoreError::NotFound(format!("plugin {id} does not declare panel `{panel}`")));
        }
        let v = match action {
            None => self.call(id, "render", panel, "", "")?,
            Some((act, values)) => {
                let values: serde_json::Value = serde_json::from_str(values).unwrap_or(serde_json::Value::Null);
                let values = if values.is_object() { values } else { serde_json::json!({}) };
                self.call(id, "action", panel, act, &values.to_string())?
            }
        };
        let out = sanitize(&v);
        if out.dropped > 0 {
            self.sink.log(
                id,
                log::Level::Warn,
                &format!("panel `{panel}`: dropped {} invalid view node(s)", out.dropped),
            );
        }
        Ok(out.nodes)
    }

    fn message(&mut self, json: &str) {
        let targets: Vec<String> = self
            .slots
            .iter()
            .filter(|(_, s)| {
                s.state != RuntimeState::Faulted
                    && s.granted.contains(&Permission::EventsMessages)
                    && s.manifest.contributes.events.iter().any(|e| e == "messageCreated")
            })
            .map(|(id, _)| id.clone())
            .collect();
        for id in targets {
            if let Err(e) = self.call(&id, "event", "messageCreated", json, "") {
                log::warn!("plugin {id} failed handling messageCreated: {e}");
            }
        }
    }
}

/// Install the native functions for one plugin into `globalThis.__dz_host`.
/// Functions are only created for permissions that were granted.
fn install_host<'js>(
    ctx: &Ctx<'js>,
    plugin_id: &str,
    granted: &BTreeSet<Permission>,
    storage: Arc<dyn PluginStorage>,
    sink: Arc<dyn HostSink>,
    quota: usize,
) -> rquickjs::Result<()> {
    let host = Object::new(ctx.clone())?;

    {
        let sink = sink.clone();
        let id = plugin_id.to_string();
        let limiter = RateLimit::new(50, Duration::from_secs(1));
        host.set(
            "log",
            Function::new(ctx.clone(), move |level: i32, msg: String| {
                if limiter.allow() {
                    let lvl = match level {
                        0 => log::Level::Error,
                        1 => log::Level::Warn,
                        _ => log::Level::Info,
                    };
                    sink.log(&id, lvl, &crate::util::truncate_chars(&msg, 2000));
                }
            })?,
        )?;
    }

    if granted.contains(&Permission::StorageLocal) {
        let (s, id) = (storage.clone(), plugin_id.to_string());
        host.set(
            "sget",
            Function::new(ctx.clone(), move |key: String| match s.get(&id, &key) {
                Ok(Some(v)) => envelope_ok(serde_json::Value::String(v)),
                Ok(None) => envelope_ok(serde_json::Value::Null),
                Err(e) => envelope_err(e),
            })?,
        )?;
        let (s, id) = (storage.clone(), plugin_id.to_string());
        host.set(
            "sset",
            Function::new(ctx.clone(), move |key: String, value: String| {
                if key.is_empty() || key.chars().count() > 128 {
                    return envelope_err("storage keys must be 1–128 characters");
                }
                if value.len() > MAX_STORAGE_VALUE {
                    return envelope_err(format!("storage value exceeds {MAX_STORAGE_VALUE} bytes"));
                }
                let used = match s.usage_bytes(&id) {
                    Ok(u) => u,
                    Err(e) => return envelope_err(e),
                };
                let existing = match s.get(&id, &key) {
                    Ok(v) => v.map(|v| v.len() + key.len()).unwrap_or(0),
                    Err(e) => return envelope_err(e),
                };
                let next = used.saturating_sub(existing) + key.len() + value.len();
                if next > quota {
                    return envelope_err(format!("storage quota exceeded ({next} of {quota} bytes)"));
                }
                match s.set(&id, &key, &value) {
                    Ok(()) => envelope_ok(serde_json::Value::Null),
                    Err(e) => envelope_err(e),
                }
            })?,
        )?;
        let (s, id) = (storage.clone(), plugin_id.to_string());
        host.set(
            "sdel",
            Function::new(ctx.clone(), move |key: String| match s.delete(&id, &key) {
                Ok(()) => envelope_ok(serde_json::Value::Null),
                Err(e) => envelope_err(e),
            })?,
        )?;
        let (s, id) = (storage, plugin_id.to_string());
        host.set(
            "skeys",
            Function::new(ctx.clone(), move || match s.keys(&id) {
                Ok(k) => envelope_ok(serde_json::json!(k)),
                Err(e) => envelope_err(e),
            })?,
        )?;
    }

    if granted.contains(&Permission::Notifications) {
        let (sink, id) = (sink.clone(), plugin_id.to_string());
        let limiter = RateLimit::new(5, Duration::from_secs(10));
        host.set(
            "notify",
            Function::new(ctx.clone(), move |title: String, body: String| {
                if !limiter.allow() {
                    return envelope_err("notification rate limit reached (5 per 10 s)");
                }
                sink.notify(&id, &crate::util::truncate_chars(&title, 80), &crate::util::truncate_chars(&body, 300));
                envelope_ok(serde_json::Value::Null)
            })?,
        )?;
    }

    if granted.contains(&Permission::UiPanels) || granted.contains(&Permission::UiSidebar) {
        let (sink, id) = (sink, plugin_id.to_string());
        let limiter = RateLimit::new(20, Duration::from_secs(1));
        host.set(
            "refresh",
            Function::new(ctx.clone(), move |panel: String| {
                if !limiter.allow() {
                    return envelope_err("refresh rate limit reached");
                }
                sink.refresh(&id, &crate::util::truncate_chars(&panel, 32));
                envelope_ok(serde_json::Value::Null)
            })?,
        )?;
    }

    ctx.globals().set("__dz_host", host)?;
    Ok(())
}
