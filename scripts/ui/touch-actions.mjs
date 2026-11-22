import assert from "node:assert/strict";
import { button, center, named, noDetails, noRail } from "./touch-input.mjs";

export async function reveal(ctx, title, direction) {
  const row = await ctx.row(title);
  const from = { x: row.x + (direction < 0 ? row.width * .75 : 16), y: center(row).y };
  await ctx.touch.swipe(from, { x: from.x + (direction < 0 ? -110 : 190), y: from.y });
  await noDetails(ctx, "a horizontal task-body swipe must not open details or navigate Back");
}

export async function verifyTouchActions(ctx) {
  const { touch, saved, control, until, capture } = ctx;
  const original = await saved();
  const deleted = await ctx.task("Touch 01");
  await reveal(ctx, deleted.title, -1);
  await control(named("SwipeDelete"), "revealed Delete action");
  assert.deepEqual(await saved(), original, "swiping only reveals an action; it cannot delete a task");
  await capture("touch-delete-rail");
  assert(!(await ctx.snapshot()).elements.some(element => element.type === "TextBlock" && element.text === "Rename list"),
    "the stale hover tooltip cannot reopen over the revealed touch action rail");
  await touch.tapControl(named("SwipeDelete"), "open swipe deletion confirmation");
  await control(button("Cancel"), "deletion cancellation");
  assert.deepEqual(await saved(), original, "the destructive swipe requires confirmation");
  await touch.tapControl(button("Cancel"), "cancel swipe deletion");
  assert.deepEqual(await saved(), original, "cancelling preserves the complete document");
  await reveal(ctx, deleted.title, -1);
  await touch.tapControl(named("SwipeDelete"), "confirmable Delete action");
  await touch.tapControl(button("Delete"), "confirm deletion");
  await until(async () => !(await saved()).tasks.some(task => task.id === deleted.id), "confirmed touch deletion is saved");
  assert.deepEqual((await saved()).tasks, original.tasks.filter(task => task.id !== deleted.id),
    "confirmed deletion preserves every other task");
  await ctx.row("Touch 02");
  await capture("touch-after-delete");
  await ctx.dismissStatus();

  const title = "Touch 02";
  const beforeDay = await saved();
  await reveal(ctx, title, 1);
  assert.equal((await control(named("SwipeMyDay"), "revealed My Day action")).automationName, "Add to My Day");
  await control(named("SwipeMove"), "revealed Move action");
  assert.deepEqual(await saved(), beforeDay, "revealing right-side actions cannot change tasks");
  await capture("touch-actions-rail");
  await touch.tapControl(named("SwipeMyDay"), "add through the My Day rail");
  await until(async () => (await ctx.task(title)).my_day, "My Day addition is saved");
  await reveal(ctx, title, 1);
  assert.equal((await control(named("SwipeMyDay"), "updated My Day action")).automationName, "Remove from My Day");
  await touch.tapControl(named("SwipeMyDay"), "remove through the My Day rail");
  await until(async () => !(await ctx.task(title)).my_day, "My Day removal is saved");
  await reveal(ctx, title, 1);
  await touch.tapControl(named("SwipeMove"), "choose a destination by touch");
  await touch.tapControl(button(ctx.destination.name), "touch fixture destination");
  await until(async () => (await ctx.task(title)).list_id === ctx.destination.id, "the swipe Move action saves the destination");
  await noRail(ctx);
  await noDetails(ctx, "rail actions leave the task list open");
  console.log("ok: real touch swipe rails, deletion confirmation/cancellation, My Day toggle, and list movement");
}
