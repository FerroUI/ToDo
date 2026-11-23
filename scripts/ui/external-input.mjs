import assert from "node:assert/strict";
import { center, named } from "./touch-input.mjs";
import { runPointerStage } from "./input-diagnostics.mjs";

// Real CDP drag data follows the browser and retained-control event pipeline.
export async function dragExternalInput(ui, target, items, { accept = true, hover = false } = {}) {
  const { page, snapshot, until } = ui;
  return runPointerStage(ui, accept ? "external task drop" : "external task cancellation", async state => {
    const point = center(target);
    const data = { items, dragOperationsMask: 1 };
    state.destination = target.name || target.automationName;
    state.formats = items.map(item => item.mimeType);
    for (const type of ["dragEnter", "dragOver"]) {
      state.phase = type;
      const result = await page.send("Input.dispatchDragEvent", { type, ...point, data });
      assert(!result.error, result.error?.message);
    }
    if (hover) {
      state.phase = "hover feedback";
      await until(async () => (await snapshot()).elements.some(named("ExternalTaskDropHint")), "native external-drop destination feedback");
    }
    if (!accept) {
      state.phase = "Escape cancellation";
      await page.press("Escape");
      // Prove cancellation before terminating the browser target. CDP's source-
      // only dragCancel leaves Blink's target behind; the next same-canvas enter
      // then becomes an over. A real late Drop must remain rejected and causes
      // Chromium's NONE -> DragLeave path to end the target lifecycle as well.
      await until(async () => !(await snapshot()).elements.some(named("ExternalTaskDropHint")), "Escape removes external-drop feedback before a late drop");
    }
    state.phase = accept ? "drop" : "late drop after cancellation";
    const result = await page.send("Input.dispatchDragEvent", { type: "drop", ...point, data });
    assert(!result.error, result.error?.message);
    await page.evaluate("new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)))");
  });
}
