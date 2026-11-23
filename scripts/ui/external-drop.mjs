import assert from "node:assert/strict";
import { button, named, center } from "./touch-input.mjs";

// CDP supplies a real browser drag data store. The page's drag listeners, native
// IDataTransfer adapter, retained controls and view model handle every import.
// No callback imports a document or invokes an application command directly.
export async function verifyExternalDrop(ui) {
  const { page, saved, snapshot, control, clickControl, edit, until, resize, capture } = ui;
  await resize(1280, 800);
  const choose = name => clickControl(element => element.type.endsWith("MenuItem") && element.automationName === name, name);
  async function createList(name) {
    await clickControl(button("New list"), `create ${name}`);
    await edit("ListName", name, true);
    return until(async () => (await saved()).lists.find(list => list.name === name), `${name} saved`);
  }
  async function sidebar(id) {
    const match = named(`NavigateList(${id})`);
    for (let attempt = 0; attempt < 12; attempt++) {
      const target = (await snapshot()).elements.find(match);
      if (target?.hit) return target;
      await page.wheel(140, 580, 0, target && target.y < 100 ? -360 : 360);
    }
    return control(match, `external-drop destination ${id}`);
  }
  async function openList(id) {
    const point = center(await sidebar(id));
    await page.click(point.x, point.y);
  }
  const tasks = async id => (await saved()).tasks.filter(task => task.list_id === id)
    .sort((a, b) => a.position - b.position || a.id - b.id);
  async function drag(target, items, { accept = true, hover = false } = {}) {
    const point = center(target);
    const data = { items, dragOperationsMask: 1 };
    for (const type of ["dragEnter", "dragOver"]) {
      const result = await page.send("Input.dispatchDragEvent", { type, ...point, data });
      assert(!result.error, result.error?.message);
    }
    if (hover) {
      await until(async () => (await snapshot()).elements.some(named("ExternalTaskDropHint")), "native external-drop destination feedback");
    }
    if (!accept) {
      // CDP dragCancel ends only the drag source; a synthetic external drag has
      // no source to dispatch a target leave. Exercise the real native Escape
      // cancellation path before releasing that CDP source state.
      await page.press("Escape");
    }
    const result = await page.send("Input.dispatchDragEvent", { type: accept ? "drop" : "dragCancel", ...point, data });
    assert(!result.error, result.error?.message);
    await page.evaluate("new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)))");
  }
  const plain = text => [{ mimeType: "text/plain", data: text }];
  const html = value => [{ mimeType: "text/html", data: value }, ...plain("Weaker fallback must not become a task")];
  const body = () => control(named("TaskDropSurface"), "native task drop surface");

  const source = await createList("External drop source");
  await edit("NewTask", "External original", true);
  const original = await until(async () => (await tasks(source.id))[0], "external-drop source fixture");
  const destination = await createList("External drop destination");

  const beforeCanceled = await saved();
  await drag(await body(), plain("Canceled external task"), { accept: false, hover: true });
  assert.deepEqual(await saved(), beforeCanceled, "canceled native external drag leaves the whole document unchanged");
  assert(!(await snapshot()).elements.some(named("ExternalTaskDropHint")), "cancellation removes external-drop feedback");

  await drag(await body(), plain("- [ ] External first\r\n• External second\n"), { hover: true });
  await until(async () => (await tasks(destination.id)).length === 2, "plain-text external drop creates both tasks");
  assert.deepEqual((await tasks(destination.id)).map(task => task.title), ["External first", "External second"]);
  await page.press("z", ["ctrl"]);
  await until(async () => (await tasks(destination.id)).length === 0, "one immediate undo removes the complete external text drop");

  const foreign = { ...original, title: "External structured 💼", notes: "Transferred note", important: true,
    tags: ["External category"], steps: [{ id: original.id, title: "Transferred step", completed: true }],
    attachments: [{ id: 70001, name: "foreign.txt", mime_type: "text/plain", size: 1, storage_key: "foreign-private-file" }],
    assigned_to: "another-account", flagged_email: true,
    external: { provider: "Planner", id: "foreign-task", url: "https://example.com/task" },
    microsoft_data: { id: "foreign-task", etag: "foreign-version" },
    linked_resources: [{ id: "foreign-resource", external_id: "remote-authority", application_name: "External application",
      display_name: "Source page", web_url: "https://example.com/source" }] };
  // Its task ID collides with a live source, and cut=true is deliberately foreign.
  // Neither fact grants a transfer lease through external drag and drop.
  const payload = { kind: "ferroui.todo.tasks", version: 1, token: "9".repeat(64), cut: true, tasks: [foreign] };
  const marked = `<ul data-ferroui-todo-v1="${Buffer.from(JSON.stringify(payload), "utf8").toString("hex")}"><li>External structured 💼</li></ul>`;
  await openList(source.id);
  await drag(await sidebar(destination.id), html(marked));
  const copied = await until(async () => (await tasks(destination.id))[0], "structured sidebar drop creates an independent task");
  assert.notEqual(copied.id, original.id);
  assert.equal(copied.title, foreign.title);
  assert.equal(copied.notes, foreign.notes);
  assert.equal(copied.important, true);
  assert.deepEqual(copied.tags, foreign.tags);
  assert.equal(copied.steps.length, 1);
  assert.notEqual(copied.steps[0].id, original.id);
  assert.equal(copied.steps[0].title, "Transferred step");
  assert.equal(copied.steps[0].completed, true);
  assert.deepEqual(copied.attachments, []);
  assert.equal(copied.external ?? null, null);
  assert.equal(copied.microsoft_data ?? null, null);
  assert.equal(copied.assigned_to, null);
  assert.equal(copied.flagged_email, false);
  assert.equal(copied.linked_resources[0].id, "");
  assert.equal(copied.linked_resources[0].external_id, "");
  assert.equal(copied.linked_resources[0].web_url, "https://example.com/source");
  assert.deepEqual((await tasks(source.id)), [original], "an external cut offer never moves or changes a matching local source");
  await page.press("z", ["ctrl"]);
  await until(async () => (await tasks(destination.id)).length === 0, "structured external drop has one undo transaction");

  await openList(destination.id);
  const beforeMalformed = await saved();
  await drag(await body(), html('<ul data-ferroui-todo-v1="zz"><li>Invalid</li></ul>'));
  await until(async () => (await snapshot()).elements.some(element => element.text?.includes("clipboard task data is invalid")), "malformed external task payload reports its error");
  assert.deepEqual(await saved(), beforeMalformed, "malformed structured drops cannot fall back to weaker text or partially import");

  await edit("NewTask", "Native editor keeps task precedence");
  const beforeEditor = await saved();
  await drag(await control(named("NewTask"), "native editor drop target"), plain("Editor text one\nEditor text two"));
  assert.deepEqual(await saved(), beforeEditor, "a drop over a native text editor does not create task rows");
  await edit("NewTask", "");
  await drag(await body(), plain("External recovery task"), { hover: true });
  await until(async () => (await tasks(destination.id)).some(task => task.title === "External recovery task"), "a valid external drop still works after cancellation, malformed data and editor input");
  await capture("external-task-drop");

  for (const list of [destination, source]) {
    await openList(list.id);
    await clickControl(button("List options"), "external-drop fixture list options");
    await choose("Delete list");
    await clickControl(button("Delete"), "delete external-drop fixture list");
    await until(async () => !(await saved()).lists.some(item => item.id === list.id), "external-drop fixture cleaned up");
  }
  console.log("ok: real external text/structured drops, sidebar targeting, copy-only authority, provider/file sanitization, one undo, cancellation and editor precedence");
}
