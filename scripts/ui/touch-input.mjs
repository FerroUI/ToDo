import assert from "node:assert/strict";

export const button = label => element => element.type.endsWith("Button") && element.automationName === label;
export const named = name => element => element.name === name;
export const center = element => ({ x: element.x + element.width / 2, y: element.y + element.height / 2 });

// Real contacts enter Chromium's pointer pipeline; no application command substitutes for a gesture.
export function touchInput(ui) {
  const { page, control } = ui;
  let contact = null;
  const frame = () => page.evaluate("new Promise(resolve => requestAnimationFrame(resolve))");
  async function send(type, point) {
    const result = await page.send("Input.dispatchTouchEvent", {
      type,
      touchPoints: point ? [{ ...point, id: 1, radiusX: 4, radiusY: 4, force: 1 }] : [],
    });
    assert(!result.error, result.error?.message);
  }
  async function start(point) {
    assert.equal(contact, null, "the previous touch must end before another contact starts");
    contact = point;
    await send("touchStart", point);
  }
  async function move(to, steps = 6) {
    assert(contact, "touch movement requires a live contact");
    const from = contact;
    for (let step = 1; step <= steps; step++) {
      contact = { x: from.x + (to.x - from.x) * step / steps, y: from.y + (to.y - from.y) * step / steps };
      await send("touchMove", contact);
      await frame();
    }
  }
  async function finish(cancel = false) {
    if (!contact) return;
    contact = null;
    await send(cancel ? "touchCancel" : "touchEnd");
    await frame();
    await frame();
  }
  async function tap(point) {
    await start(point);
    await finish();
  }
  async function tapControl(match, label) {
    const target = await control(match, label);
    await tap(center(target));
    return target;
  }
  async function swipe(from, to, cancel = false) {
    await start(from);
    await move(to);
    await finish(cancel);
  }
  return { start, move, end: () => finish(), cancel: () => finish(true), tap, tapControl, swipe, frame };
}

export async function noDetails(ui, label) {
  assert(!(await ui.snapshot()).elements.some(named("TaskTitle")), label);
}

export async function noRail(ui) {
  assert(!(await ui.snapshot()).elements.some(element => element.hit && /^Swipe(?:Delete|MyDay|Move)$/.test(element.name || "")),
    "the gesture must not leave a task action rail open");
}
