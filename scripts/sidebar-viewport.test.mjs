import assert from "node:assert/strict";
import test from "node:test";
import { revealSidebarControl, revealSidebarControls, sidebarViewport } from "./ui/sidebar-viewport.mjs";

const named = name => element => element.name === name;

function harness({ width = 390, height = 844, top = 200, bottom = 792, offset = 0, rows, broadHit = false }) {
  const clip = { x: 0, y: top, width: 279, height: bottom - top };
  const wheel = [], swipes = [];
  const inside = row => row.y >= top && row.y + row.height <= bottom;
  const snapshot = () => ({
    width, height,
    elements: [
      { name: "SidebarScroll", type: "ScrollViewer", ...clip, hit: true },
      { name: "ListNavigation", type: "StackPanel", x: 8, y: top + 4 - offset, width: 263, height: 2000, hit: true },
      ...rows.map(row => ({ type: "Button", x: 8, width: 263, height: 44, ...row, y: row.y - offset }))
        .filter(row => row.y < height && row.y + row.height > 0)
        .map(row => ({ ...row, hit: broadHit || inside(row) })),
    ],
  });
  const checkPoint = ({ x, y }) => assert(x > clip.x && x < clip.x + clip.width
    && y > top && y < bottom, "scroll input stays inside the actual navigation viewport");
  const scroll = distance => { offset = Math.max(0, Math.min(1400, offset + distance)); };
  return {
    wheel, swipes, snapshot: async () => snapshot(),
    page: {
      wheel: async (x, y, dx, dy) => {
        checkPoint({ x, y }); assert.equal(dx, 0);
        wheel.push(dy); scroll(dy);
      },
      evaluate: async script => {
        assert(script.includes("requestAnimationFrame"));
        assert(!script.includes("todoApp") && !script.includes("localStorage"), "reveal only observes native layout");
      },
    },
    touch: {
      swipe: async (from, to) => {
        checkPoint(from); checkPoint(to); assert.equal(from.x, to.x);
        swipes.push({ from, to }); scroll(from.y - to.y);
      },
    },
  };
}

test("touch drawer target behind the fixed footer is revealed with actual finger scrolling", async () => {
  // ff96: destination y815..859, native navigation clip y200..792.
  const ui = harness({ rows: [{ name: "NavigateList(43)", y: 815 }] });
  const row = await revealSidebarControl(ui, named("NavigateList(43)"), "touch destination", { touch: ui.touch });
  assert.equal(ui.wheel.length, 0, "touch preparation never substitutes a mouse wheel");
  assert(ui.swipes.length > 0);
  assert(ui.swipes[0].to.y < ui.swipes[0].from.y, "an upward finger swipe reveals lower rows");
  assert(row.hit && row.y >= 200 && row.y + row.height <= 792);
  assert.deepEqual(row, (await ui.snapshot()).elements.find(named(row.name)), "returned bounds are reacquired after scrolling");
});

test("desktop recovery finds an entirely omitted touch fixture through ordinary scrolling", async () => {
  // ff96: 1280x800 recovery has clip y160..748; source is below the canvas.
  const ui = harness({ width: 1280, height: 800, top: 160, bottom: 748,
    rows: [{ name: "NavigateList(44)", y: 820 }] });
  assert(!(await ui.snapshot()).elements.some(named("NavigateList(44)")));
  const row = await revealSidebarControl(ui, named("NavigateList(44)"), "saved touch fixture");
  assert(ui.wheel.length > 0 && ui.wheel[0] > 0);
  assert(row.hit && row.y + row.height <= 748);
});

test("empty-group guidance below its scroll clip remains invalid even with a broad hit flag", async () => {
  // ff96: guidance y949..987 is behind the footer at y948 on a 1000px canvas.
  const ui = harness({ width: 1280, height: 1000, top: 160, bottom: 948,
    rows: [{ name: "EmptyGroup64", x: 30, y: 949, width: 233, height: 38 }], broadHit: true });
  const row = await revealSidebarControl(ui, named("EmptyGroup64"), "empty-group drop guidance");
  assert(ui.wheel.length > 0 && ui.wheel[0] > 0);
  assert(row.y + row.height <= 948);
});

test("drag preparation returns source and target from the same fully visible snapshot", async () => {
  const ui = harness({ rows: [{ name: "source", y: 600 }, { name: "target", y: 815 }] });
  const rows = await revealSidebarControls(ui, [named("source"), named("target")], "drag endpoints");
  const view = await ui.snapshot();
  for (const row of rows) {
    assert(row.hit && row.y >= 200 && row.y + row.height <= 792);
    assert.deepEqual(row, view.elements.find(named(row.name)));
  }
});

test("an already visible target requires no cleanup input after a rejected gesture", async () => {
  const ui = harness({ rows: [{ name: "target", y: 400 }] });
  await revealSidebarControl(ui, named("target"), "immediate recovery tap", { touch: ui.touch });
  assert.deepEqual(ui.wheel, []);
  assert.deepEqual(ui.swipes, []);
});

test("an omitted target above the viewport is found by scrolling toward the content origin", async () => {
  const ui = harness({ offset: 600, rows: [{ name: "target", y: 400 }] });
  const row = await revealSidebarControl(ui, named("target"), "earlier list");
  assert(ui.wheel[0] < 0);
  assert(row.hit && row.y >= 200);
});

test("sidebar input bounds are also clipped by the canvas", () => {
  const view = { width: 200, height: 400, elements: [
    { name: "SidebarScroll", x: 0, y: 160, width: 279, height: 588 },
  ] };
  assert.deepEqual(sidebarViewport(view), { x: 0, y: 160, width: 200, height: 240 });
});
