import assert from "node:assert/strict";
import { sleep } from "../../FerroUI/scripts/browser/harness.mjs";
import { dragSidebar } from "./sidebar-input.mjs";
import { button, named, center, touchInput } from "./touch-input.mjs";

const menu = name => element => element.type.endsWith("MenuItem") && element.automationName === name;

// Every mutation below enters the compiled controls. Clipboard preparation represents
// another application writing the real system clipboard, never a task-model callback.
export async function verifyBulkParity(ui) {
  const { page, saved, snapshot, control, clickControl, edit, until, resize, capture } = ui;
  await resize(1280, 800);
  await page.send("Emulation.setFocusEmulationEnabled", { enabled: true });
  await page.setPermission("clipboard-read", "granted");
  await page.setPermission("clipboard-write", "granted");
  const originalSettings = { ...(await saved()).settings };
  const choose = name => clickControl(menu(name), name);
  const createList = async name => {
    await clickControl(button("New list"), `create ${name}`);
    await edit("ListName", name, true);
    return until(async () => (await saved()).lists.find(list => list.name === name), `${name} saved`);
  };
  const sidebar = async id => {
    const match = named(`NavigateList(${id})`);
    for (let turn = 0; turn < 12; turn++) {
      const target = (await snapshot()).elements.find(match);
      if (target?.hit) return target;
      await page.wheel(140, 580, 0, target && target.y < 100 ? -360 : 360);
    }
    return control(match, `sidebar list ${id}`);
  };
  const openList = async id => {
    const target = await sidebar(id);
    await page.click(target.x + target.width / 2, target.y + target.height / 2);
  };
  const tasks = async id => (await saved()).tasks.filter(task => task.list_id === id).sort((a, b) => a.position - b.position || a.id - b.id);
  const order = async id => (await tasks(id)).map(task => task.id);
  const matchTasks = async (ids, predicate) => {
    const document = await saved();
    return ids.every(id => predicate(document.tasks.find(task => task.id === id)));
  };
  const select = async titles => {
    if ((await snapshot()).elements.some(element => element.hit && button("Exit task selection")(element))) {
      await clickControl(button("Exit task selection"), "clear previous selection");
    }
    await clickControl(button("List options"), "list options for selection");
    await choose("Select tasks");
    for (const title of titles) await clickControl(button(title), `select ${title}`);
    await control(element => element.type === "TextBlock" && element.text === `${titles.length} selected`, "stable selected-task count");
  };
  const all = async () => {
    await clickControl(button("List options"), "list options for select all");
    await choose("Select tasks");
    await clickControl(button("Select all visible tasks"), "select all in current list");
  };

  const destination = await createList("Bulk parity destination");
  const source = await createList("Bulk parity source");
  for (const title of ["Bulk C", "Bulk A", "Bulk B", "Bulk D"]) await edit("NewTask", title, true);
  const original = await tasks(source.id);
  const [c, a, b, d] = original;
  await select([c.title, b.title]);
  await clickControl(named("SelectionOptions"), "bulk options toolbar");
  await choose("Mark as important");
  await until(() => matchTasks([c.id, b.id], task => task.important), "bulk importance is saved");
  await dragSidebar(ui, button(b.title), button(d.title), { fraction: .8 });
  await until(async () => JSON.stringify(await order(source.id)) === JSON.stringify([a.id, d.id, c.id, b.id]), "mouse drag preserves ordered selected block");
  await page.press("z", ["ctrl"]);
  await until(async () => JSON.stringify(await order(source.id)) === JSON.stringify(original.map(task => task.id)), "one undo restores the entire mouse drag");
  const beforeCancel = await saved();
  await sidebar(destination.id);
  await dragSidebar(ui, button(c.title), named(`NavigateList(${destination.id})`), { cancel: true });
  assert.deepEqual(await saved(), beforeCancel, "Escape discards the complete selected drag");
  await dragSidebar(ui, button(c.title), named(`NavigateList(${destination.id})`));
  await until(async () => JSON.stringify(await order(destination.id)) === JSON.stringify([c.id, b.id]), "mouse drop moves both tasks in original display order");
  assert.deepEqual(await order(source.id), [a.id, d.id]);
  await openList(destination.id);
  await all();
  await page.press("c", ["ctrl"]);
  await until(() => page.evaluate(`navigator.clipboard.readText().then(text => text === "Bulk C\\nBulk B")`), "native copy writes stable plain-text task order");
  const html = await page.evaluate(`navigator.clipboard.read().then(async items => {
    const item=items.find(item=>item.types.includes('text/html'));
    return item ? await (await item.getType('text/html')).text() : '';
  })`);
  assert(html.includes("data-ferroui-todo-v1="), "system clipboard retains the portable structured task format");
  await openList(source.id);
  await page.press("v", ["ctrl"]);
  await until(async () => (await tasks(source.id)).length === 4, "native paste creates both task copies");
  const copies = (await tasks(source.id)).filter(task => !original.some(old => old.id === task.id));
  assert.deepEqual(copies.map(task => task.title), [c.title, b.title]);
  assert(copies.every(task => task.important), "clipboard metadata survives the real browser clipboard");
  await openList(destination.id);
  await all();
  await page.press("x", ["ctrl"]);
  await until(async () => (await snapshot()).elements.some(element => element.text?.includes("tasks cut")), "native cut succeeds without deleting sources");
  assert.deepEqual(await order(destination.id), [c.id, b.id]);
  const countBeforeCutPaste = (await saved()).tasks.length;
  await openList(source.id);
  await page.press("v", ["ctrl"]);
  await until(async () => (await tasks(destination.id)).length === 0, "cut paste moves original source identifiers");
  assert.equal((await saved()).tasks.length, countBeforeCutPaste);
  await page.press("z", ["ctrl"]);
  await until(async () => JSON.stringify(await order(destination.id)) === JSON.stringify([c.id, b.id]), "one undo restores the complete cut transfer");

  await page.evaluate(`navigator.clipboard.writeText("- [ ] Bulk pasted one\\r\\n• Bulk pasted two\\n\\n").then(()=>true)`);
  await page.press("v", ["ctrl"]);
  await until(async () => (await tasks(source.id)).some(task => task.title === "Bulk pasted two"), "multi-line clipboard creates separate tasks");
  assert((await tasks(source.id)).some(task => task.title === "Bulk pasted one"));
  await page.press("z", ["ctrl"]);
  await until(async () => (await tasks(source.id)).length === 4, "multi-line paste is one undo transaction");
  const beforeEditor = await saved();
  await edit("NewTask", "Native editor clipboard");
  await page.press("a", ["ctrl"]);
  await page.press("x", ["ctrl"]);
  await until(() => page.evaluate(`navigator.clipboard.readText().then(text => text === "Native editor clipboard")`), "text editor owns cut");
  await page.press("v", ["ctrl"]);
  await until(async () => (await control(named("NewTask"), "native composer after paste")).text === "Native editor clipboard", "text editor owns paste");
  assert.deepEqual(await saved(), beforeEditor, "editor clipboard shortcuts never create or remove tasks");
  await edit("NewTask", "");
  console.log("ok: ordered multi-task mouse drag, cancellation, native task copy/cut/paste, multi-line paste and editor precedence");

  await select([a.title, d.title]);
  await clickControl(named("SelectionOptions"), "bulk category toolbar command");
  await choose("Categories");
  await edit("BulkCategoryName", "Bulk category", true);
  await until(() => matchTasks([a.id, d.id], task => task.tags.includes("Bulk category")), "category added atomically to selected tasks");
  await clickControl(button("Done"), "close bulk category picker");
  await clickControl(named("SelectionOptions"), "bulk custom date command");
  await choose("Set due date");
  await choose("Pick a date");
  await clickControl(button("Save"), "save custom date for the selection");
  await until(() => matchTasks([a.id, d.id], task => !!task.due_date), "custom date applies to the complete selection");
  await page.press("z", ["ctrl"]);
  await until(() => matchTasks([a.id, d.id], task => !task.due_date), "custom bulk date has one undo entry");

  const touch = touchInput(ui);
  await page.send("Emulation.setTouchEmulationEnabled", { enabled: true, maxTouchPoints: 2 });
  try {
    await touch.start(center(await control(button(a.title), "held selected task")));
    // The native Holding recognizer owns its 300ms timer. Selection already exists,
    // so allow that timer before moving the same real contact.
    await sleep(450);
    const target = await control(button(b.title), "touch block insertion target");
    await touch.move({ x: target.x + target.width / 2, y: target.y + target.height * .2 });
    await touch.end();
    await until(async () => JSON.stringify(await order(source.id)) === JSON.stringify([copies[0].id, a.id, d.id, copies[1].id]), "held touch drag preserves both selected tasks");
    const beforeTouchCancel = await saved();
    await touch.start(center(await control(button(a.title), "held task for cancellation")));
    await sleep(450);
    const cancelTarget = await control(button(c.title), "cancelled touch target");
    await touch.move({ x: cancelTarget.x + cancelTarget.width / 2, y: cancelTarget.y + cancelTarget.height * .2 });
    await touch.cancel();
    assert.deepEqual(await saved(), beforeTouchCancel, "touchCancel discards every selected member");
  } finally {
    await touch.cancel();
    await page.send("Emulation.setTouchEmulationEnabled", { enabled: false });
  }
  await clickControl(button("Exit task selection"), "finish bulk touch selection");
  await capture("bulk-task-parity");
  await smartVisibility(ui, originalSettings);
  for (const list of [source, destination]) {
    await openList(list.id);
    await clickControl(button("List options"), "disposable bulk fixture list options");
    await choose("Delete list");
    await clickControl(button("Delete"), "delete disposable bulk fixture list");
    await until(async () => !(await saved()).lists.some(item => item.id === list.id), "bulk fixture cleaned up");
  }
  console.log("ok: bulk category/date editors, held multi-task touch drag, cancellation and persisted smart-list visibility");
}

