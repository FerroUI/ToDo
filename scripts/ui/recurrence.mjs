import assert from "node:assert/strict";

const button = name => element => element.type.endsWith("Button") && element.automationName === name;
const named = name => element => element.name === name;
const menu = name => element => element.type.endsWith("MenuItem") && (element.automationName === name || element.automationName?.startsWith(`${name},`));

/** Real native controls exercise parser -> draft -> domain -> persistence. */
export async function verifyRecurrence(ui) {
  const { page, saved, snapshot, clickControl, clickText, control, edit, until, navigate, resize, capture } = ui;
  await resize(1280, 900);
  const original = (await saved()).settings;
  const visible = async (match, label) => {
    for (let step = 0; step < 12; step++) {
      if ((await snapshot()).elements.some(element => element.hit && match(element))) return control(match, label);
      await page.wheel(790, 650, 0, 180);
    }
    return control(match, label);
  };
  const settingsToggle = async (name, key, value) => {
    if ((await saved()).settings[key] === value) return;
    await clickControl(button("Settings"), "open schedule settings");
    const toggle = element => element.type.endsWith("ToggleSwitch") && element.automationName === name;
    for (let step = 0; step < 15; step++) {
      if ((await snapshot()).elements.some(element => element.hit && toggle(element))) break;
      await page.wheel(1210, 650, 0, 180);
    }
    await clickControl(toggle, name);
    await until(async () => (await saved()).settings[key] === value, `${name} saved`);
    await clickControl(button("Close settings"), "close schedule settings");
  };
  await settingsToggle("Recognize dates, times and repeats (English)", "recognize_dates", true);
  await settingsToggle("Remove recognized schedules from titles", "remove_recognized_dates", false);
  try {
    await clickControl(button("New list"), "create recurrence review list");
    await edit("ListName", "Recurrence review", true);
    await until(async () => (await saved()).lists.some(list => list.name === "Recurrence review"), "recurrence review list saved");
    const year = await page.evaluate("new Date().getFullYear() + 1");
    const anchor = `${year}-01-01`;
    const title = `Weekly review every Mon and Thu starting ${anchor} for 3 occurrences`;
    await edit("NewTask", title);
    await control(element => element.name === "RecognizedSchedule" && element.text?.includes("3 occurrences"), "complete recurrence preview before submission");
    await page.press("Enter");
    const task = await until(async () => (await saved()).tasks.find(task => task.title === title), "recognized repeat preserves the default title");
    const current = async () => (await saved()).tasks.find(value => value.id === task.id);
    assert.equal(task.recurrence.anchor, anchor);
    assert.deepEqual(task.recurrence.days_of_week, ["monday", "thursday"]);
    assert.deepEqual(task.recurrence.end, { kind: "after", value: 3 });
    await clickText(title);
    await clickControl(named("TaskRepeat"), "open current repeat");
    await clickControl(menu("Custom"), "custom native recurrence editor");
    await control(named("RepeatStart"), "native recurrence start calendar input");
    await clickControl(named("RepeatDayWednesday"), "add Wednesday to weekday selection");
    await visible(named("RepeatCount"), "finite occurrence count");
    await edit("RepeatCount", "0", true);
    await visible(element => element.name === "ScheduleValidation" && element.text?.includes("positive occurrence"), "finite range inline validation");
    assert.equal((await current()).recurrence.end.value, 3, "invalid range cannot change the task");
    await visible(named("RepeatCount"), "correct occurrence count");
    await edit("RepeatCount", "4");
    await visible(button("Save custom repeat"), "save complete native repeat");
    await capture("recurrence-custom-range");
    await clickControl(button("Save custom repeat"), "save weekday set and count together");
    await until(async () => (await current()).recurrence.end.value === 4, "finite recurrence editor saves");
    assert.deepEqual((await current()).recurrence.days_of_week, ["monday", "wednesday", "thursday"]);
    await page.press("Escape");
    await page.press("z", ["ctrl"]);
    await until(async () => (await current()).recurrence.end.value === 3, "one undo restores the whole recurrence edit");
    assert.deepEqual((await current()).recurrence.days_of_week, ["monday", "thursday"]);
    await page.press("z", ["ctrl", "shift"]);
    await until(async () => (await current()).recurrence.end.value === 4, "redo restores the finite schedule");

    const rangeTitle = `Range review from ${year}-02-01 through ${year}-02-05 at 09:30`;
    await edit("NewTask", rangeTitle, true);
    const range = await until(async () => (await saved()).tasks.find(task => task.title === rangeTitle), "recognized date range saved");
    assert.equal(range.start_date, `${year}-02-01`); assert.equal(range.due_date, `${year}-02-05`);
    assert.equal(range.reminder.minutes_after_midnight, 570);
    await clickText(rangeTitle);
    await control(named("TaskStartDate"), "start date appears in task details");
    await clickControl(named("TaskDueDate"), "edit task date range");
    await clickControl(menu("Pick a date"), "open range calendar");
    await edit("TaskRangeStart", `${year}-02-06`);
    await clickControl(button("Save"), "validate reversed range");
    await control(element => element.name === "ScheduleValidation" && element.text?.includes("on or after"), "reversed range remains in native editor");
    assert.equal((await saved()).tasks.find(task => task.id === range.id).start_date, `${year}-02-01`);
    await edit("TaskRangeStart", `${year}-01-31`);
    await clickControl(button("Save"), "save corrected start date");
    await until(async () => (await saved()).tasks.find(task => task.id === range.id).start_date === `${year}-01-31`, "corrected start date persisted");
    await navigate("Page.reload", { ignoreCache: true });
    assert.equal((await current()).recurrence.end.value, 4);
    assert.equal((await saved()).tasks.find(task => task.id === range.id).start_date, `${year}-01-31`);
    console.log("ok: English recurrence/ranges, native weekday sets and finite validation, one-step undo/redo, corrected start date, and reload persistence");
  } finally {
    await page.press("Escape");
    await settingsToggle("Recognize dates, times and repeats (English)", "recognize_dates", original.recognize_dates !== false);
    await settingsToggle("Remove recognized schedules from titles", "remove_recognized_dates", original.remove_recognized_dates === true);
    await resize(1280, 800);
  }
}
