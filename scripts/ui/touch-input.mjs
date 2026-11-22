import assert from "node:assert/strict";

export const button = label => element => element.type.endsWith("Button") && element.automationName === label;
export const named = name => element => element.name === name;
export const center = element => ({ x: element.x + element.width / 2, y: element.y + element.height / 2 });

// Real contacts enter Chromium's pointer pipeline; no application command substitutes for a gesture.
export function touchInput(ui) {
  const { page, control } = ui;
  const contacts = new Map();
  let primary = null;
  const frame = () => page.evaluate("new Promise(resolve => requestAnimationFrame(resolve))");
  async function send(type) {
    const result = await page.send("Input.dispatchTouchEvent", {
      type,
      touchPoints: [...contacts].map(([id, position]) => ({ ...position, id, radiusX: 4, radiusY: 4, force: 1 })),
    });
    assert(!result.error, result.error?.message);
  }
  async function startContacts(points) {
    assert.equal(contacts.size, 0, "the previous touch must end before another contact starts");
    assert(points.length >= 1 && points.length <= 2, "the device supports one or two contacts");
    points.forEach((point, index) => contacts.set(index + 1, point));
    primary = 1;
    await send("touchStart");
  }
  const start = point => startContacts([point]);
  async function moveContact(id, to, steps = 6) {
    const from = contacts.get(id);
    assert(from, "touch movement requires a live contact with the same identifier");
    for (let step = 1; step <= steps; step++) {
      contacts.set(id, { x: from.x + (to.x - from.x) * step / steps, y: from.y + (to.y - from.y) * step / steps });
      await send("touchMove");
      await frame();
    }
  }
  const move = (to, steps = 6) => moveContact(primary, to, steps);
  async function addContact(point) {
    assert.equal(contacts.size, 1, "a second finger requires exactly one active contact");
    const id = contacts.has(1) ? 2 : 1;
    contacts.set(id, point);
    await send("touchStart");
    await frame();
    return id;
  }
  async function releaseContact(id) {
    assert(contacts.delete(id), "only a live contact can be released");
    if (primary === id) primary = contacts.keys().next().value ?? null;
    // CDP touchEnd ends every contact. touchMove reconciles the remaining active
    // points, generating a release for the removed ID without renumbering survivors.
    await send(contacts.size ? "touchMove" : "touchEnd");
    await frame();
  }
  async function finish(cancel = false, { settle = true } = {}) {
    if (!contacts.size) return;
    contacts.clear();
    primary = null;
    await send(cancel ? "touchCancel" : "touchEnd");
    if (settle) {
      await frame();
      await frame();
    }
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
  return { start, startContacts, move, moveContact, addContact, releaseContact,
    end: options => finish(false, options), cancel: options => finish(true, options), tap, tapControl, swipe, frame };
}

export async function noDetails(ui, label) {
  assert(!(await ui.snapshot()).elements.some(named("TaskTitle")), label);
}

export async function noRail(ui) {
  assert(!(await ui.snapshot()).elements.some(element => element.hit && /^Swipe(?:Delete|MyDay|Move)$/.test(element.name || "")),
    "the gesture must not leave a task action rail open");
}
