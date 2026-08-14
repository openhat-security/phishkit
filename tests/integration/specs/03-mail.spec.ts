import {
  assessmentName,
  clickTestId,
  createAssessment,
  ensureReady,
  expectTestId,
  recipientEmails,
  recipientList,
  setValue,
  stamp,
  visitNav,
} from "../helpers";

describe("mail", () => {
  it("saves a template, imports recipients, and shows delivery presets", async () => {
    await ensureReady();
    await createAssessment(`${assessmentName} mail`);

    await visitNav("templates");
    await expectTestId("templates-view");
    await clickTestId("template-starter");
    await setValue("template-name", `Lure link ${stamp}`);
    await clickTestId("template-tab-preview");
    await clickTestId("template-tab-source");
    await clickTestId("template-save");
    await expectTestId("template-name");

    await visitNav("recipients");
    await expectTestId("recipients-view");
    await setValue("recipient-list-name", recipientList);
    await setValue("recipient-paste", recipientEmails);
    await clickTestId("recipient-import");
    const paste = await $('[data-testid="recipient-paste"]');
    await paste.waitForExist({ timeout: 10000 });

    await visitNav("delivery");
    await expectTestId("delivery-view");
    await clickTestId("delivery-preset-gmail");
    await clickTestId("delivery-preset-smtp");
    await clickTestId("delivery-preset-ses_smtp");
    await clickTestId("delivery-preset-resend");
    await clickTestId("delivery-preset-gmail");
    await setValue("delivery-label", "Integration sender");
    await expectTestId("delivery-save");
    await expectTestId("delivery-send-test");
  });
});
