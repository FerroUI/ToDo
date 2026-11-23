// A small integration gate over the shipped WASM: real input, durable edits, and rendered previews.
// node scripts/ui-smoke.mjs <built-site> [output-directory]
import { writeFile } from "node:fs/promises";
import path from "node:path";
import assert from "node:assert/strict";
import { sleep } from "../FerroUI/scripts/browser/harness.mjs";
import { createHarness } from "./ui/harness.mjs";
import { verifyAdaptive } from "./ui/adaptive.mjs";
import { verifyNativeMenus } from "./ui/native-menus.mjs";
import { verifyListPresentation } from "./ui/list-presentation.mjs";
import { verifySidebar } from "./ui/sidebar.mjs";
import { verifyScheduling } from "./ui/scheduling.mjs";
import { verifyDetailPresentation } from "./ui/detail-review.mjs";
import { verifySettingsPresentation } from "./ui/settings-review.mjs";
import { verifyTouch } from "./ui/touch.mjs";
import { runReviewGroups } from "./ui/review-groups.mjs";
import { verifyDemo } from "./ui/demo.mjs";
import { verifyPstRoundTrip } from "./ui/pst-roundtrip.mjs";
import { verifyHashtags } from "./ui/hashtags.mjs";
import { verifyRichNotes } from "./ui/rich-notes.mjs";
import { verifyBulkParity } from "./ui/bulk-parity.mjs";
import { verifyExternalDrop } from "./ui/external-drop.mjs";
import { verifyCategoryManagement } from "./ui/category-management.mjs";
import { verifyWallpapers } from "./ui/wallpapers.mjs";
import { verifyRecurrence } from "./ui/recurrence.mjs";
import { verifyPushSettings } from "./ui/push-settings.mjs";

const [site, output = "test-results/ui", ...extra] = process.argv.slice(2);
if (!site || extra.length) throw new Error("usage: ui-smoke.mjs <built-site> [output-directory]");
const ui = await createHarness(site, output);
const { page, snapshot, saved, until, ready, control, clickControl, clickText,
  focusControl, edit, navigate } = ui;

let failure;
let reviewFailures = [];
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
  await control(element => element.type.endsWith("MenuItem") && element.automationName === "Promote to task", "step menu");
  await page.press("Escape");
  await control(element => element.name === `StepOptions${step.id}` && element.focused, "restored step-options focus");
  // No pointer input between dismissal and submission: shortcuts must retain their route.
  const beforeShortcut = await currentTask();
  // Chrome reserves Ctrl+N before renderer dispatch when CDP supplies a native key code.
  // The app also accepts Ctrl+Alt+N, which exercises the same route without that reservation.
  await page.press("n", ["ctrl", "alt"]);
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
  console.log("ok: Escape restores focus; Ctrl+Alt+N previews and saves tomorrow's task without pointer input");
  await clickText("Edited task");
  // Leave the note focused; pagehide/beforeunload must save it without an export or blur helping.
  await edit("TaskNotes", "Saved note");
  await until(async () => {
    const note = await control(element => element.name === "TaskNotes", "focused note before reload");
    return note.focused && note.text === "Saved note";
  }, "the native rich editor accepts the pending note before host departure");
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
  await until(async () => (await saved()).lists.some(list => list.name === "Test list"), "the inline list name saves");
  console.log("ok: inline list-name input and Enter submission");

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

  await verifyAdaptive(ui, { taskId: task.id });
  await verifyNativeMenus(ui);
  await verifyScheduling(ui, task.id);
  reviewFailures = await runReviewGroups(ui, output, [
    { name: "detail", run: () => verifyDetailPresentation(ui, task.id) },
    { name: "settings", run: () => verifySettingsPresentation(ui) },
    { name: "list", run: () => verifyListPresentation(ui) },
    { name: "hashtags", run: () => verifyHashtags(ui) },
    { name: "rich-notes", run: () => verifyRichNotes(ui) },
    { name: "bulk-parity", run: () => verifyBulkParity(ui) },
    { name: "external-drop", run: () => verifyExternalDrop(ui) },
    { name: "category-management", run: () => verifyCategoryManagement(ui) },
    { name: "push-settings", run: () => verifyPushSettings(ui) },
    { name: "wallpapers", run: () => verifyWallpapers(ui, output) },
    { name: "recurrence", run: () => verifyRecurrence(ui) },
    { name: "touch", run: () => verifyTouch(ui, output) },
    { name: "sidebar", run: () => verifySidebar(ui) },
    { name: "demo", run: () => verifyDemo(ui, output) },
    { name: "pst-roundtrip", run: () => verifyPstRoundTrip(ui, output) },
  ]);
} catch (error) {
  failure = error;
  // Keep the primary error even if a failed renderer cannot provide a diagnostic snapshot.
  try { await page.screenshot(path.join(output, "failure.png")); } catch {}
  try { await writeFile(path.join(output, "failure-view.json"), JSON.stringify(await snapshot())); } catch {}
} finally {
  try { await writeFile(path.join(output, "browser.log"), page.log.join("\n")); } catch {}
  try { await page.close(); } catch (error) { failure ??= error; }
}
if (reviewFailures.length) {
  throw new AggregateError([...reviewFailures.map(review => review.error), ...(failure ? [failure] : [])],
    `UI review groups failed: ${reviewFailures.map(review => `${review.name} (${review.phase})`).join(", ")}${failure ? "; gate cleanup also failed" : ""}`);
}
if (failure) throw failure;
