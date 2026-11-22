import assert from "node:assert/strict";

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
    await page.wheel(width - 80, 600, 0, 2400);
    await control(button("Keyboard shortcuts"), "settings footer actions");
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
  await resize(1280, 800);
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
    await capture(`list-theme-dialog-${theme.toLowerCase()}`);
    await page.press("Escape");
  }
  await changeTheme(originalTheme);
}
