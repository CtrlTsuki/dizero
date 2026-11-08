// A single shared tooltip element, positioned next to the hovered/focused
// control. Shown after a short delay; hidden on leave, blur, click or Escape.

type Side = "right" | "top" | "bottom" | "left";

let el: HTMLDivElement | null = null;
let timer: ReturnType<typeof setTimeout> | undefined;

function ensure(): HTMLDivElement {
  if (el) return el;
  el = document.createElement("div");
  el.className = "dz-tooltip";
  el.setAttribute("role", "tooltip");
  el.id = "dz-tooltip";
  document.body.appendChild(el);
  return el;
}

function hide() {
  clearTimeout(timer);
  if (el) el.dataset.open = "false";
}

function show(node: HTMLElement, text: string, side: Side) {
  const tip = ensure();
  tip.textContent = text;
  tip.dataset.open = "true";
  const r = node.getBoundingClientRect();
  const t = tip.getBoundingClientRect();
  const gap = 8;
  let x = 0;
  let y = 0;
  if (side === "right") {
    x = r.right + gap;
    y = r.top + r.height / 2 - t.height / 2;
  } else if (side === "left") {
    x = r.left - t.width - gap;
    y = r.top + r.height / 2 - t.height / 2;
  } else if (side === "top") {
    x = r.left + r.width / 2 - t.width / 2;
    y = r.top - t.height - gap;
  } else {
    x = r.left + r.width / 2 - t.width / 2;
    y = r.bottom + gap;
  }
  x = Math.max(6, Math.min(x, window.innerWidth - t.width - 6));
  y = Math.max(6, Math.min(y, window.innerHeight - t.height - 6));
  tip.style.transform = `translate(${Math.round(x)}px, ${Math.round(y)}px)`;
}

export function tooltip(node: HTMLElement, params: { text: string; side?: Side } | string) {
  let opts = typeof params === "string" ? { text: params } : params;
  const enter = () => {
    clearTimeout(timer);
    timer = setTimeout(() => opts.text && show(node, opts.text, opts.side ?? "top"), 420);
  };
  const key = (e: KeyboardEvent) => e.key === "Escape" && hide();
  node.addEventListener("mouseenter", enter);
  node.addEventListener("focus", enter);
  node.addEventListener("mouseleave", hide);
  node.addEventListener("blur", hide);
  node.addEventListener("mousedown", hide);
  node.addEventListener("keydown", key);
  return {
    update(p: { text: string; side?: Side } | string) {
      opts = typeof p === "string" ? { text: p } : p;
    },
    destroy() {
      hide();
      node.removeEventListener("mouseenter", enter);
      node.removeEventListener("focus", enter);
      node.removeEventListener("mouseleave", hide);
      node.removeEventListener("blur", hide);
      node.removeEventListener("mousedown", hide);
      node.removeEventListener("keydown", key);
    },
  };
}
