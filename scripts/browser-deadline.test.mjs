import assert from "node:assert/strict";
import test from "node:test";
import { boundPage } from "./ui/deadline.mjs";

test("a stalled hidden-target request leaves the independent browser channel usable for recovery", async () => {
  let rejectLate;
  const calls = [];
  const page = boundPage({
    send: () => new Promise((_, reject) => { rejectLate = reject; }),
    browserSend: async method => { calls.push(method); return { acknowledged: true }; },
  }, 10);
  await assert.rejects(page.send("Input.dispatchTouchEvent", { type: "touchEnd" }),
    /timed out.*send Input\.dispatchTouchEvent \(touchEnd\)/);
  assert.deepEqual(await page.browserSend("Target.activateTarget"), { acknowledged: true });
  assert.deepEqual(calls, ["Target.activateTarget"]);
  // A late transport rejection is already observed by the deadline race.
  rejectLate(new Error("old target closed"));
  await new Promise(resolve => setImmediate(resolve));
});

test("original protocol failures retain their identity and a stalled frame has an explicit diagnostic", async () => {
  const original = new Error("renderer disconnected");
  const page = boundPage({
    send: async () => { throw original; },
    evaluate: () => new Promise(() => {}),
  }, 10);
  await assert.rejects(page.send("Page.reload"), error => error === original);
  await assert.rejects(page.evaluate("new Promise(resolve => requestAnimationFrame(resolve))"),
    /timed out.*evaluate.*requestAnimationFrame/);
});
