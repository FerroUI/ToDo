import assert from "node:assert/strict";
import { composerGeometry, verifyComposerLayout } from "./composer-layout.mjs";

export async function verifyListPresentation(ui) {
  const { page, saved, snapshot, control, clickControl, edit, until, resize, capture } = ui;
  const button = name => element => element.type.endsWith("Button") && element.automationName === name;
  const choose = name => clickControl(element => element.type.endsWith("MenuItem") && element.automationName === name, name);
  await resize(1280, 800);
  const account = await control(element => element.name === "AccountSettings", "account settings button");
  assert.equal(account.automationName, "Account settings", "the account button exposes an action, not its Grid content type");
  assert.equal((await control(element => element.name === "NewTask", "labeled task composer")).automationName,
    "Add a task", "the composer has a stable accessible name independent of its placeholder");
  await clickControl(button("New list"), "new list for sorting");
  const title = "A longer list name gets the full header width";
  await edit("ListName", title, true);
  const list = (await saved()).lists.find(list => list.name === title);
  for (const task of ["Sort bravo", "Sort alpha"]) await edit("NewTask", task, true);
  await verifyComposerLayout(ui, "Sort bravo");
  await verifyComposerHint(ui, button);
  await clickControl(button("Sort tasks"), "sort menu");
  await choose("Alphabetically");
  const row = text => control(button(text), text);
  await until(async () => (await row("Sort alpha")).y < (await row("Sort bravo")).y, "alphabetic row order");
  await clickControl(element => element.name === "ReverseSort", "reverse sort direction");
  await until(async () => (await saved()).lists.find(item => item.id === list.id).sort_reversed, "sort direction is saved");
  await until(async () => (await row("Sort bravo")).y < (await row("Sort alpha")).y, "reverse row order");
  await resize(320, 740);
  const header = await control(element => element.name === "RenameList", "compact list title");
  assert(header.width >= 280, "the title uses the complete compact content width");
  for (const match of [element => element.name === "ReverseSort", button("Remove sort")]) {
    const item = await control(match, "compact sort controls");
    assert(item.x >= 0 && item.x + item.width <= 320, "sort status stays within the viewport");
  }
  await capture("list-sorted-narrow");
  await clickControl(button("Remove sort"), "remove sorting");
  await until(async () => (await saved()).lists.find(item => item.id === list.id).sort === "Manual", "manual order is restored");
  assert(!(await snapshot()).elements.some(element => element.name === "ReverseSort"));
  await control(element => button("Sort tasks")(element) && element.focused,
    "clearing sort restores focus to the retained sort trigger");
  await resize(1280, 800);
  await edit("SearchTasks", "Sort alpha", true);
  await control(button("Close search"), "search exit");
  await control(button("Sort alpha"), "search match");
  assert(!(await snapshot()).elements.some(button("Sort bravo")), "search filters the other row");
  await clickControl(button("Clear search"), "clear the search query");
  await until(async () => (await control(element => element.name === "RenameList", "previous list")).automationName === "Rename list", "clear returns to the previous list");
  await control(button("Sort bravo"), "the previous list's second row");
  assert.equal((await control(element => element.name === "SearchTasks", "cleared search field")).text || "", "");
  // Exercise the separate close affordance when compact navigation is hidden.
  await edit("SearchTasks", "Sort alpha", true);
  await resize(390, 844);
  await clickControl(button("Close search"), "compact close search");
  await control(element => element.name === "RenameList", "previous list after compact search");
  await control(element => element.name === "NewTask" && element.focused, "compact cancellation restores composer focus");
  await resize(1280, 800);
  console.log("ok: labeled account/composer, bounded schedule hint, full-width titles, reversible sort, manual reset, and desktop/compact search return");
}

async function verifyComposerHint(ui, button) {
  const { control, clickControl, edit, resize, capture } = ui;
  await clickControl(button("Sort bravo"), "open details beside the composer");
  for (const width of [800, 980]) {
    await resize(width, 800);
    const before = await control(element => element.name === "NewTask", `${width}px empty composer`);
    await edit("NewTask", "Read tomorrow");
    const hint = await control(element => element.name === "RecognizedSchedule", `${width}px recognized schedule`);
    const after = await control(element => element.name === "NewTask", `${width}px composer with schedule`);
    await composerGeometry(ui, `${width}px composer beside details`, { hint: true, focused: true, text: "Read tomorrow" });
    assert(after.width >= 200, `${width}px keeps a usable editor beside details`);
    assert(Math.abs(after.width - before.width) <= 1, "recognized dates must not consume the editor's width");
    assert(hint.y >= after.y + after.height - 1, "the schedule hint occupies a separate line below the editor");
    assert(hint.x >= after.x && hint.x + hint.width <= after.x + after.width + 1,
      "the schedule hint stays inside the editor column");
    if (width === 980) await capture("composer-schedule-narrow-pane");
    await edit("NewTask", "");
  }
  await clickControl(button("Close details"), "return from composer layout check");
  await resize(1280, 800);
}
