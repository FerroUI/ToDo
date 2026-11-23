import assert from "node:assert/strict";
import { revealSettingsControl } from "./settings-input.mjs";

const button = label => element => element.type.endsWith("Button") && element.automationName === label;
const menu = label => element => element.type.endsWith("MenuItem") && element.automationName === label;

// The Graph boundary has separate transport/account/consent tests. Here supplied
// host capabilities/catalogue drive real native form, menu and confirmation UI.
export async function verifyCategoryManagement(ui) {
  const { page, saved, snapshot, resize, control, clickControl, edit, until, capture } = ui;
  const original = (await saved()).settings;
  const category = { id: "category-review", name: "#Release, 2027", preset: "preset7", color: "#0078D4", color_name: "Blue" };
  const visible = (match, label) => revealSettingsControl(ui, match, label);
  const bounded = (element, width) => {
    assert(element.x >= -1 && element.x + element.width <= width + 1, "category management stays inside its viewport");
    assert(element.height >= 32, "category action keeps a usable touch target");
  };
  await page.evaluate(`todoApp.cloudCapabilities(true,true,true); todoApp.cloudState(true,'Category review account'); todoApp.setIntegrationPreference('outlook_categories_enabled',true); todoApp.setIntegrationPreference('outlook_categories_write_enabled',false); todoApp.outlookCategories(${JSON.stringify(JSON.stringify([category]))});`);
  try {
    for (const [width, label] of [[1280, "wide"], [320, "narrow"]]) {
      await resize(width, 844);
      await clickControl(button("Settings"), "Settings category management");
      await visible(button("Manage Outlook categories"), "manage category catalogue", width);
      await clickControl(button("Manage Outlook categories"), "expand catalogue editor");
      bounded(await visible(button("Enable category editing"), "separate category write consent", width), width);
      await page.evaluate("todoApp.setIntegrationPreference('outlook_categories_write_enabled',true)");
      await visible(element => element.name === "OutlookCategoryName", "native category name", width);
      await visible(button("Create Outlook category"), "create category", width);
      await clickControl(button("Create Outlook category"), "validate empty category");
      await visible(element => element.text?.startsWith("Enter a category name of 1–255"), "empty category validation is inline");
      await visible(element => element.name === "OutlookCategoryName", "category editor after empty-name validation");
      await edit("OutlookCategoryName", category.name);
      await until(async () => (await snapshot()).elements.some(element => element.name === "OutlookCategoryName"
        && element.focused && element.text === category.name), "native category editor accepts the duplicate-name draft");
      await page.press("Enter");
      await visible(element => element.text === "An Outlook category with this name already exists.", "duplicate catalogue names are rejected before a service request");
      await visible(button("New category color: Blue"), "new category color after validation");
      await clickControl(button("New category color: Blue"), "new category native color menu");
      bounded(await control(menu("Purple"), "Purple category preset"), width);
      await clickControl(menu("Purple"), "choose native color preset");
      await control(button("New category color: Purple"), "selected category preset survives rerender");
      assert.equal((await control(element => element.name === "OutlookCategoryName", "retained category draft")).text, category.name);
      await visible(button(`Color for ${category.name}: Blue`), "existing catalogue color", width);
      await clickControl(button(`Color for ${category.name}: Blue`), "existing color menu");
      bounded(await control(menu("Purple"), "existing category color choice"), width);
      await page.press("Escape");
      await visible(button(`Remove category ${category.name} from Outlook`), "remove catalogue category", width);
      await clickControl(button(`Remove category ${category.name} from Outlook`), "review category removal");
      bounded(await visible(button("Confirm category removal"), "confirm catalogue removal", width), width);
      bounded(await control(button("Cancel category removal"), "cancel catalogue removal"), width);
      await capture(`category-management-${label}`);
      await clickControl(button("Cancel category removal"), "cancel without changing Outlook");
      assert(!(await snapshot()).elements.some(element => element.hit && button("Confirm category removal")(element)));
      await clickControl(button("Close settings"), "close catalogue review");
      await page.evaluate("todoApp.setIntegrationPreference('outlook_categories_write_enabled',false)");
    }
  } catch (error) {
    // Preserve the form before cleanup closes Settings and removes its draft.
    try { await capture("category-management-failure"); } catch {}
    throw error;
  } finally {
    if ((await snapshot()).elements.some(element => element.hit && button("Close settings")(element)))
      await clickControl(button("Close settings"), "close category review after failure");
    await page.evaluate(`todoApp.outlookCategories('[]'); todoApp.setIntegrationPreference('outlook_categories_enabled',${original.outlook_categories_enabled === true}); todoApp.setIntegrationPreference('outlook_categories_write_enabled',${original.outlook_categories_write_enabled === true}); todoApp.cloudState(false,'Local account'); todoApp.cloudCapabilities(true,false,false);`);
    await resize(1280, 800);
  }
}
