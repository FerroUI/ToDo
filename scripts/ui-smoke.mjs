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
  const element = await control(element => element.type.endsWith("TextBlock") && element.text === text, text);
  await page.click(element.x + element.width / 2, element.y + element.height / 2);
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
  await clickText("Test task");
  await edit("NewStep", "First step", true);
  await until(async () => (await currentTask()).steps.some(step => step.title === "First step"), "step creation is saved");
  // The step caused a parent rerender. Its retained title editor must still commit through Enter.
  await edit("TaskTitle", "Edited task", true);
  await until(async () => (await currentTask()).title === "Edited task", "the retained title editor commits");
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

  await clickText("New list");
  await edit("ListName", "Test list", true);
  await until(async () => (await saved()).lists.some(list => list.name === "Test list"), "the list dialog saves");
  console.log("ok: list dialog input and Enter submission");

  const demo = new URL(page.url);
  demo.search = "?demo=1&nooffline=1";
  await navigate("Page.navigate", { url: demo.href });
  await control(element => element.type.endsWith("TextBlock") && element.text === "Plan the week ahead", "demo task rows");
  await capture("my-day");
  await clickText("Review the new designs");
  await control(element => element.name === "TaskTitle", "demo task details");
  await capture("details");
  await page.press("Escape");
  await page.resize(390, 844);
  await until(async () => (await snapshot()).width < 400, "the narrow viewport");
  await control(element => element.name === "NewTask", "narrow task composer");
  await capture("my-day-narrow");
  healthy();
  console.log(`ok: desktop, task-detail and narrow renders; ${page.requests.length} requests; previews in ${output}`);
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
