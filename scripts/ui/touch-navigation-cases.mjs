import assert from "node:assert/strict";
import { button, center, named, noDetails } from "./touch-input.mjs";
import { navigateList, noTransient, topTasks } from "./touch-combination-helpers.mjs";

export async function verifyNavigationReversal(ctx) {
  const { touch, page, saved, control, snapshot, until, capture } = ctx;
  const tasks = await topTasks(ctx), baseline = await saved();
  await touch.tapControl(button(tasks[0].title), "details before a reversed edge Back");
  await control(named("TaskTitle"), "compact details for directional Back");
  await touch.start({ x: 12, y: 300 });
  await touch.move({ x: 130, y: 300 });
  await until(async () => (await snapshot()).elements.some(element => element.name === "TouchGestureMessage"
    && element.text?.includes("Back")), "edge Back is recognized before reversal");
  await touch.move({ x: 2, y: 300 });
  await touch.end();
  await control(named("TaskTitle"), "reversing Back before release preserves details");
  await noTransient(ctx, "reversed Back clears its feedback");
  await touch.swipe({ x: 12, y: 300 }, { x: 130, y: 300 });
  await control(named("NewTask"), "a fresh edge Back still closes details");

  await touch.tapControl(button("Show navigation"), "drawer before wrong-direction motion");
  const target = await control(named(`NavigateList(${ctx.destination.id})`), "unselected list beneath the rejected drag");
  const start = { x: target.x + 40, y: center(target).y };
  await touch.swipe(start, { x: target.x + target.width - 36, y: start.y });
  await control(named("SearchTasks"), "rightward drawer drag cannot activate its underlying list");
  await capture("touch-drawer-rejected-drag");
  const reverse = center(await control(named(`NavigateList(${ctx.destination.id})`), "drawer reversal target"));
  await touch.start(reverse);
  await touch.move({ x: reverse.x - 90, y: reverse.y });
  await touch.move({ x: reverse.x + 20, y: reverse.y });
  await touch.end();
  await control(named("SearchTasks"), "reversing dismissal preserves the drawer");
  await noTransient(ctx, "reversed drawer motion leaves no feedback or rail");
  await page.press("Escape");
  await ctx.row(tasks[0].title);
  await noDetails(ctx, "rejected navigation motion does not activate a task");
  await navigateList(ctx, ctx.destination);
  await control(element => element.type === "TextBlock" && element.text === ctx.destination.name,
    "a fresh ordinary tap can still navigate to the destination");
  await navigateList(ctx, ctx.source);
  assert.deepEqual(await saved(), baseline, "reversed and rejected navigation gestures preserve all document data");
  console.log("ok: signed edge Back/drawer reversal, wrong-way row drag suppression and ordinary navigation recovery");
}

export async function verifyNoopRelease(ctx) {
  const { touch, saved, control, edit } = ctx;
  await navigateList(ctx, ctx.destination);
  let tasks = (await saved()).tasks.filter(task => task.list_id === ctx.destination.id);
  if (!tasks.length) {
    await edit("NewTask", "Touch edge task", true);
    await ctx.until(async () => (await saved()).tasks.filter(task => task.list_id === ctx.destination.id).length === 1,
      "ordinary entry persists the unscrollable edge fixture");
    tasks = (await saved()).tasks.filter(task => task.list_id === ctx.destination.id);
  }
  assert.equal(tasks.length, 1, "the edge fixture has exactly one unscrollable row");
  const baseline = await saved(), point = center(await ctx.row(tasks[0].title));
  await touch.swipe(point, { x: point.x, y: point.y - 12 });
  await noDetails(ctx, "an unscrollable upward drag ending inside its row cannot click it");
  await noTransient(ctx, "edge overscroll leaves no stale gesture");
  assert.deepEqual(await saved(), baseline);
  await navigateList(ctx, ctx.source);
  const source = await topTasks(ctx);
  await touch.start(center(await ctx.row(source[0].title)));
  await control(button("Exit task selection"), "selection before unsupported horizontal motion");
  await touch.end();
  const row = await ctx.row(source[1].title);
  await touch.swipe({ x: row.x + 20, y: center(row).y }, { x: row.x + 115, y: center(row).y });
  await control(element => element.type === "TextBlock" && element.text === "1 selected",
    "unsupported horizontal motion cannot toggle another selected row");
  await noDetails(ctx, "unsupported movement cannot open details");
  await noTransient(ctx, "selection-mode horizontal motion cannot reveal a rail");
  await touch.tapControl(button("Exit task selection"), "leave the no-op selection fixture");
  assert.deepEqual(await saved(), baseline, "no-op movement and selection preserve the document");
  console.log("ok: unscrollable-edge release and unsupported selection-mode motion never become native clicks");
}
