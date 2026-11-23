import assert from "node:assert/strict";
import test from "node:test";
import { runInNewContext } from "node:vm";
import { inputTraceScript, runPointerStage } from "./ui/input-diagnostics.mjs";

function fixture(t) {
  t.mock.method(console, "log", () => {});
  const calls = [], trace = { events: [{ type: "pointerdown", buttons: 1 }] };
  const snapshot = { elements: [{ name: "TaskList" }] };
  const h = { calls, trace, snapshot, released: false };
  h.ctx = {
    inputTrace: async () => {
      calls.push("trace"); assert.equal(h.released, false);
      if (h.traceError) throw h.traceError;
      return trace;
    },
    snapshot: async () => {
      calls.push("snapshot"); assert.equal(h.released, false);
      if (h.snapshotError) throw h.snapshotError;
      return snapshot;
    },
    page: { mouseUp: async (x, y) => {
      calls.push(["release", x, y]); h.released = true;
      if (h.cleanupError) throw h.cleanupError;
    } },
  };
  return h;
}

test("a failing pointer stage captures both observations before release and retains the original error", async t => {
  const h = fixture(t), original = new assert.AssertionError({ message: "Insertion marker missing" });
  const source = { id: 23 }, target = { id: 22 };
  await assert.rejects(runPointerStage(h.ctx, "mouse-drag", state => {
    assert.equal(state.stage, "mouse-drag"); assert.equal(state.phase, "begin");
    assert.equal(state.mouse, undefined);
    Object.assign(state, { phase: "await-marker", mouse: { x: 196, y: 189 }, source, target });
    throw original;
  }), error => error === original);
  assert.deepEqual(h.calls, ["trace", "snapshot", ["release", 196, 189]]);
  assert.deepEqual(original.inputDiagnostics, {
    stage: "mouse-drag", phase: "await-marker", mouse: { x: 196, y: 189 }, source, target,
    trace: h.trace, snapshot: h.snapshot,
  });
});

test("independent diagnostic failures cannot hide the primary assertion or skip release", async t => {
  for (const failures of [["trace"], ["snapshot"], ["trace", "snapshot"]]) {
    const h = fixture(t), original = new Error("Original drag failure");
    for (const name of failures) h[`${name}Error`] = new Error(`${name} unavailable`);
    await assert.rejects(runPointerStage(h.ctx, "diagnostic-failure", async state => {
      state.mouse = { x: 10, y: 20 }; state.phase = "pressed";
      throw original;
    }), error => error === original);
    assert.deepEqual(h.calls, ["trace", "snapshot", ["release", 10, 20]]);
    for (const name of ["trace", "snapshot"]) {
      if (failures.includes(name)) assert.match(original.inputDiagnostics[`${name}Error`], new RegExp(`${name} unavailable`));
      else assert.equal(original.inputDiagnostics[name], h[name]);
    }
  }
});

test("release failure is recorded without replacing an existing operation or diagnostic failure", async t => {
  const h = fixture(t), original = new Error("Original marker assertion");
  h.cleanupError = new Error("Mouse release rejected"); h.traceError = new Error("Trace unavailable");
  await assert.rejects(runPointerStage(h.ctx, "release-failure", state => {
    state.mouse = { x: 31, y: 47 }; throw original;
  }), error => error === original);
  assert.match(original.inputDiagnostics.cleanupError, /Mouse release rejected/);
  assert.match(original.inputDiagnostics.traceError, /Trace unavailable/);
  assert.equal(original.inputDiagnostics.snapshot, h.snapshot);
  assert.deepEqual(h.calls.at(-1), ["release", 31, 47]);
});

test("a successful pointer stage awaits release at the latest coordinates and returns its result", async t => {
  const h = fixture(t), release = Promise.withResolvers(), started = Promise.withResolvers();
  h.ctx.page.mouseUp = async (x, y) => { h.calls.push(["release", x, y]); started.resolve(); await release.promise; };
  const expected = { dropped: true };
  let completed = false;
  const pending = runPointerStage(h.ctx, "successful-drop", async state => {
    state.mouse = { x: 5, y: 6 };
    await Promise.resolve(); state.mouse = { x: 90, y: 110 };
    return expected;
  }).then(result => { completed = true; return result; });
  await started.promise;
  assert.equal(completed, false, "stage completion must include mouse release");
  assert.deepEqual(h.calls, [["release", 90, 110]], "successful operations do not collect failure evidence");
  release.resolve(); assert.equal(await pending, expected);
});

