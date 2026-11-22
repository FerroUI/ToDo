import assert from "node:assert/strict";
import { bottom, centerX, centerY, close, one, right, textPresenter } from "./layout-geometry.mjs";

const named = name => element => element.name === name;
const button = label => element => element.type.endsWith("Button") && element.automationName === label;

export async function composerGeometry(ui, label, { hint = false, focused, text } = {}) {
  return ui.until(async () => {
    const view = await ui.snapshot();
    const get = name => one(view.elements, named(name), `${label}: ${name}`);
    const bar = get("TaskComposer"), row = get("ComposerContentRow"), plus = get("ComposerAddIcon");
    const editor = get("NewTask"), submit = one(view.elements, button("Add task"), "composer submit");
    for (const [name, item] of [["plus", plus], ["editor", editor], ["submit", submit]]) {
      close(centerY(item), centerY(row), `${label}: ${name} is vertically centered in the content row`);
      assert(item.y >= row.y - 1 && bottom(item) <= bottom(row) + 1, `${label}: ${name} fits its row`);
    }
    assert(editor.hit && submit.hit, `${label}: editor and submit remain hit-testable`);
    assert(bar.x >= -1 && right(bar) <= view.width + 1, `${label}: composer fits the viewport`);
    if (focused !== undefined) assert.equal(editor.focused, focused, `${label}: editor focus`);
    if (text !== undefined) assert.equal(editor.text || "", text, `${label}: retained draft`);
    const hints = view.elements.filter(named("RecognizedSchedule"));
    if (hint) {
      const schedule = one(hints, () => true, `${label}: recognized schedule`);
      assert(schedule.y >= bottom(row) - 1 && bottom(schedule) <= bottom(bar) + 1,
        `${label}: schedule occupies a separate, contained line below the primary row`);
      assert(schedule.x >= editor.x && right(schedule) <= right(editor) + 1,
        `${label}: schedule stays inside the editor column`);
    } else {
      assert.equal(hints.length, 0, `${label}: an ordinary draft has no schedule line`);
      // Compare actual native containers; no expected bar height or padding can hide the original offset.
      close(centerY(row), centerY(bar), `${label}: content is centered in the complete composer bar`);
    }
    if (editor.text) close(centerY(textPresenter(view, "NewTask")), centerY(row), `${label}: typed text is centered`);
    return { view, bar, row, plus, editor, submit };
  }, label);
}

function alignWithTask({ view, plus, submit }, title) {
  const row = one(view.elements, button(title), "comparison task row");
  const inRow = element => element.hit && centerY(element) >= row.y && centerY(element) <= bottom(row);
  const complete = one(view.elements, element => inRow(element) && element.type === "CheckBox"
    && element.automationName === "Mark as completed", "task completion");
  const importance = one(view.elements, element => inRow(element)
    && (button("Mark as important")(element) || button("Remove importance")(element)), "task importance");
  const label = one(view.elements, element => /TextBlock$/.test(element.type) && element.text === title, "task title text");
  close(centerX(plus), centerX(complete), "composer plus aligns with task completion");
  close(centerX(submit), centerX(importance), "composer submit aligns with task importance");
  close(textPresenter(view, "NewTask").x, label.x, "composer text aligns with task title text");
}

export async function verifyComposerLayout(ui, taskTitle) {
  const { page, saved, control, clickControl, edit, resize, capture } = ui;
  const before = await saved();
  const original = (await control(named("NewTask"), "composer before layout review")).text || "";
  for (const [width, label] of [[1280, "wide"], [390, "narrow"]]) {
    await resize(width, 844);
    await edit("NewTask", "");
    await page.press("Tab");
    const empty = await composerGeometry(ui, `${label} empty composer`, { focused: false, text: "" });
    await capture(`composer-empty-${label}`);
    await clickControl(named("NewTask"), "focus the empty composer");
    await composerGeometry(ui, `${label} focused composer`, { focused: true, text: "" });
    await edit("NewTask", "Align composer controls");
    alignWithTask(await composerGeometry(ui, `${label} typed composer`, { focused: true, text: "Align composer controls" }), taskTitle);
    await edit("NewTask", "Read tomorrow");
    const scheduled = await composerGeometry(ui, `${label} scheduled composer`, { hint: true, focused: true, text: "Read tomorrow" });
    close(scheduled.editor.width, empty.editor.width, `${label}: schedule preserves the editor width`);
    close(scheduled.row.height, empty.row.height, `${label}: schedule preserves the primary row height`);
    await capture(`composer-schedule-${label}`);
  }
  await edit("NewTask", original);
  await resize(1280, 800);
  assert.deepEqual(await saved(), before, "composer layout and focus changes must not submit or alter task data");
}
