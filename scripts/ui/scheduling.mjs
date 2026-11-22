import assert from "node:assert/strict";

export async function verifyScheduling(ui, taskId) {
  const { page, saved, control, clickControl, clickText, edit, until, capture } = ui;
  const current = async () => (await saved()).tasks.find(task => task.id === taskId);
  const button = name => element => element.type.endsWith("Button") && element.automationName === name;
  const choose = label => clickControl(element => element.type.endsWith("MenuItem")
    && (element.automationName === label || element.automationName?.startsWith(`${label},`)), label);
  const open = name => clickControl(element => element.name === name, name);
  await clickControl(button("Test list"), "task's list for scheduling");
  await clickText((await current()).title);

  await open("TaskDueDate");
  await choose("Tomorrow");
  await until(async () => (await current()).due_date, "quick due date is persisted");
  const tomorrow = (await current()).due_date;
  await clickControl(button("Remove due date"), "direct due-date removal");
  await until(async () => !(await current()).due_date, "due date is removed");
  await open("TaskDueDate");
  await choose("Pick a date");
  await control(element => element.name === "ScheduleCalendar", "native calendar");
  const previous = await control(element => element.automationName === "Previous month", "native previous month");
  const next = await control(element => element.automationName === "Next month", "native next month");
  assert(previous.width > 0 && next.width > 0);
  // The modal focuses Calendar, whose native keyboard route selects adjacent days.
  await page.press("ArrowRight");
  await capture("schedule-calendar");
  await clickControl(button("Save"), "save native calendar selection");
  await until(async () => (await current()).due_date === tomorrow, "native keyboard date selection saves tomorrow");
  await clickControl(button("Remove due date"), "clear selected calendar date");

  await open("TaskReminder");
  await choose("Tomorrow");
  await until(async () => (await current()).reminder?.date === tomorrow, "quick reminder date");
  assert.equal((await current()).reminder.minutes_after_midnight, 9 * 60);
  await open("TaskReminder");
  await choose("Pick a date & time");
  await edit("ReminderTime", "25:99", true);
  await control(element => element.name === "ScheduleValidation" && element.text?.includes("24-hour"), "inline reminder validation");
  assert.equal((await current()).reminder.minutes_after_midnight, 9 * 60, "invalid time cannot alter the saved reminder");
  await control(element => element.name === "ReminderTime" && element.focused, "invalid time regains focus");
  await edit("ReminderTime", "10:15", true);
  await until(async () => (await current()).reminder?.minutes_after_midnight === 615, "corrected custom reminder saves");
  await clickControl(button("Remove reminder"), "direct reminder removal");
  await until(async () => !(await current()).reminder, "reminder is removed");

  await open("TaskRepeat");
  await choose("Custom");
  await edit("RepeatInterval", "0", true);
  await control(element => element.name === "ScheduleValidation" && element.text?.includes("positive"), "inline recurrence validation");
  assert.equal((await current()).recurrence, null);
  await edit("RepeatInterval", "3", true);
  await until(async () => (await current()).recurrence?.every === 3, "custom recurrence persists");
  await clickControl(button("Remove repeat"), "direct repeat removal");
  await until(async () => !(await current()).recurrence, "repeat is removed");
  await page.press("Escape");
  await control(element => element.name === "NewTask", "list after scheduling review");
  console.log("ok: native schedule quick menus, calendar keyboard selection, inline validation, custom correction, and direct removal");
}
