/**
 * Full desktop walkthrough for docs videos — slower form fill, every major view.
 * Requires an e2e-featured release binary (make e2e-tauri) + cookie-session demo on :9080.
 *
 * Skips: real SMTP send/test, admin hosts prompts, destructive purge/archive confirms.
 */
describe("phishkit walkthrough", () => {
  const stamp = Date.now();
  const assessmentName = `Walkthrough ${stamp}`;
  const assessmentDomain = "demo-cookie.local.phishkit";
  const targetUrl = "http://127.0.0.1:9080";
  const targetName = "Cookie-session demo";
  const recipientList = "walkthrough-targets";
  const recipientEmails = "alice@example.test\nbob@example.test";

  /** Pause so viewers can read the screen (video reporter also slows frames). */
  const dwell = (ms = 1200) => browser.pause(ms);

  async function dumpDebug(label: string) {
    try {
      const title = await browser.getTitle();
      const url = await browser.getUrl();
      const html = await browser.getPageSource();
      // eslint-disable-next-line no-console
      console.log(
        `[e2e-debug:${label}] title=${title} url=${url} htmlLen=${html.length}`
      );
      // eslint-disable-next-line no-console
      console.log(html.slice(0, 2000));
    } catch (e) {
      // eslint-disable-next-line no-console
      console.log(`[e2e-debug:${label}] dump failed: ${e}`);
    }
  }

  async function clickTestId(id: string) {
    const el = await $(`[data-testid="${id}"]`);
    await el.waitForExist({ timeout: 25000 });
    await dwell(400);
    try {
      await el.waitForClickable({ timeout: 5000 });
      await el.click();
    } catch {
      await browser.execute((tid) => {
        const node = document.querySelector(
          `[data-testid="${tid}"]`
        ) as HTMLElement | null;
        if (!node) throw new Error(`missing ${tid}`);
        node.click();
      }, id);
    }
    await dwell(500);
  }

  async function typeSlow(testid: string, text: string, perCharMs = 70) {
    const el = await $(`[data-testid="${testid}"]`);
    await el.waitForExist({ timeout: 20000 });
    await el.click();
    await dwell(250);
    // Clear via select-all + delete (more reliable than clearValue on WKWebView).
    await browser.keys(["Meta", "a"]);
    await browser.keys("Backspace");
    await dwell(200);
    for (const ch of text) {
      if (ch === "\n") {
        await browser.keys("Enter");
      } else {
        await browser.keys(ch);
      }
      await browser.pause(perCharMs);
    }
    await dwell(400);
  }

  async function setValueSlow(testid: string, text: string) {
    // Fallback when keys() is flaky: set value then pause so UI settles.
    const el = await $(`[data-testid="${testid}"]`);
    await el.waitForExist({ timeout: 20000 });
    await el.click();
    await dwell(200);
    await el.setValue(text);
    await dwell(600);
  }

  async function ensureHome() {
    const crumb = await $('[data-testid="breadcrumb-assessments"]');
    if (await crumb.isExisting()) {
      await clickTestId("breadcrumb-assessments");
      await dwell(800);
    }
    const switcher = await $('[data-testid="ctx-switch-assessment"]');
    if (await switcher.isExisting()) {
      await clickTestId("ctx-switch-assessment");
      await dwell(800);
    }
  }

  async function visitNav(id: string, settleMs = 1400) {
    await clickTestId(`nav-${id}`);
    await dwell(settleMs);
  }

  it("full product tour: home → assessment → mail → target → recon → sessions", async () => {
    try {
      const handles = await browser.getWindowHandles();
      if (handles.length) {
        await browser.switchToWindow(handles[handles.length - 1]);
      }
    } catch {
      /* ignore */
    }

    const url = await browser.getUrl();
    if (!url || url === "about:blank" || url === "data:,") {
      for (const candidate of [
        "tauri://localhost/",
        "http://tauri.localhost/",
        "https://tauri.localhost/",
      ]) {
        try {
          await browser.url(candidate);
          await dwell(1500);
          const next = await browser.getUrl();
          if (next && next !== "about:blank") break;
        } catch {
          /* try next */
        }
      }
    }

    try {
      await browser.waitUntil(
        async () =>
          browser.execute(() => {
            const root = document.getElementById("root");
            return !!(root && root.childElementCount > 0);
          }),
        {
          timeout: 60000,
          interval: 500,
          timeoutMsg: "React #root never mounted children",
        }
      );
    } catch (e) {
      await dumpDebug("root-not-mounted");
      throw e;
    }

    await $("[data-testid='main-content']").waitForExist({ timeout: 15000 });
    await dwell(1500);

    // ── Home: Assessments ──────────────────────────────────────────────
    await ensureHome();
    await visitNav("assessments", 1600);

    try {
      if (await $('[data-testid="new-assessment-empty"]').isExisting()) {
        await clickTestId("new-assessment-empty");
      } else {
        await clickTestId("new-assessment");
      }
    } catch (e) {
      await dumpDebug("new-assessment");
      throw e;
    }
    await dwell(900);

    try {
      await typeSlow("assessment-name", assessmentName, 55);
    } catch {
      await setValueSlow("assessment-name", assessmentName);
    }
    try {
      await typeSlow("assessment-domain", assessmentDomain, 45);
    } catch {
      await setValueSlow("assessment-domain", assessmentDomain);
    }
    try {
      await typeSlow("assessment-authorized-by", "Security lead", 60);
    } catch {
      await setValueSlow("assessment-authorized-by", "Security lead");
    }
    try {
      await typeSlow("assessment-auth-ref", `SOW-${stamp}`, 50);
    } catch {
      await setValueSlow("assessment-auth-ref", `SOW-${stamp}`);
    }
    try {
      await typeSlow(
        "assessment-notes",
        "Authorized walkthrough of every console surface.",
        35
      );
    } catch {
      await setValueSlow(
        "assessment-notes",
        "Authorized walkthrough of every console surface."
      );
    }
    await dwell(800);
    await clickTestId("assessment-create");
    await dwell(2000);

    // ── Assessment: Overview ───────────────────────────────────────────
    await visitNav("overview", 1800);
    await $('[data-testid="overview-view"]').waitForExist({ timeout: 15000 });
    await dwell(2000);
    // Scroll lifecycle section into view without confirming destructive actions.
    await browser.execute(() => {
      document
        .querySelector('[data-testid="overview-export"]')
        ?.scrollIntoView({ block: "center", behavior: "instant" });
    });
    await dwell(1800);
    await browser.execute(() => {
      document
        .querySelector('[data-testid="overview-purge"]')
        ?.scrollIntoView({ block: "center", behavior: "instant" });
    });
    await dwell(1600);
    await browser.execute(() => {
      document
        .querySelector('[data-testid="overview-archive"]')
        ?.scrollIntoView({ block: "center", behavior: "instant" });
    });
    await dwell(1600);
    await browser.execute(() => window.scrollTo(0, 0));
    await dwell(800);

    // ── Templates ──────────────────────────────────────────────────────
    await visitNav("templates", 1600);
    await $('[data-testid="templates-view"]').waitForExist({ timeout: 15000 });
    await dwell(1000);
    await clickTestId("template-starter");
    await dwell(1200);
    try {
      await typeSlow("template-name", `Lure link ${stamp}`, 45);
    } catch {
      await setValueSlow("template-name", `Lure link ${stamp}`);
    }
    await dwell(600);
    await clickTestId("template-tab-preview");
    await dwell(1800);
    await clickTestId("template-tab-source");
    await dwell(1000);
    await clickTestId("template-save");
    await dwell(1600);

    // ── Recipients ─────────────────────────────────────────────────────
    await visitNav("recipients", 1600);
    await $('[data-testid="recipients-view"]').waitForExist({ timeout: 15000 });
    try {
      await typeSlow("recipient-list-name", recipientList, 50);
    } catch {
      await setValueSlow("recipient-list-name", recipientList);
    }
    try {
      await typeSlow("recipient-paste", recipientEmails, 40);
    } catch {
      await setValueSlow("recipient-paste", recipientEmails);
    }
    await dwell(1000);
    await clickTestId("recipient-import");
    await dwell(1800);

    // ── Delivery (form only — no SMTP send) ────────────────────────────
    await visitNav("delivery", 1600);
    await $('[data-testid="delivery-view"]').waitForExist({ timeout: 15000 });
    await dwell(1000);
    await clickTestId("delivery-preset-gmail");
    await dwell(900);
    await clickTestId("delivery-preset-smtp");
    await dwell(900);
    await clickTestId("delivery-preset-ses_smtp");
    await dwell(900);
    await clickTestId("delivery-preset-resend");
    await dwell(900);
    await clickTestId("delivery-preset-gmail");
    await dwell(800);
    try {
      await typeSlow("delivery-label", "Walkthrough demo sender", 45);
    } catch {
      await setValueSlow("delivery-label", "Walkthrough demo sender");
    }
    await dwell(1500);

    // ── Campaigns: Guided + Composer + Express tabs ────────────────────
    await visitNav("campaigns", 1600);
    await $('[data-testid="campaigns-view"]').waitForExist({ timeout: 15000 });
    await dwell(1000);

    await clickTestId("campaign-flow-guided");
    await dwell(1200);
    await clickTestId("guided-preset-generic");
    await dwell(1400);
    // Walk remaining scenario cards so viewers see options.
    await clickTestId("guided-preset-m365-login");
    await dwell(900);
    await clickTestId("guided-preset-okta-sso");
    await dwell(900);
    await clickTestId("guided-preset-awareness-payroll");
    await dwell(900);
    await clickTestId("guided-preset-generic");
    await dwell(1000);
    if (await $('[data-testid="guided-next"]').isExisting()) {
      const next = await $('[data-testid="guided-next"]');
      if (await next.isEnabled()) {
        await clickTestId("guided-next");
        await dwell(1600);
        // Target & lure step — pause for reading; may not advance without lure.
        await dwell(2000);
        if (
          (await $('[data-testid="guided-next"]').isExisting()) &&
          (await $('[data-testid="guided-next"]').isEnabled())
        ) {
          await clickTestId("guided-next");
          await dwell(1600);
        }
      }
    }

    await clickTestId("campaign-flow-composer");
    await dwell(2000);
    const aup = await $('[data-testid="aup-accept"]');
    if (await aup.isExisting()) {
      await clickTestId("aup-accept");
      await dwell(1200);
    }
    await clickTestId("campaign-flow-express");
    await dwell(2000);
    await clickTestId("campaign-flow-guided");
    await dwell(1200);

    // ── Results ────────────────────────────────────────────────────────
    await visitNav("results", 1600);
    await $('[data-testid="results-view"]').waitForExist({ timeout: 15000 });
    await dwell(1800);
    if (await $('[data-testid="results-import-toggle"]').isExisting()) {
      await clickTestId("results-import-toggle");
      await dwell(1200);
      const sampleEvents = `[{"email":"alice@example.test","event":"delivered"},{"email":"alice@example.test","event":"opened"},{"email":"bob@example.test","event":"clicked"}]`;
      try {
        await typeSlow("results-events", sampleEvents, 12);
      } catch {
        await setValueSlow("results-events", sampleEvents);
      }
      await dwell(1500);
      // Import only if a campaign is selected (button may stay disabled).
      const importBtn = await $('[data-testid="results-import"]');
      if ((await importBtn.isExisting()) && (await importBtn.isEnabled())) {
        await clickTestId("results-import");
        await dwell(1500);
      }
    }

    // ── Targets → create cookie-session demo ───────────────────────────────────
    await visitNav("targets", 1600);
    await clickTestId("new-target");
    await dwell(900);
    try {
      await typeSlow("target-url", targetUrl, 45);
    } catch {
      await setValueSlow("target-url", targetUrl);
    }
    try {
      await typeSlow("target-name", targetName, 55);
    } catch {
      await setValueSlow("target-name", targetName);
    }
    await dwell(800);
    await clickTestId("target-create");
    // Detect can take several seconds.
    await dwell(10000);

    // ── Target overview ────────────────────────────────────────────────
    if (await $('[data-testid="target-overview-recon"]').isExisting()) {
      await dwell(1800);
      await clickTestId("target-overview-recon");
    } else {
      await visitNav("recon", 1600);
    }
    await dwell(2000);

    // ── Recon & Proxy ──────────────────────────────────────────────────
    if (await $('[data-testid="dest-step-proxy"]').isExisting()) {
      await clickTestId("dest-step-proxy");
      await dwell(1600);
    }
    // Show runtime controls without triggering admin prompts / proxy start.
    await browser.execute(() => {
      document
        .querySelector('[data-testid="proxy-start"]')
        ?.scrollIntoView({ block: "center", behavior: "instant" });
    });
    await dwell(2000);
    if (await $('[data-testid="advanced-toggle"]').isExisting()) {
      await clickTestId("advanced-toggle");
      await dwell(2200);
      await clickTestId("advanced-toggle");
      await dwell(800);
    }
    if (await $('[data-testid="dest-go-captures"]').isExisting()) {
      await clickTestId("dest-go-captures");
      await dwell(1800);
      if (await $('[data-testid="captures-sync"]').isExisting()) {
        await clickTestId("captures-sync");
        await dwell(1600);
      }
      if (await $('[data-testid="dest-step-proxy"]').isExisting()) {
        await clickTestId("dest-step-proxy");
        await dwell(1200);
      }
    }

    // ── Sessions ───────────────────────────────────────────────────────
    await visitNav("sessions", 1600);
    await $('[data-testid="sessions-view"]').waitForExist({ timeout: 20000 });
    await dwell(1200);
    if (await $('[data-testid="sessions-sync"]').isExisting()) {
      await clickTestId("sessions-sync");
      await dwell(1600);
    }
    for (const f of ["creds", "tokens", "attributed", "all"]) {
      if (await $(`[data-testid="sessions-filter-${f}"]`).isExisting()) {
        await clickTestId(`sessions-filter-${f}`);
        await dwell(900);
      }
    }
    if (await $('[data-testid="sessions-show-empty"]').isExisting()) {
      await clickTestId("sessions-show-empty");
      await dwell(1200);
      await clickTestId("sessions-show-empty");
      await dwell(800);
    }
    if (await $('[data-testid="sessions-search"]').isExisting()) {
      try {
        await typeSlow("sessions-search", "alice", 80);
      } catch {
        await setValueSlow("sessions-search", "alice");
      }
      await dwell(1200);
      try {
        await typeSlow("sessions-search", "", 20);
      } catch {
        await setValueSlow("sessions-search", "");
      }
    }
    await dwell(1500);

    // ── Context / chrome ───────────────────────────────────────────────
    if (await $('[data-testid="ctx-target"]').isExisting()) {
      await clickTestId("ctx-target");
      await dwell(1400);
    }
    if (await $('[data-testid="ctx-back-targets"]').isExisting()) {
      await clickTestId("ctx-back-targets");
      await dwell(1600);
    }
    if (await $('[data-testid="ctx-assessment"]').isExisting()) {
      await clickTestId("ctx-assessment");
      await dwell(1600);
    }

    // Revisit overview once more after work, then Delivery from assessment nav.
    await visitNav("overview", 1600);
    await dwell(1500);
    await visitNav("delivery", 1400);
    await dwell(1400);

    // Activity log
    await clickTestId("activity-log");
    await dwell(2200);
    if (await $('[data-testid="activity-close"]').isExisting()) {
      await clickTestId("activity-close");
    }
    await dwell(1000);

    // Home Delivery (global sender library) via switcher
    if (await $('[data-testid="ctx-switch-assessment"]').isExisting()) {
      await clickTestId("ctx-switch-assessment");
      await dwell(1200);
      await visitNav("delivery", 1600);
      await dwell(1600);
      await visitNav("assessments", 1600);
      await dwell(1600);
    }

    await dwell(2000);
  });
});
