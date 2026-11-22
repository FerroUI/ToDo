import assert from "node:assert/strict";
import { verifySettingsPresentation } from "./settings-review.mjs";

const button = label => element => element.type.endsWith("Button") && element.automationName === label;
const menu = label => element => element.type.endsWith("MenuItem") && element.automationName === label;
const named = name => element => element.name === name;
function bounded(element, width, label) {
  assert(element.x >= -1 && element.x + element.width <= width + 1,
    `${label} overflows ${width}px: ${JSON.stringify(element)}`);
}

// Review the retained editors and actual native dialogs using only ordinary input.
// Call after verifyScheduling, with the original task still in its custom list.
export async function verifyDetailPresentation(ui, taskId) {
  const { page, resize, saved, control, clickControl, edit, until, capture } = ui;
  const current = async () => (await saved()).tasks.find(task => task.id === taskId);
  const original = await current();
  const longTitle = "Review accessibility and release notes before the next product planning meeting";
  const longStep = "Check keyboard navigation and long labels in every compact detail view";
  const note = "Notes stay readable when a detail pane becomes narrow.\nReview the dates, steps and release checklist before sharing the plan.";
  await resize(1280, 800);
  await clickControl(button(original.title), "task for detail layout review");
  await edit("TaskNotes", note);
  await page.press("Enter", ["ctrl"]);
  await until(async () => (await current()).notes === note, "multiline note commits with Control+Enter");
  await edit("NewStep", longStep, true);
  const step = await until(async () => (await current()).steps.find(step => step.title === longStep), "long step is saved");
  await edit("TaskTitle", longTitle, true);
  await until(async () => (await current()).title === longTitle, "long title is saved");
  for (const [width, label] of [[1280, "wide"], [320, "narrow"]]) {
    await resize(width, 844);
    const title = await control(named("TaskTitle"), "wrapping task title");
    const stepField = await control(named(`Step${step.id}`), "wrapping step title");
    bounded(title, width, "Task title");
    bounded(stepField, width, "Step title");
    assert(title.height > 40, "long task title must wrap instead of clipping");
    assert(stepField.height > 36, "long step title must wrap instead of clipping");
    await capture(`detail-long-${label}`);
  }

  await clickControl(named(`StepOptions${step.id}`), "long-step context menu");
  await control(menu("Promote to task"), "step menu actions");
  await capture("step-menu-narrow");
  await page.press("Escape");
  await clickControl(button("Delete task"), "task delete confirmation");
  await control(button("Cancel"), "delete cancellation");
  await capture("delete-confirmation-narrow");
  await clickControl(button("Cancel"), "cancel task deletion");
  assert(await current(), "canceling must keep the task");

  await clickControl(named("TaskRepeat"), "repeat menu");
  await clickControl(menu("Custom"), "custom recurrence");
  for (const name of ["RepeatInterval", "RepeatUnit"]) bounded(await control(named(name), name), 320, name);
  await capture("repeat-form-narrow");
  await page.press("Escape");
  await clickControl(named("TaskReminder"), "reminder menu");
  await clickControl(menu("Pick a date & time"), "custom reminder");
  await edit("ReminderTime", "25:99", true);
  bounded(await control(named("ScheduleValidation"), "reminder validation"), 320, "Validation message");
  await capture("reminder-validation-narrow");
  await page.press("Escape");

  // Return the task to the state expected by the remaining gate scenarios.
  await clickControl(named(`StepOptions${step.id}`), "temporary step options");
  await clickControl(menu("Delete step"), "remove temporary step");
  await until(async () => !(await current()).steps.some(item => item.id === step.id), "temporary step is removed");
  await edit("TaskTitle", original.title, true);
  await edit("TaskNotes", original.notes);
  await page.press("Enter", ["ctrl"]);
  await until(async () => (await current()).title === original.title && (await current()).notes === original.notes, "original detail text is restored");
  const restored = await current();
  for (const field of ["steps", "due_date", "reminder", "recurrence"]) assert.deepEqual(restored[field], original[field], `${field} is unchanged by dismissed forms`);
  await page.press("Escape");
  await control(named("NewTask"), "list after detail review");
  await verifySettingsPresentation(ui);
  console.log("ok: wrapped detail editors, compact native menus, confirmation, recurrence and reminder forms; presentation captures");
}
