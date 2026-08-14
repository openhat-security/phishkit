import { clickTestId, ensureReady, expectTestId, visitNav } from "../helpers";

describe("setup", () => {
  it("completes first-run wizard and shows the console", async () => {
    await ensureReady();
    await expectTestId("main-content");
    await visitNav("assessments");
    await expectTestId("new-assessment");
  });

  it("settings reports a storage mode after setup", async () => {
    await visitNav("settings");
    await expectTestId("settings-view");
    const mode = await $('[data-testid="settings-storage-mode"]');
    await mode.waitForExist({ timeout: 10000 });
    const text = (await mode.getText()).toLowerCase();
    expect(text.length).toBeGreaterThan(0);
    expect(text).toMatch(/persistent|ephemeral/);
  });
});
