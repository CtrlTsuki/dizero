// CPU cost of scrolling a long conversation, summed over DiZero's whole
// process tree. Used to decide whether software rendering (--disable-gpu)
// is an acceptable default. Run against an app started with the CDP port.
//   node tests/e2e/scroll-bench.mjs <label>

import { execSync } from "node:child_process";
import { attach } from "./driver.mjs";

const label = process.argv[2] ?? "run";

/** Total CPU seconds used so far by dizero.exe and all its descendants. */
function treeCpuSeconds() {
  const script = new URL("../../scripts/tree-cpu.ps1", import.meta.url);
  const path = decodeURIComponent(script.pathname).replace(/^\/([A-Za-z]:)/, "$1");
  return Number(execSync(`powershell -NoProfile -ExecutionPolicy Bypass -File "${path}"`).toString().trim());
}

const { browser, page } = await attach();
try {
  // Ensure a long history in #general (idempotent-ish: only seeds when short).
  await page.click('button[aria-label="Workspace"]');
  await page.keyboard.press("Control+k");
  await page.fill('input[aria-label="Jump to a channel or page"]', "general");
  await page.keyboard.press("Enter");
  await page.waitForSelector("article.row");
  const count = await page.evaluate(async () => {
    const invoke = window.__TAURI_INTERNALS__.invoke;
    const snap = await invoke("workspace_snapshot");
    const general = snap.conversations.find((c) => c.name === "general");
    const page1 = await invoke("messages_list", { conversationId: general.id, before: null, limit: 500 });
    if (page1.messages.length < 250) {
      for (let i = 0; i < 260; i++) {
        await invoke("message_send", {
          conversationId: general.id,
          content: `Bench line ${i}: **bold**, \`code\` and a longer sentence so rows wrap differently across the history.`,
        });
      }
    }
    return page1.messages.length;
  });
  console.log(`history before seeding: ${count}`);
  await page.waitForTimeout(2000);

  const box = await page.locator(".list").boundingBox();
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);

  const idle0 = treeCpuSeconds();
  await page.waitForTimeout(5000);
  const idle = treeCpuSeconds() - idle0;

  const start = treeCpuSeconds();
  const t0 = Date.now();
  while (Date.now() - t0 < 10_000) {
    for (let i = 0; i < 12; i++) await page.mouse.wheel(0, -260);
    for (let i = 0; i < 12; i++) await page.mouse.wheel(0, 260);
  }
  const elapsed = (Date.now() - t0) / 1000;
  const scroll = treeCpuSeconds() - start;

  const cores = Number(execSync('powershell -NoProfile -Command "(Get-CimInstance Win32_Processor).NumberOfLogicalProcessors"').toString().trim());
  console.log(
    JSON.stringify({
      label,
      idleCpuSecondsPer5s: Number(idle.toFixed(3)),
      scrollCpuSeconds: Number(scroll.toFixed(3)),
      scrollSeconds: elapsed,
      scrollCpuPercentOfOneCore: Number(((scroll / elapsed) * 100).toFixed(1)),
      scrollCpuPercentOfMachine: Number(((scroll / elapsed / cores) * 100).toFixed(2)),
    }),
  );
} finally {
  await browser.close().catch(() => {});
}
