import assert from "node:assert/strict";
import { center, noDetails } from "./touch-input.mjs";
import { browserBack, dropMarker, noTransient, topTasks } from "./touch-combination-helpers.mjs";
import { runPointerStage } from "./input-diagnostics.mjs";

async function moveMouse(ctx, point) {
  const result = await ctx.page.send("Input.dispatchMouseEvent", { type: "mouseMoved", ...point, buttons: 1, pointerType: "mouse" });
  assert(!result.error, result.error?.message);
  await ctx.touch.frame();
}

async function beginMouse(ctx, source, target, state) {
  state.source = { id: source.id, title: source.title, list_id: source.list_id,
    flagged_email: source.flagged_email, external: source.external };
  state.target = { id: target.id, title: target.title, list_id: target.list_id,
    flagged_email: target.flagged_email, external: target.external };
  state.phase = "source-and-target";
  state.sourceControl = await ctx.row(source.title);
  state.targetControl = await ctx.row(target.title);
  const from = center(state.sourceControl), row = state.targetControl;
  const drop = { x: center(row).x, y: row.y + row.height * .2 };
  state.phase = "hover-source";
  await ctx.page.mouseMove(from.x, from.y);
  state.phase = "press-source";
  state.mouse = from;
  await ctx.page.mouseDown(from.x, from.y);
  state.phase = "cross-drag-threshold";
  state.mouse = { x: from.x + 12, y: from.y };
  await moveMouse(ctx, state.mouse);
  state.mouse = { x: from.x + 14, y: from.y };
  await moveMouse(ctx, state.mouse);
  state.phase = "move-to-target";
  state.mouse = drop;
  await moveMouse(ctx, drop);
  state.phase = "insertion-marker";
  await ctx.until(async () => (await ctx.snapshot()).elements.some(dropMarker),
    `native mouse drag exposes a valid insertion marker (${state.stage})`);
  console.log(`ok: native insertion marker (${state.stage})`);
  return drop;
}

export async function verifyMixedPointerCancellation(ctx) {
  const { touch, page, saved, until, snapshot } = ctx;
  const tasks = await topTasks(ctx), baseline = await saved();
  for (const [stage, interrupt] of [["touch-cancel", async () => {
    await touch.start(center(await ctx.row(tasks[2].title)));
    await touch.end();
  }], ["back-cancel", () => browserBack(page)]]) {
    await runPointerStage(ctx, stage, async state => {
      const drop = await beginMouse(ctx, tasks[1], tasks[0], state);
      state.phase = "interrupt-drag";
      await interrupt();
      state.phase = "cancelled-feedback";
      await noTransient(ctx, "touch or Back invalidates native mouse drag feedback");
      state.phase = "late-mouse-move";
      state.mouse = { x: drop.x + 2, y: drop.y + 2 };
      await moveMouse(ctx, state.mouse);
    });
    await noTransient(ctx, "late mouse release cannot revive a cancelled payload");
    await noDetails(ctx, "neither pointer in a cancelled mixed-input drag activates a row");
    assert.deepEqual(await saved(), baseline, "touch/Back cancellation preserves the complete document");
  }

  // A new native drag must work; ordinary wheel scrolling remains supported during it.
  await runPointerStage(ctx, "recovery", async state => {
    let drop = await beginMouse(ctx, tasks[1], tasks[0], state);
    state.phase = "wheel-during-drag";
    await page.wheel(drop.x, drop.y, 0, 40);
    await until(async () => (await snapshot()).elements.some(dropMarker), "wheel input does not cancel the native mouse drag");
    state.phase = "wheel-adjusted-target";
    const row = await ctx.row(tasks[3].title);
    state.target = { id: tasks[3].id, title: tasks[3].title, list_id: tasks[3].list_id };
    state.targetControl = row;
    drop = { x: center(row).x, y: row.y + row.height * .2 };
    state.mouse = drop;
    await moveMouse(ctx, drop);
    await until(async () => (await snapshot()).elements.some(dropMarker), "the wheel-adjusted destination remains valid");
  });
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
