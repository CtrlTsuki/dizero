import { api, errorMessage, type AppInfo, type SettingsView } from "../api";
import type { PerfProfile } from "../bindings/PerfProfile";
import type { Settings } from "../bindings/Settings";
import { toasts } from "./toasts.svelte";

export type Route = "workspace" | "plugins" | "performance" | "developer" | "settings";

class AppStore {
  route = $state<Route>("workspace");
  settings = $state<Settings | null>(null);
  profile = $state<PerfProfile | null>(null);
  info = $state<AppInfo | null>(null);
  /** Which settings section to open (deep links from elsewhere in the UI). */
  settingsSection = $state<string>("appearance");
  quickSwitcherOpen = $state(false);
  systemReducedMotion = $state(false);

  reducedMotion = $derived.by(() => {
    const s = this.settings;
    if (!s) return this.systemReducedMotion;
    if (this.profile && !this.profile.animations) return true;
    if (s.appearance.motion === "reduce") return true;
    if (s.appearance.motion === "full") return false;
    return this.systemReducedMotion;
  });

  async init() {
    const mq = window.matchMedia("(prefers-reduced-motion: reduce)");
    this.systemReducedMotion = mq.matches;
    mq.addEventListener("change", (e) => (this.systemReducedMotion = e.matches));
    const [view, info] = await Promise.all([api.settingsGet(), api.appInfo()]);
    this.applyView(view);
    this.info = info;
    if (info.recoveredDatabase) {
      toasts.push(
        {
          kind: "error",
          title: "Local data was damaged and has been reset",
          body: "The unreadable database was kept as a backup next to the new one.",
        },
        12_000,
      );
    }
  }

  applyView(v: SettingsView) {
    this.settings = v.settings;
    this.profile = v.profile;
  }

  /** Apply a change to a copy of the settings and persist it. */
  async update(mutate: (s: Settings) => void) {
    if (!this.settings) return;
    const next = $state.snapshot(this.settings) as Settings;
    mutate(next);
    const previous = this.settings;
    this.settings = next; // optimistic
    try {
      this.applyView(await api.settingsUpdate(next));
    } catch (e) {
      this.settings = previous;
      toasts.error("Couldn't save settings", errorMessage(e));
    }
  }

  go(route: Route, section?: string) {
    if (section) this.settingsSection = section;
    this.route = route;
  }
}

export const app = new AppStore();
