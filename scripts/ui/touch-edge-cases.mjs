import assert from "node:assert/strict";
import { button, center, named, noDetails, noRail } from "./touch-input.mjs";

async function hold(ctx, title) {
  await ctx.touch.start(center(await ctx.row(title)));
  await ctx.control(button("Exit task selection"), "native holding before a cancellation or long drag");
}

export async function verifyTouchEdgeCases(ctx) {
  const { touch, saved, snapshot, control, until, resize, capture } = ctx;
  const beforeSwipe = await saved();
  const from = center(await ctx.row("Touch 03"));
  await touch.start(from);
  await touch.move({ x: from.x - 80, y: from.y });
  await control(element => element.name === "SwipeDelete", "live swipe before a second finger");
  await touch.addContact({ x: from.x + 35, y: from.y + 65 });
  await touch.move({ x: from.x - 110, y: from.y });
  await touch.end();
  await noRail(ctx);
  await noDetails(ctx, "a second contact cannot activate the swiped row");
  assert.deepEqual(await saved(), beforeSwipe, "a second finger cancels an in-progress swipe without changing data");

  const beforeDrag = await saved();
  await hold(ctx, "Touch 04");
  const target = await ctx.row("Touch 05");
  const drop = { x: target.x + target.width / 2, y: target.y + target.height * .2 };
  await touch.move(drop);
  await touch.addContact({ x: drop.x + 30, y: drop.y + 65 });
  await touch.move({ x: drop.x + 2, y: drop.y + 2 });
  await touch.end();
  assert.deepEqual(await saved(), beforeDrag, "a second finger cancels a held drag even after further movement/release");
  await touch.tapControl(button("Exit task selection"), "leave the cancelled two-finger drag");

  // A shorter viewport forces the original row out of the zero-cache virtualizing panel.
  await resize(390, 700);
  const beforeScroll = await saved();
  await hold(ctx, "Touch 05");
  const viewport = await control(named("TaskList"), "the real list viewport");
  const edge = { x: viewport.x + viewport.width / 2, y: viewport.y + viewport.height - 12 };
  await touch.move(edge);
  const last = await until(async () => {
    const view = await snapshot();
    const row = view.elements.find(element => element.hit && button("Touch 16")(element));
    const source = view.elements.some(button("Touch 05"));
    return row && !source && row.y + row.height <= viewport.y + viewport.height + 1 ? row : false;
  }, "held drag autoscrolls until the source leaves realization and the last row appears", 15000);
  assert.deepEqual(await saved(), beforeScroll, "edge autoscroll cannot commit an intermediate reorder");
  await touch.move({ x: last.x + last.width / 2, y: Math.min(edge.y, last.y + last.height * .8) });
  await capture("touch-drag-autoscroll");
  await touch.end();
  await until(async () => (await ctx.task("Touch 05")).position > (await ctx.task("Touch 16")).position,
    "a single release reorders after autoscroll and source-row recycling");
  const ordered = document => document.tasks.filter(task => task.list_id === ctx.source.id)
    .sort((a, b) => a.position - b.position).map(task => task.id);
  const movedId = (await ctx.task("Touch 05")).id;
  assert.deepEqual(ordered(await saved()), [...ordered(beforeScroll).filter(id => id !== movedId), movedId],
    "the final drop moves only the source and preserves every other relative position");
  await noDetails(ctx, "autoscroll drag release cannot open details");
  await touch.tapControl(button("Exit task selection"), "leave the autoscroll drag selection");
  await resize(390, 844);
  console.log("ok: second-contact cancellation for swipe/drag and a single final drop after edge autoscroll/source recycling");
}
