import assert from "node:assert/strict";
import { reveal } from "./touch-actions.mjs";
import { button, center, named, noDetails, noRail } from "./touch-input.mjs";
import { noTransient, topTasks } from "./touch-combination-helpers.mjs";

export async function verifyRailTransitions(ctx) {
  const { touch, page, saved, snapshot, control, until, edit, capture } = ctx;
  const tasks = await topTasks(ctx);
  const baseline = await saved();
  const marker = await ctx.row(tasks[2].title);
  await reveal(ctx, tasks[0].title, -1);
  await control(named("SwipeDelete"), "rail before vertical scrolling");
  const from = center(await ctx.row(tasks[0].title));
  await touch.swipe(from, { x: from.x + 2, y: from.y - 140 });
  await until(async () => {
    const moved = (await snapshot()).elements.find(element => element.hit && button(tasks[2].title)(element));
    return !moved || moved.y < marker.y - 60;
  }, "scrolling an already-revealed row scrolls the list");
  await noTransient(ctx, "scrolling dismisses the previous rail and its feedback");
  await noDetails(ctx, "rail-to-scroll does not activate the row");
  assert.deepEqual(await saved(), baseline, "rail-to-scroll preserves every task field");

  await topTasks(ctx);
  const draft = (await control(named("NewTask"), "draft before rail-to-refresh")).text || "";
  const origin = await page.evaluate("performance.timeOrigin");
  await edit("NewTask", "Rail transition draft");
  await reveal(ctx, tasks[0].title, -1);
  const start = center(await ctx.row(tasks[0].title));
  await touch.start(start);
  await touch.move({ x: start.x, y: start.y + 110 });
  await until(async () => (await snapshot()).elements.some(element => element.name === "TouchGestureMessage"
    && element.text?.includes("Release to refresh")), "pulling an open rail becomes refresh feedback");
  await noRail(ctx);
  await capture("touch-rail-to-refresh");
  await touch.end();
  await noTransient(ctx, "rail-to-refresh completes without a stale rail or marker");
  assert.equal((await control(named("NewTask"), "draft after rail-to-refresh")).text, "Rail transition draft");
  assert.equal(await page.evaluate("performance.timeOrigin"), origin, "refresh retains the mounted document");
  assert.deepEqual(await saved(), baseline, "rail-to-refresh preserves saved data");
  await edit("NewTask", draft);
  console.log("ok: revealed rail transitions to native scrolling and pull-refresh without activation or draft loss");
}

export async function verifyDirectionalRecovery(ctx) {
  const { touch, page, saved, control } = ctx;
  const tasks = await topTasks(ctx);
  const baseline = await saved();
  const row = await ctx.row(tasks[1].title), start = center(row);
  await touch.start(start);
  await touch.move({ x: start.x - 95, y: start.y });
  await control(named("SwipeDelete"), "horizontal axis is claimed before orthogonal movement");
  await touch.move({ x: start.x - 95, y: start.y + 65 });
  assert(Math.abs((await ctx.row(tasks[1].title)).y - row.y) <= 1,
    "an established horizontal swipe cannot become vertical scrolling");
  await touch.move(start);
  await touch.end();
  await noTransient(ctx, "reversing a swipe to its origin closes the rail");
  await noDetails(ctx, "a reversed drag cannot become a release click");

  const vertical = center(await ctx.row(tasks[1].title));
  await touch.start(vertical);
  await touch.move({ x: vertical.x, y: vertical.y - 90 });
  await touch.move({ x: vertical.x + 95, y: vertical.y - 90 });
  await touch.end();
  await noTransient(ctx, "an established vertical scroll cannot reveal a horizontal rail");
  await noDetails(ctx, "orthogonal scrolling cannot activate a task");
  await topTasks(ctx);
  await reveal(ctx, tasks[0].title, 1);
  await control(named("SwipeMove"), "a fresh swipe works after reversed and orthogonal motion");
  await page.press("Escape");
  await noTransient(ctx, "Escape closes the recovered action rail");
  assert.deepEqual(await saved(), baseline, "direction changes and recovery preserve the document");
  console.log("ok: horizontal/vertical direction locking, reversal, and subsequent swipe recovery");
}