test("release failure remains a real failure when the operation itself succeeded", async t => {
  const h = fixture(t); h.cleanupError = new Error("Mouse remained pressed");
  await assert.rejects(runPointerStage(h.ctx, "cleanup-only-failure", state => {
    state.mouse = { x: 3, y: 4 }; return "done";
  }), error => error === h.cleanupError);
  assert.deepEqual(h.calls, [["release", 3, 4]]);
});

test("failure before a press records evidence without synthesizing a mouse release", async t => {
  const h = fixture(t), original = new Error("Source unavailable");
  await assert.rejects(runPointerStage(h.ctx, "source-lookup", () => { throw original; }), error => error === original);
  assert.deepEqual(h.calls, ["trace", "snapshot"]);
  assert.equal(original.inputDiagnostics.phase, "begin");
  h.calls.length = 0;
  assert.equal(await runPointerStage(h.ctx, "no-pointer", () => 42), 42);
  assert.deepEqual(h.calls, []);
});

test("the serialized browser installer observes input passively and retains bounded ordered evidence", () => {
  const listeners = new Map(); let time = 0;
  const canvas = { tagName: "CANVAS", id: "view", hasPointerCapture: id => id === 42 };
  const document = { activeElement: canvas, visibilityState: "visible", hasFocus: () => true };
  const browser = { document, performance: { now: () => ++time },
    addEventListener: (type, callback, options) => {
      assert.equal(options.capture, true); assert.equal(options.passive, true);
      assert.equal(listeners.has(type), false); listeners.set(type, callback);
    } };
  runInNewContext(inputTraceScript, browser);
  for (const type of ["pointerdown", "pointermove", "pointerup", "pointercancel", "gotpointercapture", "lostpointercapture",
    "mousedown", "mousemove", "mouseup", "focus", "blur", "visibilitychange", "popstate", "wheel",
    "dragenter", "dragover", "dragleave", "drop", "dragend"]) {
    assert.equal(typeof listeners.get(type), "function", `${type} is observable`);
  }
  for (let index = 0; index < 520; index++) {
    listeners.get("pointermove")({ target: canvas, isTrusted: true, pointerId: 42, pointerType: "mouse", isPrimary: true,
      button: -1, buttons: 1, clientX: index, clientY: 19, pressure: .5, ctrlKey: false,
      preventDefault() { assert.fail("diagnostics must not cancel input"); },
      stopPropagation() { assert.fail("diagnostics must not change routing"); } });
  }
  const trace = browser.__todoInputTrace.read();
  assert.equal(trace.total, 520); assert.equal(trace.dropped, 8); assert.equal(trace.events.length, 512);
  assert.equal(trace.events[0].sequence, 9); assert.equal(trace.events.at(-1).sequence, 520);
  assert.equal(trace.events.at(-1).clientX, 519); assert.equal(trace.events.at(-1).captured, true);
  assert.equal(trace.events.at(-1).target, "canvas#view"); assert.equal(trace.events.at(-1).buttons, 1);
  assert.equal(trace.events.at(-1).pointerType, "mouse"); assert.equal(trace.events.at(-1).trusted, true);
  assert.equal(trace.events.at(-1).time, 520);
  trace.events.length = 0;
  assert.equal(browser.__todoInputTrace.read().events.length, 512, "reading the trace does not expose the retained event array");
  document.hasFocus = () => false; document.visibilityState = "hidden";
  document.activeElement = { tagName: "INPUT", id: "editor" };
  listeners.get("visibilitychange")({ target: document, isTrusted: true });
  const hidden = browser.__todoInputTrace.read();
  assert.equal(hidden.focus, false); assert.equal(hidden.visibility, "hidden"); assert.equal(hidden.activeElement, "input#editor");
  assert.equal(hidden.total, 521); assert.equal(hidden.dropped, 9);
  assert.equal(hidden.events.at(-1).type, "visibilitychange"); assert.equal(hidden.events.at(-1).visibility, "hidden");
  listeners.get("dragover")({ type: "dragover", target: canvas, isTrusted: true, clientX: 780, clientY: 440,
    get dataTransfer() { assert.fail("diagnostics must not inspect or materialize the native transfer"); },
    preventDefault() { assert.fail("diagnostics must not accept a drop"); },
    stopPropagation() { assert.fail("diagnostics must not change routing"); } });
  const drag = browser.__todoInputTrace.read().events.at(-1);
  assert.equal(drag.type, "dragover"); assert.equal(drag.clientX, 780); assert.equal(drag.clientY, 440);
});
