import {
  assessmentName,
  clickTestId,
  createAssessment,
  ensureReady,
  expectTestId,
  setValue,
  targetName,
  targetUrl,
  visitNav,
} from "../helpers";

describe("target and recon", () => {
  it("creates the cookie-session demo target and opens recon", async () => {
    await ensureReady();
    await createAssessment(`${assessmentName} target`);

    await visitNav("targets");
    await clickTestId("new-target");
    await setValue("target-url", targetUrl);
    await setValue("target-name", targetName);
    await clickTestId("target-create");

    await browser.waitUntil(
      async () =>
        (await $('[data-testid="target-overview-recon"]').isExisting()) ||
        (await $('[data-testid="nav-recon"]').isExisting()),
      {
        timeout: 30000,
        interval: 500,
        timeoutMsg: "target detect did not reach overview/recon",
      }
    );

    if (await $('[data-testid="target-overview-recon"]').isExisting()) {
      await clickTestId("target-overview-recon");
    } else {
      await visitNav("recon");
    }

    if (await $('[data-testid="dest-step-proxy"]').isExisting()) {
      await clickTestId("dest-step-proxy");
    }
    await expectTestId("proxy-start");
    if (await $('[data-testid="advanced-toggle"]').isExisting()) {
      await clickTestId("advanced-toggle");
      await clickTestId("advanced-toggle");
    }
  });
});
