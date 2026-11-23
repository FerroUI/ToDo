import assert from "node:assert/strict";
import { button, center, noDetails } from "./touch-input.mjs";
import { dropMarker, endSelection, noTransient, topTasks } from "./touch-combination-helpers.mjs";
import { prepareTabInterruption } from "./tab-interruption.mjs";
import { runPointerStage } from "./input-diagnostics.mjs";

// CDP pen events go through Chromium's PointerEvent/PenDevice pipeline. No
// application command substitutes for a press, hold, move or release.
export async function verifyPenCancellation(ctx) {
  const { page, touch, saved, snapshot, control, until, capture } = ctx;
  const tabs = await prepareTabInterruption(ctx);
  let down = false, last = { x: 0, y: 0 };
  const send = async (type, point = last) => {
    last = point;
    const result = await page.send("Input.dispatchMouseEvent", {
      type, ...point, pointerType: "pen", button: type === "mouseMoved" ? "none" : "left",
      buttons: type === "mouseReleased" ? 0 : 1, clickCount: type === "mouseMoved" ? 0 : 1,
      force: type === "mouseReleased" ? 0 : .5, tiltX: 10, tiltY: 20, twist: 30,
    });
    assert(!result.error, result.error?.message);
    down = type !== "mouseReleased";
    await touch.frame();
  };
  const penTap = async point => { await send("mousePressed", point); await send("mouseReleased"); };
  let failure;
  try {
    await runPointerStage(ctx, "pen cancellation and tab interruption", async () => {
      const tasks = await topTasks(ctx), baseline = await saved();
      await penTap(center(await ctx.row(tasks[0].title)));
      await control(element => element.name === "TaskTitle", "pen opens a task normally");
      await touch.tapControl(button("Close details"), "close pen-opened details");
      for (const penFirst of [true, false]) {
        const first = center(await ctx.row(tasks[0].title)), second = center(await ctx.row(tasks[1].title));
        if (penFirst) { await send("mousePressed", first); await touch.start(second); }
        else { await touch.start(first); await send("mousePressed", second); }
        await send("mouseReleased"); await touch.end();
        await noDetails(ctx, "mixed pen/finger releases do not activate either task");
        await noTransient(ctx, "mixed pen/finger contact clears feedback");
        assert.deepEqual(await saved(), baseline, "mixed-device cancellation keeps every saved field");
      }
      await send("mousePressed", center(await ctx.row(tasks[1].title)));
      await control(button("Exit task selection"), "native pen Holding enters selection");
      const row = await ctx.row(tasks[0].title), point = { x: center(row).x, y: row.y + row.height * .2 };
      await send("mouseMoved", { x: last.x + 18, y: last.y });
      await send("mouseMoved", point);
      await until(async () => (await snapshot()).elements.some(dropMarker), "held pen displays its native insertion marker");
      await capture("pen-held-drag");
      await page.press("Escape");
      await send("mouseMoved", { x: point.x + 3, y: point.y + 2 });
      await send("mouseReleased");
      await noTransient(ctx, "cancelled pen cannot revive its drag after Escape");
      assert.deepEqual(await saved(), baseline, "late pen release never commits the cancelled drop");
      await endSelection(ctx);

      // A native Button owns capture without an app row recognizer. Tab blur
      // must retire that pen too, otherwise the late Up can click the button.
      await send("mousePressed", center(await control(button("List options"), "list options before pen interruption")));
      await tabs.deactivate("pen tab actually loses activation");
      await tabs.restore("pen tab returns to the foreground");
      await send("mouseReleased");
      assert(!(await snapshot()).elements.some(element => element.hit && element.type.endsWith("MenuItem")), "a cancelled native button does not click on pen release");
      await penTap(center(await control(button("List options"), "list options after cancellation")));
      await until(async () => (await snapshot()).elements.some(element => element.hit && element.type.endsWith("MenuItem")), "first fresh pen opens the native menu");
      await page.press("Escape");
      assert.deepEqual(await saved(), baseline);
      console.log("ok: native pen tap/hold/drag, mixed pen/finger cancellation, Escape, tab blur, and first fresh pen recovery");
    });
  } catch (error) {
    failure = error;
  } finally {
    for (const cleanup of [() => tabs.restore(), async () => { if (down) await send("mouseReleased"); },
      () => touch.cancel(), () => tabs.close()]) {
      try { await cleanup(); } catch (error) { failure ??= error; }
    }
  }
  if (failure) throw failure;
}
