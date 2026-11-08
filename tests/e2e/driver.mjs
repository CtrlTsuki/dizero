// Attach to a running DiZero (started with
//   WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222)
// over the Chrome DevTools Protocol. This drives the *real* app, not a mock.

import { chromium } from "playwright-core";
import { mkdirSync } from "node:fs";
import { join } from "node:path";

export const PORT = Number(process.env.DZ_CDP_PORT ?? 9222);
export const OUT = process.env.DZ_E2E_OUT ?? join(process.cwd(), "tests", "e2e", "out");

export async function waitForCdp(timeoutMs = 180_000) {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    try {
      const r = await fetch(`http://127.0.0.1:${PORT}/json/version`);
      if (r.ok) return;
    } catch {
      /* not up yet */
    }
    await new Promise((r) => setTimeout(r, 1000));
  }
  throw new Error(`no CDP endpoint on port ${PORT} after ${timeoutMs} ms`);
}

export async function attach() {
  await waitForCdp();
  const browser = await chromium.connectOverCDP(`http://127.0.0.1:${PORT}`);
  const deadline = Date.now() + 60_000;
  while (Date.now() < deadline) {
    for (const ctx of browser.contexts()) {
      for (const page of ctx.pages()) {
        if ((await page.title().catch(() => "")) === "DiZero") {
          await page.waitForSelector("nav.rail", { timeout: 60_000 });
          return { browser, page };
        }
      }
    }
    await new Promise((r) => setTimeout(r, 500));
  }
  throw new Error("DiZero page not found");
}

export async function shot(page, name) {
  mkdirSync(OUT, { recursive: true });
  const path = join(OUT, `${name}.png`);
  await page.screenshot({ path });
  return path;
}

export function check(cond, msg) {
  if (!cond) throw new Error(`CHECK FAILED: ${msg}`);
  console.log(`  ✓ ${msg}`);
}
