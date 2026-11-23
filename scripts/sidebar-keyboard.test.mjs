import assert from "node:assert/strict";
import test from "node:test";
import { expectSidebarFocus } from "./ui/sidebar-keyboard.mjs";

const expected = element => element.name === "NavigateList(68)";
const row = { name: "NavigateList(68)", type: "Button", automationName: "Sidebar B",
  focused: true, hit: false, x: 24, y: 777, width: 247, height: 44 };

function harness(rows, { height = 1000 } = {}) {
  return {
    // No input or application mutation API is available: checking keyboard
    // behavior must not fix it by scrolling, clicking, or moving the pointer.
    snapshot: async () => ({ width: 1280, height, elements: [
      { name: "SidebarScroll", type: "ScrollViewer", x: 0, y: 160, width: 279, height: 788 },
      ...rows,
    ] }),
    until: async (check, label) => {
      const value = await check();
      assert(value, label);
      return value;
    },
  };
}

test("Right-to-child verifies native focus despite the tooltip that covers its pointer hit point", async () => {
  // Exact 5157a5b failure geometry: focused Sidebar B is inside y160..948,
  // but the group's native tooltip covers the row's center (147.5,799).
  const tooltip = { name: "ThemeShadowWrapper", type: "Border", x: 147, y: 777, width: 120, height: 33, hit: true };
  const x = row.x + row.width / 2, y = row.y + row.height / 2;
  assert(x >= tooltip.x && x <= tooltip.x + tooltip.width && y >= tooltip.y && y <= tooltip.y + tooltip.height);
  assert.equal(row.hit, false);
  assert.deepEqual(await expectSidebarFocus(harness([row, tooltip]), expected, "Right enters the first group member"), row);
});

test("focus on another member cannot satisfy the expected first-child identity", async () => {
  await assert.rejects(expectSidebarFocus(harness([
    { ...row, focused: false }, { ...row, name: "NavigateList(66)", focused: true, y: 822 },
  ]), expected, "expected first member"), /expected first member/);
});

test("a focused but clipped row remains a real keyboard navigation failure", async () => {
  await assert.rejects(expectSidebarFocus(harness([{ ...row, y: 920 }]), expected, "focused child must be revealed"),
    /focused child must be revealed/);
  await assert.rejects(expectSidebarFocus(harness([row], { height: 800 }), expected, "focused child must fit the canvas"),
    /focused child must fit the canvas/);
});

test("duplicate rendered identities cannot pass a keyboard focus assertion", async () => {
  await assert.rejects(expectSidebarFocus(harness([row, { ...row, y: 822 }]), expected, "unique child"),
    /Ambiguous sidebar focus target: unique child/);
});
