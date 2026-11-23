import assert from "node:assert/strict";
import test from "node:test";
import { followSidebarTarget } from "./ui/touch-sidebar-input.mjs";

test("held sidebar movement follows a target shifted by native scrolling before release", async () => {
  const moves = [];
  let target = { x: 8, y: 908, width: 231, height: 40 };
  const ctx = {
    control: async () => ({ ...target }),
    touch: { move: async point => {
      moves.push(point);
      if (moves.length === 1) target.y -= 24;
    } },
  };
  await followSidebarTarget(ctx, () => true, "group center");
  assert.equal(moves.length, 2, "a moved target requires a new real pointer movement");
  assert.deepEqual(moves[0], { x: 123.5, y: 928 });
  assert.deepEqual(moves[1], { x: 123.5, y: 904 });
  assert(moves[0].y > target.y + target.height, "the old coordinate would drop outside the group");
  assert.equal(moves.at(-1).y, target.y + target.height / 2, "the final contact is still at the group center");
});

test("group reorder preserves the requested before-row position after layout changes", async () => {
  const moves = [];
  let y = 400;
  await followSidebarTarget({
    control: async () => ({ x: 8, y, width: 263, height: 44 }),
    touch: { move: async point => { moves.push(point); if (moves.length === 1) y -= 12; } },
  }, () => true, "preceding list", .15);
  assert.deepEqual(moves.at(-1), { x: 139.5, y: 394.6 });
  assert(moves.at(-1).y < y + 22, "reacquisition must not turn Before into After");
});

test("a continuously moving target fails without releasing at an unrelated coordinate", async () => {
  let moves = 0, y = 500;
  await assert.rejects(followSidebarTarget({
    control: async () => ({ x: 8, y, width: 263, height: 44 }),
    touch: { move: async () => { moves++; y -= 12; } },
  }, () => true, "moving group"), /did not settle beneath the finger/);
  assert.equal(moves, 12);
});
