import assert from "node:assert/strict";
import test from "node:test";
import { dragControls } from "./ui/mouse-drag.mjs";

const named = name => element => element.name === name;
const row = (name, x, y, width = 820, height = 54) => ({ name, type: "Button", x, y, width, height, hit: true });

function harness(views) {
  const events = [];
  let reads = 0;
  return {
    events,
    inputTrace: async () => ({ events: events.slice() }),
    snapshot: async () => ({ width: 1280, height: 800, elements: views[Math.min(reads++, views.length - 1)] }),
    until: async (check, label) => {
      for (let turn = 0; turn < views.length; turn++) {
        const result = await check();
        if (result) return result;
      }
      throw new Error(label);
    },
    page: {
      mouseMove: async (x, y) => events.push({ action: "hover", x, y }),
      mouseDown: async (x, y) => events.push({ action: "press", x, y }),
      mouseUp: async (x, y) => events.push({ action: "release", x, y }),
      send: async (method, event) => events.push({ action: method, ...event }),
      wheel: async () => assert.fail("A prepared task drag must not scroll the unrelated sidebar"),
    },
  };
}

test("bulk task reorder targets the task viewport rather than navigation", async () => {
  // Captured 348bdd failure: SidebarScroll x0..279, both task rows x388..1208.
  const ui = harness([[row("Bulk B", 388, 290), row("Bulk D", 388, 346)]]);
  await dragControls(ui, named("Bulk B"), named("Bulk D"), { fraction: .8 });
  assert.deepEqual(ui.events.find(event => event.action === "press"), { action: "press", x: 798, y: 317 });
  assert.deepEqual(ui.events.at(-1), { action: "release", x: 798, y: 389.2 });
  assert(ui.events.some(event => event.type === "mouseMoved" && event.buttons === 1 && event.pointerType === "mouse"));
});

test("task-to-list cancellation delivers Escape before the real pointer release", async () => {
  const ui = harness([[row("task", 388, 178), row("list", 8, 595, 263, 44)]]);
  await dragControls(ui, named("task"), named("list"), { cancel: true });
  const escape = ui.events.findIndex(event => event.type === "rawKeyDown" && event.key === "Escape");
  const release = ui.events.findIndex(event => event.action === "release");
  assert(escape > 0 && escape < release);
  assert.deepEqual(ui.events[release], { action: "release", x: 139.5, y: 617 });
  assert.equal(ui.events.at(-1).type, "keyUp");
});

test("drag endpoints are reacquired together after layout and never taken from stale bounds", async () => {
  const ui = harness([
    [row("source", 388, 290), { ...row("target", 388, 346), hit: false }],
    [row("source", 388, 234), row("target", 388, 290)],
  ]);
  await dragControls(ui, named("source"), named("target"));
  assert.deepEqual(ui.events.find(event => event.action === "press"), { action: "press", x: 798, y: 261 });
  assert.deepEqual(ui.events.at(-1), { action: "release", x: 798, y: 317 });
});

test("a partially offscreen endpoint cannot begin a drag despite a broad hit flag", async () => {
  const ui = harness([[row("source", 388, 290), row("target", 388, 780)]]);
  await assert.rejects(dragControls(ui, named("source"), named("target")), /visible mouse-drag source and destination/);
  assert.deepEqual(ui.events, []);
});

test("a protocol rejection keeps its original cause and releases the pressed pointer", async () => {
  const ui = harness([[row("source", 388, 290), row("target", 388, 346)]]);
  ui.page.send = async () => ({ error: { message: "Original pointer protocol rejection" } });
  await assert.rejects(dragControls(ui, named("source"), named("target")), error => {
    assert.match(error.message, /Original pointer protocol rejection/);
    assert.equal(error.inputDiagnostics.trace.events.some(event => event.action === "release"), false,
      "the original pressed state is observed before cleanup");
    return true;
  });
  assert.equal(ui.events.at(-1).action, "release");
});
