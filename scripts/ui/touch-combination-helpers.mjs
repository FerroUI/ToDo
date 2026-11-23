import assert from "node:assert/strict";
import { button, center, named, noDetails, noRail } from "./touch-input.mjs";
import { revealSidebarControl } from "./sidebar-viewport.mjs";

export const dropMarker = element => /^(?:Task|Sidebar)Drop(?:Before|After)$/.test(element.name || "");

export async function noTransient(ctx, label) {
  await ctx.until(async () => !(await ctx.snapshot()).elements.some(element => dropMarker(element)
    || element.name === "TouchGestureMessage" || /^Swipe(?:Delete|MyDay|Move)$/.test(element.name || "")), label);
}

export async function topTasks(ctx) {
  const tasks = (await ctx.saved()).tasks.filter(task => task.list_id === ctx.source.id && !task.completed)
    .sort((a, b) => a.position - b.position);
  assert(tasks.length >= 4, "the combined-gesture fixture has enough distinct task targets");
  await ctx.control(named("NewTask"), "ordinary task list before a combined gesture");
  for (let attempt = 0; attempt < 8; attempt++) {
    const view = await ctx.snapshot();
    const list = view.elements.find(named("TaskList"));
    assert(list, "the native task viewport exists");
    const first = view.elements.find(element => element.hit && button(tasks[0].title)(element));
    if (first && Math.abs(first.y - list.y) <= 1) return tasks;
    const from = { x: list.x + list.width / 2, y: list.y + list.height * .25 };
    await ctx.touch.swipe(from, { x: from.x, y: Math.min(list.y + list.height - 20, from.y + 300) });
  }
  throw new Error("Real touch scrolling did not return to the first task");
}

export async function heldDrag(ctx, source, target) {
  await ctx.touch.start(center(await ctx.row(source.title)));
  await ctx.control(button("Exit task selection"), "native hold enters selection");
  const row = await ctx.row(target.title);
  const point = { x: center(row).x, y: row.y + row.height * .2 };
  await ctx.touch.move(point);
  await ctx.until(async () => (await ctx.snapshot()).elements.some(dropMarker), "the held drag has a real valid target marker");
  return point;
}

export async function endSelection(ctx) {
  await ctx.touch.tapControl(button("Exit task selection"), "leave gesture selection");
  await ctx.control(named("NewTask"), "composer after gesture selection");
}

export async function browserBack(page) {
  const history = await page.send("Page.getNavigationHistory");
  assert(!history.error, history.error?.message);
  assert(history.currentIndex > 0, "the live app exposes a browser Back entry");
  const result = await page.send("Page.navigateToHistoryEntry", { entryId: history.entries[history.currentIndex - 1].id });
  assert(!result.error, result.error?.message);
  await page.evaluate("new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)))");
}

export async function navigateList(ctx, list) {
  if (!(await ctx.snapshot()).elements.some(named("SidebarScroll"))) {
    await ctx.touch.tapControl(button("Show navigation"), "open navigation between gesture cases");
  }
  await revealSidebarControl(ctx, named(`NavigateList(${list.id})`), `navigation to ${list.name}`, { touch: ctx.touch });
  await ctx.touch.tapControl(named(`NavigateList(${list.id})`), `open ${list.name} through navigation`);
  await ctx.control(named("NewTask"), "composer after ordinary list navigation");
}

export async function recoverTap(ctx, task) {
  await ctx.touch.tapControl(button(task.title), "a fresh tap after gesture cancellation");
  assert.equal((await ctx.control(named("TaskTitle"), "details after fresh tap")).text, task.title);
  await ctx.page.press("Escape");
  await ctx.control(named("NewTask"), "return from the recovered tap");
  await noDetails(ctx, "the recovered detail pane closes normally");
  await noRail(ctx);
}
