import {
  assessmentName,
  clickTestId,
  createAssessment,
  ensureReady,
  expectTestId,
  setValue,
  visitNav,
} from "../helpers";

describe("sessions", () => {
  it("shows session filters and search", async () => {
    await ensureReady();
    await createAssessment(`${assessmentName} sessions`);
    await visitNav("sessions");
    await expectTestId("sessions-view", 20000);
    await expectTestId("sessions-sync");
    await clickTestId("sessions-sync");
    for (const f of ["creds", "tokens", "attributed", "all"]) {
      await expectTestId(`sessions-filter-${f}`);
      await clickTestId(`sessions-filter-${f}`);
    }
    await expectTestId("sessions-search");
    await setValue("sessions-search", "alice");
    await setValue("sessions-search", "");
  });
});
