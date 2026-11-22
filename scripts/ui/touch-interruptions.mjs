import assert from "node:assert/strict";
import { button, center, named, noDetails } from "./touch-input.mjs";
import { browserBack, endSelection, heldDrag, noTransient, recoverTap, topTasks } from "./touch-combination-helpers.mjs";

export async function verifyGestureCancellation(ctx) {
  const { touch, page, saved, control, resize, capture } = ctx;
  const tasks = await topTasks(ctx), baseline = await saved();
  for (const interrupt of [() => page.press("Escape"), () => browserBack(page)]) {
    const point = center(await ctx.row(tasks[0].title));
    await touch.start(point);
    await interrupt();
    await touch.move({ x: point.x + 1, y: point.y }, 1);
    await touch.end();
    await noTransient(ctx, "explicit cancellation clears an unmoved contact");
    await noDetails(ctx, "late release after cancelling a pending press cannot click the task");
    assert.deepEqual(await saved(), baseline, "cancelling before slop preserves all task data");
    await recoverTap(ctx, tasks[0]);
  }

  const drop = await heldDrag(ctx, tasks[1], tasks[0]);
  await browserBack(page);
  await noTransient(ctx, "Back cancels the active held drag");
  await control(button("Exit task selection"), "the first Back preserves the selection surface");
  await touch.move({ x: drop.x + 3, y: drop.y + 3 });
  await touch.end();
  await noDetails(ctx, "late held-drag release cannot open its source");
  assert.deepEqual(await saved(), baseline, "Back and late release cannot commit a held drop");
  await capture("touch-drag-back-cancel");
  await browserBack(page);
  await control(named("NewTask"), "a subsequent Back exits selection normally");

  await topTasks(ctx);
  const resizedDrop = await heldDrag(ctx, tasks[1], tasks[0]);
  await resize(390, 700);
  await noTransient(ctx, "height-only resize cancels held drag feedback");
  await touch.move({ x: resizedDrop.x + 2, y: resizedDrop.y + 2 });
  await touch.end();
  assert.deepEqual(await saved(), baseline, "resize and a stale release cannot commit a drop");
  await endSelection(ctx);
  await resize(390, 844);
  await recoverTap(ctx, tasks[0]);
  console.log("ok: pending press Escape/Back, held drag Back/resize, late-release suppression and fresh-input recovery");
}

export async function verifyRefreshInterruption(ctx) {
  const { touch, page, saved, snapshot, control, edit, until, resize, capture } = ctx;
  const tasks = await topTasks(ctx), baseline = await saved();
  const draft = (await control(named("NewTask"), "draft before interrupted pulls")).text || "";
  const origin = await page.evaluate("performance.timeOrigin");
  await edit("NewTask", "Interrupted pull draft");
  const startPull = async () => {
    const point = center(await ctx.row(tasks[0].title));
    await touch.start(point);
    await touch.move({ x: point.x, y: point.y + 110 });
    await until(async () => (await snapshot()).elements.some(element => element.name === "TouchGestureMessage"
      && element.text?.includes("Release to refresh")), "a real pull reaches its release threshold");
    return point;
  };
  for (const interrupt of [() => browserBack(page), () => resize(390, 700)]) {
    const point = await startPull();
    await interrupt();
    await noTransient(ctx, "Back or resize cancels an active pull");
    await touch.move({ x: point.x, y: point.y + 135 });
    await touch.end();
    await noTransient(ctx, "late movement cannot restart an interrupted pull");
    assert.equal((await control(named("NewTask"), "draft after interrupted pull")).text, "Interrupted pull draft");
    assert.deepEqual(await saved(), baseline, "pull interruption preserves the document");
    await resize(390, 844);
  }
  const reverse = await startPull();
  await touch.move({ x: reverse.x, y: reverse.y - 30 });
  assert(!(await snapshot()).elements.some(element => element.name === "TouchGestureMessage"
    && element.text?.includes("Release to refresh")), "reversing past the origin disarms pull refresh");
  await touch.end();
  await noTransient(ctx, "a reversed pull releases without feedback");
  await startPull();
  await capture("touch-refresh-recovery");
  await touch.end();
  await noTransient(ctx, "a fresh full pull completes after interruptions");
  assert.equal((await control(named("NewTask"), "retained recovered draft")).text, "Interrupted pull draft");
  assert.equal(await page.evaluate("performance.timeOrigin"), origin, "combined refresh gestures retain the mounted app");
  assert.deepEqual(await saved(), baseline);
  await edit("NewTask", draft);
  console.log("ok: pull Back/resize/reversal and repeated full-pull recovery preserve document and draft");
}
