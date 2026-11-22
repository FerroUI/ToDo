import assert from "node:assert/strict";

export async function verifyNativeMenus(ui) {
  const { page, saved, snapshot, until, control, clickControl, edit, focusControl, capture } = ui;
  const button = name => element => element.type.endsWith("Button") && element.automationName === name;
  const menu = name => element => element.type.endsWith("MenuItem") && element.automationName === name;
  const choose = name => clickControl(menu(name), `menu item ${name}`);
  const rightClick = async (match, label) => {
    const element = await control(match, label);
    const x = element.x + element.width / 2, y = element.y + element.height / 2;
    await page.mouseMove(x, y);
    for (const [type, buttons] of [["mousePressed", 2], ["mouseReleased", 0]]) {
      await page.send("Input.dispatchMouseEvent", { type, x, y, button: "right", buttons, clickCount: 1, pointerType: "mouse" });
    }
  };
  const createList = async name => {
    await clickControl(button("New list"), "New list");
    await control(element => element.name === "ListName" && element.focused, "focused inline list name");
    await edit("ListName", name, true);
    return until(async () => (await saved()).lists.find(list => list.name === name), `${name} is saved`);
  };

  const destination = await createList("Menu destination");
  const previousIds = new Set((await saved()).lists.map(list => list.id));
  await clickControl(button("New list"), "New list");
  await control(element => element.name === "ListName" && element.focused, "new list inline edit");
  const source = await until(async () => (await saved()).lists.find(list => !previousIds.has(list.id)), "untitled list exists immediately");
  assert.equal(source.name, "Untitled list");
  await edit("ListName", "Discard this name");
  assert.equal((await saved()).lists.find(list => list.id === source.id).name, "Untitled list", "keystrokes must not save");
  await page.press("Escape");
  await until(async () => !(await snapshot()).elements.some(element => element.name === "ListName"), "Escape cancels inline rename");
  assert.equal((await saved()).lists.find(list => list.id === source.id).name, "Untitled list");
  await clickControl(element => element.name === "RenameList", "Rename list header");
  await edit("ListName", "Menu source", true);
  await until(async () => (await saved()).lists.find(list => list.id === source.id).name === "Menu source", "Enter commits the inline name");
  console.log("ok: immediate list creation, transient inline draft, Escape cancellation, and Enter commit");

  for (const title of ["Menu alpha", "Menu beta", "Menu untouched"]) await edit("NewTask", title, true);
  await clickControl(button("List options"), "List options");
  await choose("Select tasks");
  await clickControl(button("Menu alpha"), "first selected task");
  await clickControl(button("Menu beta"), "second selected task");
  // Alpha's visual data did not change when Beta joined selection: menu construction must read live MVVM state.
  await rightClick(button("Menu alpha"), "selected task context menu");
  await control(menu("Delete 2 tasks"), "selection-aware task menu");
  await capture("task-context-menu");
  await choose("Mark as important");
  await until(async () => {
    const tasks = (await saved()).tasks;
    return tasks.find(task => task.title === "Menu alpha").important && tasks.find(task => task.title === "Menu beta").important;
  }, "native command changes both selected tasks");
  assert.equal((await saved()).tasks.find(task => task.title === "Menu untouched").important, false);
  await rightClick(button("Menu alpha"), "selected task context menu");
  await choose("Move tasks to");
  await choose("Menu destination");
  await until(async () => {
    const tasks = (await saved()).tasks;
    return ["Menu alpha", "Menu beta"].every(title => tasks.find(task => task.title === title).list_id === destination.id);
  }, "native submenu moves exactly the selected tasks");
  assert.equal((await saved()).tasks.find(task => task.title === "Menu untouched").list_id, source.id);
  await control(element => element.name === "NewTask", "composer after moving selection");

  // The context target is independent of the selected view.
  await rightClick(button("Menu destination"), "inactive sidebar list");
  await control(menu("Rename list"), "sidebar list menu");
  await choose("Move up");
  await control(button("Menu untouched"), "the original list remains active");
  await focusControl(button("Menu source"), "sidebar list keyboard focus");
  for (const type of ["rawKeyDown", "keyUp"]) {
    await page.send("Input.dispatchKeyEvent", { type, key: "F10", code: "F10", windowsVirtualKeyCode: 121, modifiers: 8 });
  }
  await control(menu("Rename list"), "Shift+F10 native menu");
  await page.press("Escape");
  await control(element => button("Menu source")(element) && element.focused, "focus returns to sidebar list");
  console.log("ok: live multi-selection menus, native destination submenu, explicit sidebar identity, and keyboard focus return");

  await clickControl(button("Create a new group"), "Create a new group");
  await control(element => element.name === "GroupName" && element.focused, "focused inline group name");
  await edit("GroupName", "Menu group", true);
  const group = await until(async () => (await saved()).groups.find(group => group.name === "Menu group"), "inline group creation");
  await clickControl(element => element.name === `GroupOptions${group.id}`, "group options");
  await control(menu("Delete group"), "empty-group command");
  await choose("Add list to group");
  await choose("Menu destination");
  await until(async () => (await saved()).lists.find(list => list.id === destination.id).group_id === group.id, "native group membership command");
  await clickControl(element => element.name === `GroupOptions${group.id}`, "group options");
  await choose("Ungroup lists");
  await until(async () => !(await saved()).groups.some(item => item.id === group.id), "ungroup command");
  const after = await saved();
  assert.equal(after.lists.find(list => list.id === destination.id).group_id, null);
  assert(after.tasks.some(task => task.title === "Menu alpha" && task.list_id === destination.id), "ungroup must retain tasks");
  await clickControl(button("Sort tasks"), "Sort tasks");
  await choose("Added to My Day");
  await until(async () => (await saved()).lists.find(list => list.id === source.id).sort === "MyDay", "Added to My Day sort persists");
  console.log("ok: inline group naming, add/remove membership, preserving ungroup, and Added to My Day sort");
}
