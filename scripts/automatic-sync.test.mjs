import test from "node:test";
import assert from "node:assert/strict";
import { automaticSync } from "../crates/todo-browser/wwwroot/cloud/automatic.js";

function harness(run = async () => {}) {
  let ready = true, next = 0;
  const timers = new Map(), errors = [];
  const sync = automaticSync({run,ready:() => ready, report:error => errors.push(error),
    timer:(callback,delay) => { timers.set(++next,{callback,delay}); return next; }, cancel:id => timers.delete(id)});
  return { sync, timers, errors, set ready(value) { ready = value; },
    async fire() { const [id,value] = [...timers][0]; timers.delete(id); value.callback(); await new Promise(resolve => setImmediate(resolve)); },
    get delay() { return [...timers.values()][0]?.delay; } };
}

test("automatic sync debounces edits and uses one foreground polling timer", async () => {
  let runs = 0; const h = harness(async () => runs++);
  h.sync.start(); await h.fire(); assert.equal(runs,1); assert.equal(h.delay,5000);
  h.sync.changed(); h.sync.changed(); assert.equal(h.timers.size,1); assert.equal(h.delay,900);
  await h.fire(); assert.equal(runs,2); assert.equal(h.delay,5000);
  h.ready=false; h.sync.pause(); h.sync.changed(); assert.equal(h.timers.size,0);
  h.ready=true; h.sync.resume(); await h.fire(); assert.equal(runs,3);
});

test("a save during an active sync queues one follow-up and never overlaps requests", async () => {
  const gate = Promise.withResolvers(); let runs = 0;
  const h = harness(async () => { runs++; await gate.promise; });
  h.sync.start(); await h.fire(); h.sync.changed(); h.sync.changed();
  assert.equal(runs,1); assert.equal(h.timers.size,0);
  gate.resolve(); await new Promise(resolve => setImmediate(resolve));
  assert.equal(h.delay,900); await h.fire(); assert.equal(runs,2);
});

test("expired grants stop automatic retries until an explicit retry or local resolution", async () => {
  let fail = true; const h = harness(async () => { if(fail) throw Object.assign(new Error("Sign in"),{requiresInteraction:true}); });
  h.sync.start(); await h.fire(); assert.equal(h.timers.size,0); assert.equal(h.errors.length,1);
  h.sync.resume(); assert.equal(h.timers.size,0);
  fail=false; h.sync.retry(); await h.fire(); assert.equal(h.delay,5000);
});

test("transient failures back off without repeating identical errors, and stop remains terminal", async () => {
  const h = harness(async () => { throw new Error("Offline"); });
  h.sync.start(); await h.fire(); assert.equal(h.delay,10000);
  await h.fire(); assert.equal(h.delay,20000); assert.equal(h.errors.length,1);
  h.sync.stop(); h.sync.changed(); h.sync.resume(); assert.equal(h.timers.size,0);
});
