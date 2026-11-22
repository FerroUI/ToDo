import test from "node:test";
import assert from "node:assert/strict";
import { createLifecycle, attachLifecycle } from "../crates/todo-browser/wwwroot/lifecycle.js";

function fixture() {
  const host = new EventTarget();
  host.document = new EventTarget();
  host.document.hidden = false;
  const pointers = new Map();
  host.document.getElementById = id => {
    assert.equal(id, "out");
    return {
      addEventListener(type, listener, capture) { assert.equal(capture, true); pointers.set(type, listener); },
      removeEventListener(type, listener, capture) { assert.equal(capture, true); if (pointers.get(type) === listener) pointers.delete(type); },
    };
  };
  const timers = new Set();
  host.setInterval = (callback, delay) => { assert.equal(delay, 30_000); timers.add(callback); return callback; };
  host.clearInterval = timer => timers.delete(timer);
  const calls = [];
  const runtime = { cancelGestures: () => calls.push("cancel"), flushEdits: () => calls.push("flush"),
    recoverTouchSequence: () => calls.push("recover") };
  return { host, calls, runtime, tick: () => calls.push("tick"), reportError: error => calls.push(error.message),
    timers, emit: type => host.dispatchEvent(new Event(type)),
    pointer: (type, event) => pointers.get(type)?.(event),
    hidden(value) { host.document.hidden = value; host.document.dispatchEvent(new Event("visibilitychange")); } };
}

test("blur cancels contact without changing editor focus or flushing its draft", () => {
  const api = fixture();
  const dispose = attachLifecycle(api.runtime, api.tick, api.reportError, api.host);
  api.calls.length = 0;
  api.emit("blur");
  assert.deepEqual(api.calls, ["cancel"]);
  api.emit("focus");
  assert.deepEqual(api.calls, ["cancel", "tick"]);
  dispose();
});

test("hidden/pagehide cancel before saving drafts, and returning resumes the clock", () => {
  const api = fixture();
  const dispose = attachLifecycle(api.runtime, api.tick, api.reportError, api.host);
  assert.deepEqual(api.calls, ["tick"]);
  api.calls.length = 0;
  api.hidden(true);
  api.emit("pagehide");
  api.hidden(false);
  api.emit("beforeunload");
  assert.deepEqual(api.calls, ["cancel", "flush", "cancel", "flush", "tick", "cancel", "flush"]);
  dispose();
  api.calls.length = 0;
  api.emit("blur"); api.emit("pagehide"); api.hidden(true);
  assert.deepEqual(api.calls, []);
  assert.equal(api.timers.size, 0);
});

test("failed cancellation does not prevent saving drafts or remove lifecycle protection", () => {
  const api = fixture();
  const dispose = createLifecycle({ host: api.host, tick: api.tick, reportError: api.reportError,
    cancel: () => { throw new Error("cancel error"); }, flush: api.runtime.flushEdits });
  api.calls.length = 0;
  api.hidden(true);
  assert.deepEqual(api.calls, ["cancel error", "flush"]);
  api.calls.length = 0;
  api.emit("pagehide");
  assert.deepEqual(api.calls, ["cancel error", "flush"]);
  dispose();
});

test("replacing the runtime removes old listeners and the old clock", () => {
  const old = fixture();
  attachLifecycle(old.runtime, old.tick, old.reportError, old.host);
  const next = fixture();
  const dispose = attachLifecycle(next.runtime, next.tick, next.reportError, next.host);
  old.calls.length = 0; next.calls.length = 0;
  old.emit("blur"); old.hidden(true);
  next.emit("blur");
  assert.deepEqual(old.calls, []);
  assert.equal(old.timers.size, 0);
  assert.deepEqual(next.calls, ["cancel"]);
  assert.equal(next.timers.size, 1);
  dispose();
});

test("only a trusted primary touch Down reconciles an interrupted sequence before native input", () => {
  const api = fixture();
  const dispose = attachLifecycle(api.runtime, api.tick, api.reportError, api.host);
  api.emit("blur");
  api.calls.length = 0;
  api.emit("focus");
  api.pointer("pointermove", { isTrusted: true, pointerType: "touch", isPrimary: true });
  api.pointer("pointerdown", { isTrusted: true, pointerType: "touch", isPrimary: false });
  api.pointer("pointerdown", { isTrusted: true, pointerType: "mouse", isPrimary: true });
  api.pointer("pointerdown", { isTrusted: true, pointerType: "pen", isPrimary: true });
  api.pointer("pointerdown", { isTrusted: false, pointerType: "touch", isPrimary: true });
  assert.deepEqual(api.calls, ["tick"], "focus and surviving contacts leave quarantine intact");
  api.pointer("pointerdown", { isTrusted: true, pointerType: "touch", isPrimary: true });
  assert.deepEqual(api.calls, ["tick", "recover"]);
  dispose();
  api.pointer("pointerdown", { isTrusted: true, pointerType: "touch", isPrimary: true });
  assert.deepEqual(api.calls, ["tick", "recover"], "cleanup removes the capture listener");
});
