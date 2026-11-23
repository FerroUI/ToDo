import assert from "node:assert/strict";
import { sleep } from "../../FerroUI/scripts/browser/harness.mjs";
import { runPointerStage } from "./input-diagnostics.mjs";

// Task rows and navigation entries occupy different native scroll viewers. A
// caller reveals its own targets before this driver acquires both from one
// rendered snapshot; dragging must never scroll an unrelated panel to find them.
export async function dragControls(ui, from, to, { fraction = 0.5, cancel = false } = {}) {
  assert(fraction > 0 && fraction < 1, "The drop point must be inside the target");
  const [source, target] = await ui.until(async () => {
    const view = await ui.snapshot();
    const rows = [from, to].map(match => {
      const found = view.elements.filter(match);
      assert(found.length <= 1, "Ambiguous mouse-drag endpoint");
      return found[0];
    });
    return rows.every(row => row?.hit && row.x >= 0 && row.y >= 0
      && row.x + row.width <= view.width && row.y + row.height <= view.height) && rows;
  }, "visible mouse-drag source and destination");
  const x = source.x + source.width / 2, y = source.y + source.height / 2;
  const tx = target.x + target.width / 2, ty = target.y + target.height * fraction;
  const send = async (method, event) => {
    const result = await ui.page.send(method, event);
    assert(!result?.error, result?.error?.message);
  };
  const escape = type => send("Input.dispatchKeyEvent", { type, key: "Escape", code: "Escape", windowsVirtualKeyCode: 27 });
  await runPointerStage(ui, "mouse control drag", async state => {
    state.source = source; state.target = target;
    const move = async (x, y) => {
      state.mouse = { x, y };
      await send("Input.dispatchMouseEvent", { type: "mouseMoved", x, y, buttons: 1, pointerType: "mouse" });
    };
    await ui.page.mouseMove(x, y);
    state.mouse = { x, y };
    await ui.page.mouseDown(x, y);
    await move(x + 12, y);
    await move(x + 14, y);
    await move(tx, ty);
    if (cancel) { await escape("rawKeyDown"); await move(tx + 1, ty); }
    await ui.page.mouseUp(tx, ty);
    state.mouse = null;
    if (cancel) await escape("keyUp");
  });
  await sleep(100);
}
