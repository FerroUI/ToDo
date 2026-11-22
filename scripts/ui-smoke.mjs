// A small integration gate over the shipped WASM: real input, durable edits, and rendered previews.
// node scripts/ui-smoke.mjs <built-site> [output-directory]
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import assert from "node:assert/strict";
import { open, sleep, colours } from "../FerroUI/scripts/browser/harness.mjs";

const [site, output = "test-results/ui", ...extra] = process.argv.slice(2);
if (!site || extra.length) throw new Error("usage: ui-smoke.mjs <built-site> [output-directory]");
await mkdir(output, { recursive: true });
const page = await open(site, { query: "?nooffline=1", width: 1280, height: 800, network: true, navigate: false });
const failures = [], requests = new Map();
page.on("Network.requestWillBeSent", event => requests.set(event.requestId, event.request.url));
page.on("Network.responseReceived", event => {
  if (event.response.status >= 400) failures.push(`HTTP ${event.response.status}: ${event.response.url}`);
});
page.on("Network.loadingFailed", event => {
  if (!event.canceled) failures.push(`${event.errorText}: ${requests.get(event.requestId) || event.requestId}`);
});

const snapshot = async () => JSON.parse(await page.evaluate("globalThis.todoApp.inspectView()"));
// Read saved data directly: exportData() flushes editors and could mask a broken UI commit handler.
const saved = async () => JSON.parse(await page.evaluate("localStorage.getItem('ferroui.todo.v1') || '{}'"));
function healthy() {
  const errors = [...failures, ...page.errors];
  if (errors.length) throw new Error(errors.join("\n"));
}
async function until(check, label, timeout = 10000) {
  const end = Date.now() + timeout;
  let lastError;
  while (Date.now() < end) {
    healthy();
    try { const value = await check(); if (value) return value; }
    catch (error) { lastError = error; }
    await sleep(75);
  }
  throw new Error(`Timed out: ${label}${lastError ? ` (${lastError.message})` : ""}`);
}
async function ready() {
  await until(() => page.evaluate(`(() => {
    const splash = document.querySelector('.ferroui-splash');
    return !!document.querySelector('canvas') && !!globalThis.todoApp?.inspectView
      && (!splash || (splash.classList.contains('splash-close') && getComputedStyle(splash).display === 'none'));
  })()`), "the application draws and hides its splash", 60000);
  await control(element => element.name === "NewTask", "task composer");
}
async function control(match, label) {
  return until(async () => {
    const matches = (await snapshot())?.elements.filter(element => element.hit && match(element)) || [];
    assert(matches.length <= 1, `Ambiguous rendered control: ${label}`);
    return matches[0];
  }, `visible, hit-testable ${label}`);
}
async function clickText(text) {
  await clickControl(element => element.type.endsWith("TextBlock") && element.text === text, text);
}
async function clickControl(match, label) {
  const element = await control(match, label);
  await page.click(element.x + element.width / 2, element.y + element.height / 2);
}
async function focusControl(match, label) {
  // Use real Tab navigation: clicking an importance toggle would mutate it while focusing.
  for (let tabs = 0; tabs <= 24; tabs++) {
    const element = await control(match, label);
    if (element.focused) return;
    if (tabs < 24) await page.press("Tab", ["shift"]);
  }
  throw new Error(`Cannot reach ${label} with keyboard navigation`);
}
async function edit(name, text, enter = false) {
  const field = await control(element => element.name === name, name);
  await page.click(field.x + field.width / 2, field.y + field.height / 2);
  await page.press("a", ["ctrl"]);
  await page.type(text);
  if (enter) await page.press("Enter");
}
async function navigate(command, parameters) {
  const previous = page.navigations.length;
  const result = await page.send(command, parameters);
  assert(!result.error, result.error?.message);
  await until(() => page.navigations.length > previous, "a fresh document after navigation");
  await ready();
}
async function capture(name) {
  await sleep(200);
  healthy();
  const image = await page.screenshot(path.join(output, `${name}.png`));
  assert(colours(image) > 40, `${name} does not contain a rendered interface`);
  await writeFile(path.join(output, `${name}.json`), JSON.stringify(await snapshot()));
}

