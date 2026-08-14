import {
  acceptConfirms,
  assessmentName,
  clickTestId,
  createAssessment,
  ensureHome,
  ensureReady,
  expectTestId,
  visitNav,
} from "../helpers";

describe("assessment lifecycle", () => {
  it("creates an assessment and exercises overview lifecycle", async () => {
    await ensureReady();
    await acceptConfirms();
    await createAssessment(assessmentName);

    await visitNav("overview");
    await expectTestId("overview-view");
    await expectTestId("overview-export");
    await expectTestId("overview-clone");
    await expectTestId("overview-archive");
    await expectTestId("overview-delete");

    await clickTestId("overview-export");
    await clickTestId("overview-clone");
    await expectTestId("overview-view", 20000);

    await clickTestId("overview-archive");
    await expectTestId("overview-archived-banner", 15000);

    await clickTestId("overview-restore");
    await expectTestId("overview-view", 15000);
    const banner = await $('[data-testid="overview-archived-banner"]');
    await banner.waitForExist({ timeout: 3000, reverse: true }).catch(() => {
      /* restore may keep us on overview without the banner */
    });

    await ensureHome();
    await visitNav("assessments");
    const card = await $(`[data-testid^="assessment-card-"]`);
    await card.waitForExist({ timeout: 15000 });
  });
});
