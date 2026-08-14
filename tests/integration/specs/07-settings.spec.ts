import { ensureReady, expectTestId, visitNav } from "../helpers";

describe("settings", () => {
  it("loads settings without writing host app-data paths into the form", async () => {
    await ensureReady();
    await visitNav("settings");
    await expectTestId("settings-view");
    await expectTestId("settings-storage-mode");
    const cfg = await $("p.mono");
    if (await cfg.isExisting()) {
      const text = await cfg.getText();
      expect(text).not.toContain("Library/Application Support/com.phishkit.phishkit");
    }
  });
});
