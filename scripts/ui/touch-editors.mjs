import assert from "node:assert/strict";
import { named } from "./touch-input.mjs";
import { centerY, one, textPresenter } from "./layout-geometry.mjs";

function nativeEditMenu(view) {
  const items = view.elements.filter(element => element.type.endsWith("MenuItem") && element.hit);
  return ["Cut", "Copy", "Select All"].every(command => view.elements.some(element =>
    element.type === "AccessText" && element.text === command && element.hit
    && items.some(item => element.x >= item.x && element.y >= item.y
      && element.x + element.width <= item.x + item.width + 1
      && element.y + element.height <= item.y + item.height + 1)));
}

export async function verifyDrawerEditor(ctx) {
  const { page, touch, control, edit, saved, snapshot, capture, until } = ctx;
  const before = await saved();
  const query = "Touch selection draft";
  await edit("SearchTasks", query);
  const field = await control(named("SearchTasks"), "drawer search editor before selection drag");
  assert.equal(field.text, query, "the search draft is rendered before touching it");
  const text = textPresenter(await snapshot(), "SearchTasks");
  const from = { x: text.x + Math.min(text.width - 12, 90), y: centerY(text) };
  const to = { x: text.x + 3, y: from.y };
  assert(from.x - to.x >= 72, "the real editor drag crosses the drawer dismissal threshold");

  // Keep arbitration separate from native word-selection popups. This first 9px move
  // lies between the shell's 8px slop and TextBox's 10px touch radius.
  await touch.start(from);
  await touch.move({ x: from.x - 9, y: from.y }, 1);
  assert(!(await snapshot()).elements.some(named("TouchGestureMessage")),
    "native editor movement cannot begin a shell navigation gesture");
  await touch.move(to);
  await touch.end();

  const retained = await control(named("SearchTasks"), "drawer retained after native editor drag");
  assert.equal(retained.text, query, "editor dragging preserves the unsubmitted search draft");
  assert.deepEqual(await saved(), before, "editor dragging cannot change document data");

  // Double-tap the first word at a different position from the preceding drag's
  // press, so its click count starts a new sequence. Native selection opens its edit menu.
  const currentText = textPresenter(await snapshot(), "SearchTasks");
  const word = { x: currentText.x + 18, y: centerY(currentText) };
  await touch.tap(word);
  await touch.tap(word);
  await until(async () => nativeEditMenu(await snapshot()), "native word-selection edit menu");
  const selected = one((await snapshot()).elements, named("SearchTasks"), "drawer beneath the native edit menu");
  assert.equal(selected.text, query, "word selection preserves the search draft");
  assert(selected.focused, "the native transient edit menu keeps focus in its editor");
  assert.deepEqual(await saved(), before, "native word selection cannot change document data");
  await capture("touch-editor-menu");
  await page.press("Escape");
  await until(async () => !(await snapshot()).elements.some(element => element.type.endsWith("MenuItem")),
    "Escape dismisses the native edit flyout");
  const restored = await control(named("SearchTasks"), "drawer retained after native edit-menu dismissal");
  assert(restored.focused, "Escape retains the focused search editor");
  assert.equal(restored.text, query, "Escape dismisses only the edit menu without changing the search draft");
  const view = await snapshot();
  one(view.elements, element => element.type.endsWith("Button") && element.hit
    && element.automationName === "Clear search", "external search clear action");
  assert(!view.elements.some(element => element.name === "DeleteButton" && element.textBoxName === "SearchTasks"),
    "drawer search must not show a duplicate native clear button");
  assert(!view.elements.some(named("TouchGestureMessage")),
    "native editing leaves no shell gesture feedback");
  assert.deepEqual(await saved(), before, "editing and selecting a search draft cannot change document data");
  await capture("touch-drawer-editor");
  await edit("SearchTasks", "");
  console.log("ok: native editor drag and word-selection flyout dismissal preserve navigation, focus, draft and document data");
}
