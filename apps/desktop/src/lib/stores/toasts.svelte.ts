export type ToastKind = "info" | "success" | "error" | "plugin" | "message";

export interface Toast {
  id: number;
  kind: ToastKind;
  title: string;
  body?: string;
  action?: { label: string; run: () => void };
}

const MAX_VISIBLE = 4;
let nextId = 1;

class Toasts {
  items = $state<Toast[]>([]);
  #timers = new Map<number, ReturnType<typeof setTimeout>>();

  push(t: Omit<Toast, "id">, ttlMs?: number): number {
    const id = nextId++;
    this.items = [...this.items, { ...t, id }].slice(-MAX_VISIBLE);
    const ttl = ttlMs ?? (t.kind === "error" ? 8000 : 5000);
    this.#timers.set(
      id,
      setTimeout(() => this.dismiss(id), ttl),
    );
    return id;
  }

  info(title: string, body?: string) {
    return this.push({ kind: "info", title, ...(body ? { body } : {}) });
  }
  success(title: string, body?: string) {
    return this.push({ kind: "success", title, ...(body ? { body } : {}) });
  }
  error(title: string, body?: string) {
    return this.push({ kind: "error", title, ...(body ? { body } : {}) });
  }

  dismiss(id: number) {
    const timer = this.#timers.get(id);
    if (timer) clearTimeout(timer);
    this.#timers.delete(id);
    this.items = this.items.filter((t) => t.id !== id);
  }
}

export const toasts = new Toasts();
