// End-to-end smoke test against the running desktop app.
// Usage: start DiZero with the CDP port enabled, then `node tests/e2e/smoke.mjs`.
// Steps are independent enough that a failure names the broken flow.

import { attach, shot, check } from "./driver.mjs";

const { browser, page } = await attach();
const errors = [];
page.on("pageerror", (e) => errors.push(e.message));
page.on("console", (m) => m.type() === "error" && errors.push(m.text()));

const step = async (name, fn) => {
  console.log(`• ${name}`);
  await fn();
};

const goWorkspace = async () => {
  await page.click('button[aria-label="Workspace"]');
  await page.waitForSelector("#dz-composer");
};

const openChannel = async (name) => {
  await page.keyboard.press("Control+k");
  await page.fill('input[aria-label="Jump to a channel or page"]', name);
  await page.keyboard.press("Enter");
  await page.waitForFunction((n) => document.querySelector(".header h1")?.textContent === n, name);
};

async function installAndEnable(name) {
  await page.click('button[aria-label="Plugins"]');
  await page.click('[role="tab"]:has-text("Available")');
  const row = page.locator(".list > li", { hasText: name });
  const install = row.locator("button", { hasText: /^Install$/ });
  if (await install.count()) {
    await install.click();
    await page.waitForSelector('[role="dialog"]');
    await page.click('[role="dialog"] button:has-text("Enable plugin")');
    await page.waitForSelector('[role="dialog"]', { state: "detached" });
  } else {
    // Already installed from an earlier run: make sure it's enabled.
    await page.click('[role="tab"]:has-text("Installed")');
    const sw = page.locator(".list > li", { hasText: name }).locator('[role="switch"]').first();
    if ((await sw.getAttribute("aria-checked")) !== "true") {
      await sw.click();
      await page.click('[role="dialog"] button:has-text("Enable plugin")');
      await page.waitForSelector('[role="dialog"]', { state: "detached" });
    }
  }
  await page.waitForTimeout(300);
}