let failure;
try {
  await page.navigate();
  await ready();
  await edit("NewTask", "Test task", true);
  const task = await until(async () => (await saved()).tasks?.find(task => task.title === "Test task"), "task creation is saved");
  const currentTask = async () => (await saved()).tasks.find(item => item.id === task.id);
  // There is exactly one row and no detail pane, making its importance toggle unambiguous.
  await focusControl(element => element.type.endsWith("ToggleButton") && element.automationName === "Mark as important", "row importance toggle");
  await page.press("Enter");
  await until(async () => (await currentTask()).important, "Enter activates the focused importance toggle");
  assert.equal((await currentTask()).completed, false, "Enter must not complete the row");
  await focusControl(element => element.type.endsWith("ToggleButton") && element.automationName === "Remove importance", "selected row importance toggle");
  await page.press("Space");
  await until(async () => !(await currentTask()).important, "Space activates the focused importance toggle");
  assert.equal((await currentTask()).completed, false, "Space must not also complete the row");
  console.log("ok: focused row importance toggle owns Enter and Space");

  await clickText("Test task");
  await edit("NewStep", "First step", true);
  const step = await until(async () => (await currentTask()).steps.find(step => step.title === "First step"), "step creation is saved");
  // The step caused a parent rerender. Its retained title editor must still commit through Enter.
  await edit("TaskTitle", "Edited task", true);
  await until(async () => (await currentTask()).title === "Edited task", "the retained title editor commits");
  await clickControl(element => element.name === `StepOptions${step.id}`, "step options");
  await control(element => element.type.endsWith("TextBlock") && element.text === "Promote to task", "step menu");
  await page.press("Escape");
  await control(element => element.name === `StepOptions${step.id}` && element.focused, "restored step-options focus");
  // No pointer input between dismissal and submission: shortcuts must retain their route.
  const beforeShortcut = await currentTask();
  await page.press("n", ["ctrl"]);
  await control(element => element.name === "NewTask" && element.focused, "keyboard-focused task composer");
  await page.type("Keyboard task tomorrow");
  assert.equal((await control(element => element.name === "RecognizedSchedule", "recognized due-date preview")).text, "Tomorrow");
  const tomorrow = await page.evaluate(`(() => {
    const date = new Date();
    date.setDate(date.getDate() + 1);
    return [date.getFullYear(), String(date.getMonth() + 1).padStart(2, '0'), String(date.getDate()).padStart(2, '0')].join('-');
  })()`);
  await page.press("Enter");
  const keyboardTask = await until(async () => (await saved()).tasks.find(task => task.title === "Keyboard task tomorrow"), "task creation after modal dismissal retains the recognized title");
  assert.notEqual(keyboardTask.id, task.id);
  assert.equal(keyboardTask.list_id, task.list_id);
  assert.equal(keyboardTask.due_date, tomorrow, "recognized due date uses the browser's local calendar");
  assert.deepEqual(await currentTask(), beforeShortcut, "shortcut input must not change the original task");
  console.log("ok: Escape restores focus; Ctrl+N previews and saves tomorrow's task without pointer input");
  await clickText("Edited task");
  // Leave the note focused; pagehide/beforeunload must save it without an export or blur helping.
  await edit("TaskNotes", "Saved note");
  await navigate("Page.reload", { ignoreCache: true });
  const restored = await currentTask();
  assert.equal(restored.title, "Edited task");
  assert.equal(restored.notes, "Saved note");
  assert.equal(restored.steps[0].title, "First step");
  await clickText("Edited task");
  assert.equal((await control(element => element.name === "TaskNotes", "restored note")).text, "Saved note");
  console.log("ok: task, step, retained title editor, focused-note flush, and reload persistence");

  await clickControl(element => element.name === `StepOptions${step.id}`, "restored step options");
  await clickText("Promote to task");
  const promoted = await until(async () => (await saved()).tasks.find(item => item.id !== task.id && item.title === "First step"), "step promotion creates a saved task");
  assert.equal(promoted.list_id, task.list_id);
  assert.equal(promoted.completed, step.completed);
  assert(!(await currentTask()).steps.some(item => item.id === step.id), "promotion removes the source step");
  await control(element => element.name === "NewStep" && element.focused, "new-step focus after its menu trigger is removed");
  console.log("ok: step promotion preserves its list and completion state, removes the source, and restores editing focus");

  await clickText("New list");
  await edit("ListName", "Test list", true);
  await until(async () => (await saved()).lists.some(list => list.name === "Test list"), "the list dialog saves");
  console.log("ok: list dialog input and Enter submission");

  const textControl = text => control(element => element.type.endsWith("TextBlock") && element.text === text, text);
  const dragText = async (from, to, { before = false, cancel = false } = {}) => {
    const source = await textControl(from), target = await textControl(to);
    const x = source.x + source.width / 2, y = source.y + source.height / 2;
    const tx = target.x + target.width / 2, ty = target.y + target.height * (before ? 0.2 : 0.5);
    // Preserve the real held button; harness mouseMove intentionally describes an ordinary hover.
    const move = (x, y) => page.send("Input.dispatchMouseEvent", { type: "mouseMoved", x, y, buttons: 1, pointerType: "mouse" });
    const escape = type => page.send("Input.dispatchKeyEvent", { type, key: "Escape", code: "Escape", windowsVirtualKeyCode: 27 });
    await page.mouseMove(x, y);
    await page.mouseDown(x, y);
    await move(x + 12, y); // Cross the source threshold, then enter the framework drag route.
    await move(x + 14, y);
    await move(tx, ty);
    if (cancel) { await escape("rawKeyDown"); await move(tx + 1, ty); }
    await page.mouseUp(tx, ty);
    if (cancel) await escape("keyUp");
    await sleep(100);
  };
  const testList = (await saved()).lists.find(list => list.name === "Test list");
  await clickText("My Day");
  await dragText("Edited task", "Test list");
  await until(async () => (await currentTask()).list_id === testList.id, "dragging onto a sidebar list saves the move");
  await clickText("Test list");
  await edit("NewTask", "Drag task", true);
  const dragTask = await until(async () => (await saved()).tasks.find(task => task.title === "Drag task"), "the second list task is saved");
  await dragText("Drag task", "Edited task", { before: true });
  await until(async () => {
    const tasks = (await saved()).tasks;
    return tasks.find(task => task.id === dragTask.id).position < tasks.find(item => item.id === task.id).position;
  }, "manual drag reordering persists the new positions");
  await until(async () => (await textControl("Drag task")).y < (await textControl("Edited task")).y, "the rendered rows follow their new order");
  assert(!(await snapshot()).elements.some(element => element.name === "TaskTitle"), "a drag release must not open task details");
  const beforeCancel = await saved();
  await dragText("Edited task", "Drag task", { before: true, cancel: true });
  assert.deepEqual(await saved(), beforeCancel, "Escape followed by movement and release must not mutate the document");
  console.log("ok: real drag moves between lists, reorders saved/rendered rows, and cancels without changes");

  const demo = new URL(page.url);
  demo.search = "?demo=1&nooffline=1";
  await navigate("Page.navigate", { url: demo.href });
  await control(element => element.type.endsWith("TextBlock") && element.text === "Plan the week ahead", "demo task rows");
  await capture("my-day");
  await clickText("Review the new designs");
  await control(element => element.name === "TaskTitle", "demo task details");
  await capture("details");
  await page.press("Escape");
  await until(async () => !(await snapshot()).elements.some(element => element.name === "TaskTitle"), "Escape closes task details");
  const chooseTheme = async theme => {
    await clickControl(element => element.automationName === "Settings", "Settings button");
    await clickText(`${theme} theme`);
    await until(() => page.evaluate(`JSON.parse(localStorage.getItem('ferroui.todo.demo.v1') || '{}').settings?.theme === ${JSON.stringify(theme)}`), `${theme} theme is saved`);
    await clickControl(element => element.automationName === "Close settings", "Close settings button");
    await until(async () => !(await snapshot()).elements.some(element => element.automationName === "Close settings"), "Settings closes");
  };
  await chooseTheme("Dark");
  await capture("my-day-dark");
  await chooseTheme("Light");
  await page.resize(390, 844);
  await until(async () => (await snapshot()).width < 400, "the narrow viewport");
  await control(element => element.name === "NewTask", "narrow task composer");
  await capture("my-day-narrow");
  healthy();
  console.log(`ok: desktop, task-detail, dark-theme and narrow renders; ${page.requests.length} requests; previews in ${output}`);
} catch (error) {
  failure = error;
  // Keep the primary error even if a failed renderer cannot provide a diagnostic snapshot.
  try { await page.screenshot(path.join(output, "failure.png")); } catch {}
  try { await writeFile(path.join(output, "failure-view.json"), JSON.stringify(await snapshot())); } catch {}
} finally {
  try { await writeFile(path.join(output, "browser.log"), page.log.join("\n")); } catch {}
  try { await page.close(); } catch (error) { failure ??= error; }
}
if (failure) throw failure;
