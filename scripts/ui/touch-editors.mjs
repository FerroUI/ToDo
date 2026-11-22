import assert from "node:assert/strict";
import { named } from "./touch-input.mjs";

export async function verifyDrawerEditor(ctx) {
  const { touch, control, edit, saved, snapshot, capture } = ctx;
  const before = await saved();
  const query = "Touch selection draft";
  await edit("SearchTasks", query);
  const field = await control(named("SearchTasks"), "drawer search editor before selection drag");
  assert.equal(field.text, query, "the search draft is rendered before touching it");
  const from = { x: field.x + Math.min(field.width - 20, 100), y: field.y + field.height / 2 };
  const to = { x: field.x + 12, y: from.y };
  assert(from.x - to.x >= 72, "the real editor drag crosses the drawer dismissal threshold");

  // A native double tap selects a word. The first 9px move deliberately lies between
  // the shell's 8px slop and TextBox's 10px touch radius, where capture used to be stolen.
  await touch.tap(from);
  await touch.start(from);
  await touch.move({ x: from.x - 9, y: from.y }, 1);
  assert(!(await snapshot()).elements.some(named("TouchGestureMessage")),
    "native text selection cannot begin a shell navigation gesture");
  await touch.move(to);
  await touch.end();

  const retained = await control(named("SearchTasks"), "drawer retained after text selection drag");
  assert.equal(retained.text, query, "text selection preserves the unsubmitted search draft");
  assert(!(await snapshot()).elements.some(named("TouchGestureMessage")),
    "text selection leaves no shell gesture feedback");
  assert.deepEqual(await saved(), before, "editing and selecting a search draft cannot change document data");
  await capture("touch-drawer-editor");
  await edit("SearchTasks", "");
  console.log("ok: real drawer-search selection drag preserves navigation, text and document data");
}
