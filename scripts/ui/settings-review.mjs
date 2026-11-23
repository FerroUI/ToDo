import assert from "node:assert/strict";
import { verifyConnectedSettings } from "./connected-review.mjs";

const button = label => element => element.type.endsWith("Button") && element.automationName === label;
const toggle = label => element => element.type === "ToggleSwitch" && element.automationName === label;
function bounded(element, width, label) {
  assert(element.x >= -1 && element.x + element.width <= width + 1,
    `${label} overflows ${width}px: ${JSON.stringify(element)}`);
}

export async function verifySettingsPresentation(ui) {
  const { page, resize, saved, snapshot, control, clickControl, clickText, until, capture } = ui;
  const originalTheme = (await saved()).settings.theme;
  for (const [width, label] of [[1280, "wide"], [320, "narrow"]]) {
    await resize(width, 844);
    await clickControl(button("Settings"), "open Settings for visual review");
    for (const name of ["SettingsTitle", "AccountName"]) {
      bounded(await control(element => element.name === name, name), width, name);
    }
    const setting = await control(toggle("Confirm before deleting"), "native confirmation switch");
    assert(setting.height >= 48, "Settings switch rows retain a usable touch height");
    bounded(setting, width, "Settings switch");
    if (width === 1280) {
      const original = (await saved()).settings.confirm_delete;
      // Use the visible switch track; its native control then owns Space.
      await page.click(setting.x + 20, setting.y + setting.height - 20);
      await until(async () => (await saved()).settings.confirm_delete !== original, "native switch pointer activation");
      await page.press("Space");
      await until(async () => (await saved()).settings.confirm_delete === original, "native switch keyboard activation restores the preference");
    }
    await capture(`settings-${label}`);
    await page.wheel(width - 80, 600, 0, 520);
    await until(async () => (await snapshot()).elements.some(element => element.hit && element.type === "ToggleSwitch"), "settings preferences remain visible after scrolling");
    await capture(`settings-${label}-preferences`);
    let account;
    for (let step = 0; step < 20; step++) {
      account = (await snapshot()).elements.find(element => element.type.endsWith("TextBlock") && element.text === "Microsoft account");
      if (account) break;
      await page.wheel(width - 80, 600, 0, 160);
    }
    assert(account, "Settings includes the Microsoft account section");
    await page.wheel(width - 80, 600, 0, account.y - 100);
    bounded(await control(button("Connect Microsoft account"), "Microsoft sign-in action"), width, "Microsoft sign-in");
    const clientId = (await snapshot()).elements.find(element => element.name === "MicrosoftClientId");
    if (clientId) bounded(clientId, width, "Application client ID");
    await capture(`settings-${label}-microsoft`);
    await page.wheel(width - 80, 600, 0, 2400);
    await control(button("Keyboard shortcuts"), "settings footer actions");
    for (const name of ["Export backup with files", "Restore Microsoft archive…", "Import Outlook PST…", "Export Outlook PST", "Export backup", "Restore backup…"]) {
      bounded(await control(button(name), name), width, name);
    }
    await capture(`settings-${label}-data`);
    if (width === 320) {
      await clickControl(button("Keyboard shortcuts"), "compact keyboard shortcut help");
      await control(button("Close"), "help close action");
      await capture("keyboard-shortcuts-narrow");
      await page.press("Escape");
    } else {
      await clickControl(button("Close settings"), "close wide Settings");
    }
    await until(async () => !(await snapshot()).elements.some(element => element.automationName === "Close settings"), "Settings is dismissed");
  }
  await verifyConnectedSettings(ui);
  await resize(1280, 800);
  const originalList = (await saved()).lists.find(list => list.name === "Test list");
  assert(originalList, "the newly-created list remains available for theme review");
  await clickControl(button(originalList.name), "newly-created list for theme review");
  const selectedCurrent = async width => {
    await control(button("Current list theme"), "the existing color is represented");
    const choices = (await snapshot()).elements.filter(element =>
      element.type.endsWith("Button") && element.automationName?.endsWith(" list theme"));
    const selected = choices.filter(element => element.automationItemStatus === "Selected");
    assert.equal(selected.length, 1, "exactly one list-theme choice is selected");
    assert.equal(selected[0].automationName, "Current list theme");
    for (const choice of choices) {
      assert(choice.hit, `${choice.automationName} stays reachable`);
      bounded(choice, width, choice.automationName);
    }
  };
  // The palette dialog must stay legible in either application theme.
  const changeTheme = async theme => {
    await clickControl(button("Settings"), "theme preference");
    await clickText(theme === "System" ? "Use system setting" : `${theme} theme`);
    await until(async () => (await saved()).settings.theme === theme, `${theme} theme is saved`);
    await clickControl(button("Close settings"), "close theme preference");
  };
  for (const theme of ["Light", "Dark"]) {
    await changeTheme(theme);
    await clickControl(button("List options"), "list theme menu");
    await clickControl(element => element.type.endsWith("MenuItem") && element.automationName === "Change theme", "change list theme");
    await control(button("Blue list theme"), "named list-theme swatches");
    await selectedCurrent(1280);
    await capture(`list-theme-dialog-${theme.toLowerCase()}`);
    await resize(320, 844);
    await selectedCurrent(320);
    await capture(`list-theme-dialog-${theme.toLowerCase()}-narrow`);
    await clickControl(button("Current list theme"), "keep the current list theme");
    await until(async () => !(await snapshot()).elements.some(element => element.automationName === "Current list theme"), "keeping the current theme closes the picker");
    const unchanged = (await saved()).lists.find(list => list.id === originalList.id);
    assert.equal(unchanged.color, originalList.color, "reviewing or keeping Current must preserve the stored accent");
    assert.equal(unchanged.background, originalList.background, "keeping Current must preserve the stored background");
    await resize(1280, 800);
  }
  await changeTheme(originalTheme);
}
