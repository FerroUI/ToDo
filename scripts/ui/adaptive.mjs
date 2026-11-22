import assert from "node:assert/strict";
import { verifyDrafts } from "./drafts.mjs";

const named = (view, name) => view.elements.find(element => element.name === name);
function bounded(element, width, label) {
  assert(element.x >= -1 && element.x + element.width <= width + 1,
    `${label} must fit the ${width}px viewport: ${JSON.stringify(element)}`);
}

export async function verifyAdaptive(ui, { taskId }) {
  const { page, saved, snapshot, control, clickControl, clickText, until, resize, capture } = ui;
  const task = (await saved()).tasks.find(task => task.id === taskId);
  await clickText(task.title);
  for (const width of [760, 800, 980, 1024, 320, 390]) {
    await resize(width, 844);
    const detail = await control(element => element.name === "TaskTitle", `${width}px details`);
    bounded(detail, width, "Task title");
    const view = await snapshot();
    assert.equal(!!named(view, "NewTask"), width >= 760, `${width}px main-pane visibility`);
    assert.equal(!!named(view, "SearchTasks"), width >= 980, `${width}px inline-navigation visibility`);
    if (width >= 760) {
      const composer = await control(element => element.name === "NewTask", `${width}px composer`);
      bounded(composer, width - 340, "Composer before the 340px detail pane");
      assert(composer.width >= 200, `${width}px leaves a usable list composer`);
      assert(detail.x >= width - 340, `${width}px puts details in the right pane`);
      if (width >= 980) assert(composer.x >= 280, `${width}px keeps the 280px navigation pane`);
      else await verifyDrawer(ui, composer);
    }
    if (width === 800) await capture("details-medium");
    if (width === 390) await capture("details-narrow");
    if (width < 760) {
      await verifyDialog(ui, width);
      await page.press("Escape");
      await control(element => element.name === "NewTask", "the list after closing compact details");
      // No pointer or shortcut supplies focus: Enter must reach the restored task row.
      await page.press("Enter");
      const reopened = await control(element => element.name === "TaskTitle", "details reopened from restored row focus");
      assert.equal(reopened.text, task.title);
    }
  }

  // A deliberate desktop choice survives both compact and medium layouts.
  await resize(1024, 844);
  await clickControl(element => element.automationName === "Hide navigation", "hide desktop navigation");
  await until(async () => !named(await snapshot(), "SearchTasks"), "manual navigation hiding");
  await resize(390, 844);
  await resize(1024, 844);
  assert(!named(await snapshot(), "SearchTasks"), "resizing must retain the hidden navigation preference");
  await clickControl(element => element.automationName === "Show navigation", "restore desktop navigation");
  await control(element => element.name === "SearchTasks", "restored desktop search");
  console.log("ok: 320/390 single panes, 760/800 two panes and overlay navigation, 980/1024 three panes; bounded dialogs and keyboard focus return");
  await verifyDrafts(ui, taskId);
}

async function verifyDrawer(ui, composer) {
  const { page, snapshot, control, clickControl, until } = ui;
  await clickControl(element => element.automationName === "Show navigation", "open overlay navigation");
  const search = await control(element => element.name === "SearchTasks", "drawer search");
  bounded(search, 280, "Drawer search");
  const underneath = named(await snapshot(), "NewTask");
  assert.equal(underneath.x, composer.x, "opening a drawer must not move the list");
  assert.equal(underneath.width, composer.width, "opening a drawer must not squeeze the list");
  await control(element => element.automationName === "Hide navigation" && element.focused, "drawer close-button focus");
  await page.press("Escape");
  await until(async () => !named(await snapshot(), "SearchTasks"), "Escape closes the navigation drawer");
  await control(element => element.name === "TaskTitle", "Escape preserves task details under the drawer");
  await control(element => element.automationName === "Show navigation" && element.focused, "focus returns to the navigation trigger");
}

async function verifyDialog(ui, width) {
  const { page, clickText, control, snapshot, until } = ui;
  await clickText("Add due date");
  for (const label of ["Close", "Previous month", "Next month"]) {
    bounded(await control(element => element.automationName === label, label), width, `${label} in the dialog`);
  }
  bounded(await control(element => element.type.endsWith("TextBlock") && element.text === "Due date", "dialog title"), width, "Dialog title");
  bounded(await control(element => /(?:TextBlock|AccessText)$/.test(element.type) && element.text === "Save", "dialog save action"), width, "Dialog save action");
  await page.press("Escape");
  await until(async () => !(await snapshot()).elements.some(element => element.automationName === "Close"), "the date dialog closes");
  await control(element => element.name === "TaskTitle", "date dialog dismissal preserves details");
}
