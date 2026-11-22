import assert from "node:assert/strict";
import { button, center, noDetails, noRail } from "./touch-input.mjs";

async function hold(ctx, title) {
  await ctx.touch.start(center(await ctx.row(title)));
  // Native Holding owns its 300ms timeout. Wait for its visible result instead of synthesizing it.
  await ctx.control(button("Exit task selection"), "native long press enters task selection");
}

async function dragBefore(ctx, from, to, cancel = false) {
  await hold(ctx, from);
  const target = await ctx.row(to);
  await ctx.touch.move({ x: target.x + target.width / 2, y: target.y + target.height * .2 });
  if (cancel) await ctx.touch.cancel();
  else await ctx.touch.end();
}

export async function verifyTouchDrag(ctx) {
  const { touch, saved, control, until } = ctx;
  const beforeHold = await saved();
  await hold(ctx, "Touch 03");
  await touch.end();
  await control(element => element.type === "TextBlock" && element.text === "1 selected", "single held selection");
  assert.deepEqual(await saved(), beforeHold, "long press selects without changing task data");
  await noDetails(ctx, "long-press release must not also open the task");
  await touch.tapControl(button("Exit task selection"), "leave long-press selection");

  await dragBefore(ctx, "Touch 05", "Touch 03");
  await until(async () => (await ctx.task("Touch 05")).position < (await ctx.task("Touch 03")).position,
    "held touch drag saves manual row order");
  await until(async () => (await ctx.row("Touch 05")).y < (await ctx.row("Touch 03")).y,
    "the rendered rows follow held drag order");
  await noDetails(ctx, "a held drag release cannot open details");
  await touch.tapControl(button("Exit task selection"), "leave drag selection");

  const beforeCancel = await saved();
  await dragBefore(ctx, "Touch 04", "Touch 05", true);
  assert.deepEqual(await saved(), beforeCancel, "touchCancel discards a held drag without a drop");
  await noDetails(ctx, "touchCancel cannot activate the original row");
  await touch.tapControl(button("Exit task selection"), "leave cancelled drag selection");

  await touch.tapControl(button("Sort tasks"), "sort before the drag guard check");
  await touch.tapControl(element => element.type.endsWith("MenuItem") && element.automationName === "Alphabetically", "alphabetical sort");
  await until(async () => (await saved()).lists.find(list => list.id === ctx.source.id).sort === "Alphabetical", "nonmanual sorting is active");
  const sorted = await saved();
  await dragBefore(ctx, "Touch 05", "Touch 03");
  assert.deepEqual(await saved(), sorted, "holding cannot reorder a sorted list");
  await touch.tapControl(button("Exit task selection"), "leave sorted-list selection");
  await touch.tapControl(button("Remove sort"), "restore manual sorting");
  await noRail(ctx);
  console.log("ok: native long press, manual held drag, touch cancellation, and sorted-list drag guard");
}
