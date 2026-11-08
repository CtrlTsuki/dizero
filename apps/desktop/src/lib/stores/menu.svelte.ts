export interface MenuItem {
  label: string;
  run: () => void;
  danger?: boolean;
  disabled?: boolean;
}

class ContextMenu {
  open = $state(false);
  x = $state(0);
  y = $state(0);
  items = $state<MenuItem[]>([]);
  #returnFocus: HTMLElement | null = null;

  show(x: number, y: number, items: MenuItem[]) {
    this.#returnFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    this.x = x;
    this.y = y;
    this.items = items;
    this.open = true;
  }

  showAt(anchor: HTMLElement, items: MenuItem[]) {
    const r = anchor.getBoundingClientRect();
    this.show(r.right, r.bottom + 4, items);
  }

  close() {
    if (!this.open) return;
    this.open = false;
    this.#returnFocus?.focus();
    this.#returnFocus = null;
  }
}

export const menu = new ContextMenu();