async function smartVisibility(ui, original) {
  const { page, saved, snapshot, clickControl, control, until } = ui;
  const toggle = name => element => element.type === "ToggleSwitch" && element.automationName === name;
  async function set(label, key, value) {
    if ((await saved()).settings[key] === value) return;
    for (let step = 0; step < 12; step++) {
      const item = (await snapshot()).elements.find(toggle(label));
      if (item?.hit) {
        await page.click(item.x + 20, item.y + item.height - 20);
        await until(async () => (await saved()).settings[key] === value, `${label} preference saved`);
        return;
      }
      await page.wheel(1180, 580, 0, item && item.y < 80 ? -300 : 300);
    }
    await control(toggle(label), label);
  }
  await clickControl(button("Settings"), "smart-list settings");
  await set("Important", "show_important", false);
  await set("Assigned to me", "show_assigned", true);
  await set("Auto-hide empty smart lists", "hide_empty_smart_lists", true);
  await clickControl(button("Close settings"), "close visibility preferences");
  assert(!(await snapshot()).elements.some(named("NavigateImportant")), "explicitly hidden smart list is omitted");
  const assigned = (await saved()).tasks.filter(task => !task.completed && task.assigned_to === "me").length;
  assert.equal((await snapshot()).elements.some(named("NavigateAssignedToMe")), assigned > 0, "auto-hide follows the live smart-list count");
  await clickControl(button("Settings"), "empty-list visibility setting");
  await set("Auto-hide empty smart lists", "hide_empty_smart_lists", false);
  await clickControl(button("Close settings"), "show selected empty smart lists");
  assert((await snapshot()).elements.some(named("NavigateAssignedToMe")), "enabled empty smart list remains visible when auto-hide is off");
  await clickControl(button("Settings"), "restore visibility preferences");
  await set("Important", "show_important", original.show_important ?? true);
  await set("Assigned to me", "show_assigned", original.show_assigned);
  await set("Auto-hide empty smart lists", "hide_empty_smart_lists", original.hide_empty_smart_lists ?? false);
  await clickControl(button("Close settings"), "finish visibility review");
}
