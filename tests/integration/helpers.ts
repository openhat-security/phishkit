const VIDEO = process.env.VIDEO === "1";

export const stamp = Date.now();
export const assessmentName = `Integration ${stamp}`;
export const assessmentDomain = "demo-cookie.local.phishkit";
export const targetUrl = "http://127.0.0.1:9080";
export const targetName = "Cookie-session demo";
export const recipientList = "integration-targets";
export const recipientEmails = "alice@example.test\nbob@example.test";

export async function dwell(ms = 400) {
  if (VIDEO) {
    await browser.pause(ms);
  }
}

export async function attachWindow() {
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
        break;
      } catch {
        /* try next */
      }
    }
  }

  await browser.waitUntil(
    async () => {
      const html = await browser.getPageSource();
      return html.includes("id=\"root\"") && html.length > 400;
    },
    { timeout: 30000, interval: 500, timeoutMsg: "React #root never mounted" }
  );
}

export async function acceptConfirms() {
  await browser.execute(() => {
    window.confirm = () => true;
  });
}

export async function clickTestId(id: string) {
  const el = await $(`[data-testid="${id}"]`);
  await el.waitForExist({ timeout: 25000 });
  await dwell(200);
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
  await dwell(250);
}

export async function setValue(testid: string, text: string) {
  const el = await $(`[data-testid="${testid}"]`);
  await el.waitForExist({ timeout: 20000 });
  await el.click();
  if (VIDEO) {
    await browser.keys(["Meta", "a"]);
    await browser.keys("Backspace");
    for (const ch of text) {
      if (ch === "\n") {
        await browser.keys("Enter");
      } else {
        await browser.keys(ch);
      }
      await browser.pause(40);
    }
  } else {
    await el.setValue(text);
  }
  await dwell(200);
}

export async function expectTestId(id: string, timeout = 15000) {
  const el = await $(`[data-testid="${id}"]`);
  await el.waitForExist({ timeout });
  return el;
}

export async function visitNav(id: string) {
  await clickTestId(`nav-${id}`);
  await dwell(400);
}

export async function ensureHome() {
  const crumb = await $('[data-testid="breadcrumb-assessments"]');
  if (await crumb.isExisting()) {
    await clickTestId("breadcrumb-assessments");
  }
  const switcher = await $('[data-testid="ctx-switch-assessment"]');
  if (await switcher.isExisting()) {
    await clickTestId("ctx-switch-assessment");
  }
}

/** Complete first-run setup if the wizard is showing; skip the in-app tour. */
export async function ensureReady() {
  await attachWindow();
  await acceptConfirms();

  const wizard = await $('[data-testid="setup-wizard"]');
  if (await wizard.isExisting()) {
    await clickTestId("setup-next");
    await expectTestId("setup-storage-persistent");
    await clickTestId("setup-next");
    await expectTestId("setup-persona-cybersecStudent");
    await clickTestId("setup-next");
    const tutorial = await $('[data-testid="setup-tutorial"]');
    await tutorial.waitForExist({ timeout: 10000 });
    if (await tutorial.isSelected()) {
      await clickTestId("setup-tutorial");
    }
    await clickTestId("setup-finish");
  }

  await expectTestId("main-content", 20000);
}

export async function createAssessment(name = assessmentName) {
  await ensureHome();
  await visitNav("assessments");
  if (await $('[data-testid="new-assessment-empty"]').isExisting()) {
    await clickTestId("new-assessment-empty");
  } else {
    await clickTestId("new-assessment");
  }
  await setValue("assessment-name", name);
  await setValue("assessment-domain", assessmentDomain);
  await setValue("assessment-authorized-by", "Security lead");
  await setValue("assessment-auth-ref", `SOW-${stamp}`);
  await setValue("assessment-notes", "Sandboxed integration assessment.");
  await clickTestId("assessment-create");
  await expectTestId("overview-view", 20000);
}