try {
  await step("workspace renders with demo data", async () => {
    await page.waitForSelector("article.row");
    check((await page.locator(".tile").count()) >= 3, "three demo communities in the rail");
    check((await page.locator("article.row").count()) > 0, "messages are listed");
    await shot(page, "01-workspace");
  });

  await step("send a message and get Nova's scripted reply", async () => {
    await openChannel("general");
    const marker = `e2e ${Date.now()}`;
    await page.fill("#dz-composer", `@Nova what can plugins do? (${marker})`);
    await page.keyboard.press("Enter");
    await page.waitForSelector(`article.row:has-text("${marker}")`);
    check(true, "own message appears");
    await page.waitForSelector('.typing:has-text("Nova")', { timeout: 5000 });
    check(true, "typing indicator shows");
    await page.waitForSelector('article.row:has-text("QuickJS")', { timeout: 8000 });
    check(true, "Nova replied");
    await shot(page, "02-nova-reply");
  });

  await step("quick switcher navigates", async () => {
    await openChannel("perf-tuning");
    check(true, "switched to #perf-tuning");
  });

  await step("search finds messages", async () => {
    await page.fill("#dz-search", "mutex");
    await page.waitForSelector(".results li", { timeout: 5000 });
    check((await page.locator(".results li").count()) >= 1, "search returns results");
    await shot(page, "03-search");
    await page.click('button[aria-label="Clear search"]');
  });

  await step("install Slash Tools and run /shrug", async () => {
    await installAndEnable("Slash Tools");
    await shot(page, "04-plugins");
    await goWorkspace();
    await openChannel("general");
    await page.fill("#dz-composer", "/shr");
    await page.waitForSelector('[role="listbox"][aria-label="Commands"]');
    check(true, "slash suggestions appear");
    await page.fill("#dz-composer", "/shrug works");
    await page.keyboard.press("Enter");
    await page.waitForFunction(() => document.querySelector("#dz-composer")?.value.includes("¯\\_(ツ)_/¯"));
    check(true, "/shrug rewrote the composer");
    await page.keyboard.press("Enter");
    await page.waitForFunction(() =>
      [...document.querySelectorAll("article.row")].some((r) => r.textContent?.includes("works ¯\\_(ツ)_/¯")),
    );
    check(true, "rewritten message sent");
  });

  await step("Fault Lab is contained", async () => {
    await installAndEnable("Fault Lab");
    await goWorkspace();
    const t0 = Date.now();
    await page.fill("#dz-composer", "/hang");
    await page.keyboard.press("Enter");
    await page.waitForSelector('.note.error:has-text("timed out")', { timeout: 5000 });
    check(Date.now() - t0 < 3000, `infinite loop stopped in ${Date.now() - t0} ms`);
    await page.fill("#dz-composer", "/escape");
    await page.keyboard.press("Enter");
    await page.waitForSelector('.note:has-text("Nothing reachable")', { timeout: 5000 });
    check(true, "sandbox exposes no host globals");
    await page.fill("#dz-composer", "still responsive");
    check((await page.inputValue("#dz-composer")) === "still responsive", "UI still responsive");
    await page.fill("#dz-composer", "");
    await shot(page, "05-fault-lab");
  });

  await step("panels: Message Stats and Quick Notes", async () => {
    await installAndEnable("Message Stats");
    await installAndEnable("Quick Notes");
    await goWorkspace();
    await page.click('[role="tab"][aria-label="Notes"]');
    await page.fill('[aria-label="Write a note…"]', "e2e note");
    await page.click('button:has-text("Add note")');
    await page.waitForSelector('.list li:has-text("e2e note")');
    check(true, "note added through a plugin panel");
    await page.click('.list li:has-text("e2e note") button:has-text("Pin")');
    await page.waitForSelector('.plugin-section:has-text("e2e note")');
    check(true, "pinned note shows in the sidebar section");
    await page.fill("#dz-composer", "counting this one");
    await page.keyboard.press("Enter");
    await page.click('[role="tab"][aria-label="Activity"]');
    await page.waitForSelector('.stat:has-text("Messages")', { timeout: 5000 });
    check(true, "Message Stats counted live messages");
    await shot(page, "06-panels");
  });

  await step("performance center measures the process tree", async () => {
    await page.click('button[aria-label="Performance Center"]');
    await page.waitForSelector(".strip .strip-name:has-text('DiZero core')", { timeout: 10_000 });
    const strips = await page.locator(".strips .strip").count();
    check(strips >= 2, `${strips} processes measured (core + WebView2)`);
    await page.waitForTimeout(2500);
    await shot(page, "07-performance");
  });

  await step("settings persist and apply", async () => {
    await page.click('button[aria-label="Settings (Ctrl+,)"]');
    await page.click('[role="radiogroup"][aria-label="Density"] [role="radio"]:has-text("Compact")');
    await page.waitForFunction(() => document.documentElement.dataset.density === "compact");
    check(true, "density applied");
    await shot(page, "08-settings");
    await page.click('[role="radiogroup"][aria-label="Density"] [role="radio"]:has-text("Cozy")');
  });

  await step("developer center shows logs", async () => {
    await page.click('button[aria-label="Developer Center"]');
    await page.waitForSelector(".log .line", { timeout: 6000 });
    check(true, "log entries visible");
    await shot(page, "09-developer");
  });

  await step("uninstall a plugin through the UI", async () => {
    await page.click('button[aria-label="Plugins"]');
    await page.click('[role="tab"]:has-text("Installed")');
    const row = page.locator(".list > li", { hasText: "Fault Lab" });
    await row.locator("button.expand").click();
    await row.locator('button:has-text("Uninstall")').click();
    await page.click('[role="dialog"] button:has-text("Uninstall")');
    await page.waitForSelector('.list > li:has-text("Fault Lab")', { state: "detached", timeout: 8000 });
    check(true, "Fault Lab removed from the installed list");
    await page.click('[role="tab"]:has-text("Available")');
    const reinstall = page.locator(".list > li", { hasText: "Fault Lab" }).locator("button", { hasText: /^Install$/ });
    check((await reinstall.count()) === 1, "Fault Lab is installable again from the catalog");
    await goWorkspace();
    await page.fill("#dz-composer", "/ha");
    await page.waitForTimeout(300);
    check((await page.locator('[role="listbox"] .cmd:has-text("/hang")').count()) === 0, "its /hang command is gone from the composer");
    await page.fill("#dz-composer", "");
  });

  await goWorkspace();
  check(errors.length === 0, `no page errors (${errors.join(" | ") || "none"})`);
  console.log("SMOKE PASSED");
} catch (e) {
  await shot(page, "zz-failure").catch(() => {});
  console.error(String(e));
  if (errors.length) console.error("page errors:", errors);
  process.exitCode = 1;
} finally {
  await browser.close().catch(() => {});
}
