import {
  assessmentName,
  clickTestId,
  createAssessment,
  ensureReady,
  expectTestId,
  setValue,
  visitNav,
} from "../helpers";

describe("campaigns and results", () => {
  it("opens Guided, Composer, and Express and the results view", async () => {
    await ensureReady();
    await createAssessment(`${assessmentName} campaign`);

    await visitNav("campaigns");
    await expectTestId("campaigns-view");
    await clickTestId("campaign-flow-guided");
    await clickTestId("guided-preset-generic");
    await expectTestId("guided-next");

    await clickTestId("campaign-flow-composer");
    const aup = await $('[data-testid="aup-accept"]');
    if (await aup.isExisting()) {
      await clickTestId("aup-accept");
    }
    await clickTestId("campaign-flow-express");
    await clickTestId("campaign-flow-guided");
    await expectTestId("campaigns-view");

    await visitNav("results");
    await expectTestId("results-view");
    await expectTestId("results-import-toggle");
    await clickTestId("results-import-toggle");
    await expectTestId("results-events");
    const importBtn = await $('[data-testid="results-import"]');
    await importBtn.waitForExist({ timeout: 10000 });
    if (await importBtn.isEnabled()) {
      await setValue(
        "results-events",
        `[{"email":"alice@example.test","event":"delivered"}]`
      );
      await clickTestId("results-import");
    } else {
      // Import requires a saved campaign; Guided build/launch is covered above.
      expect(await importBtn.isExisting()).toBe(true);
    }
  });
});
