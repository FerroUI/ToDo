import assert from "node:assert/strict";
import { center, noDetails } from "./touch-input.mjs";
import { browserBack, dropMarker, noTransient, topTasks } from "./touch-combination-helpers.mjs";

async function moveMouse(ctx, point) {
  const result = await ctx.page.send("Input.dispatchMouseEvent", { type: "mouseMoved", ...point, buttons: 1, pointerType: "mouse" });
  assert(!result.error, result.error?.message);
  await ctx.touch.frame();
}

async function beginMouse(ctx, source, target) {
  const from = center(await ctx.row(source.title)), row = await ctx.row(target.title);
  const drop = { x: center(row).x, y: row.y + row.height * .2 };
  await ctx.page.mouseMove(from.x, from.y);
  await ctx.page.mouseDown(from.x, from.y);
  await moveMouse(ctx, { x: from.x + 12, y: from.y });
  await moveMouse(ctx, { x: from.x + 14, y: from.y });
  await moveMouse(ctx, drop);
  await ctx.until(async () => (await ctx.snapshot()).elements.some(dropMarker), "native mouse drag exposes a valid insertion marker");
  return drop;
}

export async function verifyMixedPointerCancellation(ctx) {
  const { touch, page, saved, until, snapshot } = ctx;
  const tasks = await topTasks(ctx), baseline = await saved();
  for (const interrupt of [async () => {
    await touch.start(center(await ctx.row(tasks[2].title)));
    await touch.end();
  }, () => browserBack(page)]) {
    const drop = await beginMouse(ctx, tasks[1], tasks[0]);
    try {
      await interrupt();
      await noTransient(ctx, "touch or Back invalidates native mouse drag feedback");
      await moveMouse(ctx, { x: drop.x + 2, y: drop.y + 2 });
    } finally {
      await page.mouseUp(drop.x + 2, drop.y + 2);
    }
    await noTransient(ctx, "late mouse release cannot revive a cancelled payload");
    await noDetails(ctx, "neither pointer in a cancelled mixed-input drag activates a row");
    assert.deepEqual(await saved(), baseline, "touch/Back cancellation preserves the complete document");
  }

  // A new native drag must work; ordinary wheel scrolling remains supported during it.
  let drop = await beginMouse(ctx, tasks[1], tasks[0]);
  try {
    await page.wheel(drop.x, drop.y, 0, 40);
    await until(async () => (await snapshot()).elements.some(dropMarker), "wheel input does not cancel the native mouse drag");
    const row = await ctx.row(tasks[3].title);
    drop = { x: center(row).x, y: row.y + row.height * .2 };
    await moveMouse(ctx, drop);
    await until(async () => (await snapshot()).elements.some(dropMarker), "the wheel-adjusted destination remains valid");
  } finally {
    await page.mouseUp(drop.x, drop.y);
  }
  const order = data => data.tasks.filter(task => task.list_id === ctx.source.id)
    .sort((a, b) => a.position - b.position).map(task => task.id);
  const expected = order(baseline).filter(id => id !== tasks[1].id);
  expected.splice(expected.indexOf(tasks[3].id), 0, tasks[1].id);
  await until(async () => JSON.stringify(order(await saved())) === JSON.stringify(expected), "a fresh native drop works after mixed-input cancellation and wheel scrolling");
  const after = await saved();
  assert.deepEqual({ ...after, tasks: baseline.tasks }, baseline, "native reordering changes no non-task document fields");
  assert.deepEqual(after.tasks, baseline.tasks.map(task => task.list_id === ctx.source.id
    ? { ...task, position: after.tasks.find(item => item.id === task.id).position } : task),
  "the recovered drop preserves all task data except the intended list positions");
  await noDetails(ctx, "the successful recovery drop is not also a click");
  await noTransient(ctx, "the completed mouse drop clears its feedback");
  console.log("ok: native mouse drag touch/Back cancellation, inert late release, and fresh wheel-assisted drop recovery");
}
