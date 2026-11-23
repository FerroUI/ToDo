import assert from "node:assert/strict";
import { button, touchInput } from "./touch-input.mjs";

export async function verifyHashtags(ui) {
  const { page, snapshot, saved, control, clickControl, edit, until, resize, capture, focusControl } = ui;
  await resize(1280, 800);
  await clickControl(button("New list"), "create hashtag review list");
  await edit("ListName", "Hashtag review", true);
  const list = (await saved()).lists.find(list => list.name === "Hashtag review");
  let title = "#Release Plan publishing";
  for (const name of [title, "Related note", "Unrelated item"]) await edit("NewTask", name, true);
  const task = (await saved()).tasks.find(task => task.title === title);
  await clickControl(button("Related note"), "related task");
  await edit("TaskNotes", "Track #Release with the team");
  await clickControl(button("Close details"), "save related note");

  const optionNames = () => page.evaluate(`Array.from(document.querySelectorAll('[role="option"]'), node => node.getAttribute('aria-label'))`);
  await until(async () => (await optionNames()).includes(title), "task containers expose task names");
  assert(!(await optionNames()).some(name => name?.includes("todo_ui::")), "internal row types must not reach accessibility");

  // Geometry comes from the native Hyperlink automation peer mirrored to the
  // browser. Real pointer input still goes to FerroUI's canvas and text host.
  const linkPoint = label => until(() => page.evaluate(`(() => {
    const nodes = [...document.querySelectorAll('[role="link"]')].filter(node => node.getAttribute('aria-label') === ${JSON.stringify(label)});
    const boxes = nodes.map(node => node.getBoundingClientRect()).filter(box => box.width > 0 && box.height > 0);
    return boxes.length === 1 ? { x: boxes[0].x + boxes[0].width / 2, y: boxes[0].y + boxes[0].height / 2 } : null;
  })()`), label);
  const document = (await saved()).tasks;
  let point = await linkPoint("Search for #Release");
  await page.click(point.x, point.y);
  await control(button("Close search"), "hashtag search");
  assert.equal((await control(element => element.name === "SearchTasks", "hashtag search query")).text, "#Release");
  await control(button("Related note"), "tag in notes is searchable");
  assert(!(await snapshot()).elements.some(button("Unrelated item")), "unrelated tasks are excluded");
  assert(!(await snapshot()).elements.some(element => element.name === "TaskTitle"), "clicking a tag does not also open task details");
  assert.deepEqual((await saved()).tasks, document, "hashtag navigation never edits tasks");
  const focusedNames = () => page.evaluate(`Array.from(document.querySelectorAll('[aria-activedescendant]'), host =>
    document.getElementById(host.getAttribute('aria-activedescendant'))?.getAttribute('aria-label'))`);
  await until(async () => (await focusedNames()).includes(title), "hashtag search restores focus to a real result");
  await page.press("ArrowDown");
  await until(async () => (await focusedNames()).includes("Related note"), "result navigation works immediately after a hashtag click");
  await capture("hashtag-search");
  await clickControl(button("Close search"), "return from hashtag search");

  await clickControl(button(title), "open hashtag title for editing");
  title = "#Milestone Plan publishing";
  await edit("TaskTitle", title, true);
  await clickControl(button("Close details"), "close renamed hashtag task");
  await until(async () => (await optionNames()).includes(title), "recycled task container updates its accessible name");
  assert(!(await optionNames()).includes("#Release Plan publishing"));
  await linkPoint("Search for #Milestone");
  assert.equal(await page.evaluate(`document.querySelectorAll('[role="link"][aria-label="Search for #Release"]').length`), 0,
    "a retained title must not keep stale link spans");
  await focusControl(element => element.name === "HashtagTaskTitle" && element.automationName === title, "native hashtag text host");
  await page.press("Tab");
  await page.press("Enter");
  await control(button("Close search"), "keyboard hashtag search");
  assert.equal((await control(element => element.name === "SearchTasks", "keyboard search query")).text, "#Milestone");
  assert.equal((await saved()).tasks.find(item => item.id === task.id).completed, false,
    "Enter on a link must not run the task-list completion command");
  await clickControl(button("Close search"), "return after keyboard tag activation");

  await resize(390, 844);
  const touch = touchInput(ui);
  const enabled = await page.send("Emulation.setTouchEmulationEnabled", { enabled: true, maxTouchPoints: 2 });
  assert(!enabled.error, enabled.error?.message);
  try {
    point = await linkPoint("Search for #Milestone");
    await touch.start(point);
    await touch.cancel();
    assert(!(await snapshot()).elements.some(button("Close search")), "cancelled link contact does not navigate");
    await touch.tap(point);
    await control(button("Close search"), "compact touch hashtag search");
    assert(!(await snapshot()).elements.some(element => element.name === "TaskTitle"), "touch does not also open details");
    await capture("hashtag-search-narrow");
    await clickControl(button("Close search"), "return from compact hashtag search");
  } finally {
    await touch.cancel();
    const disabled = await page.send("Emulation.setTouchEmulationEnabled", { enabled: false });
    assert(!disabled.error, disabled.error?.message);
  }
  await resize(1280, 800);
  await clickControl(button("List options"), "hashtag fixture options");
  await clickControl(element => element.type.endsWith("MenuItem") && element.automationName === "Delete list", "remove hashtag fixture");
  await clickControl(button("Delete"), "confirm hashtag fixture cleanup");
  await until(async () => !(await saved()).lists.some(item => item.id === list.id), "hashtag fixture is removed");
  if ((await snapshot()).elements.some(element => element.hit && button("Dismiss message")(element))) {
    await clickControl(button("Dismiss message"), "dismiss fixture status");
  }
  console.log("ok: native inline hashtag pointer, keyboard and touch navigation; cancellation, cross-field search, title recycling and named task options");
}
