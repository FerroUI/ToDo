import test from "node:test";
import assert from "node:assert/strict";
import { DeliveryQueue } from "../src/queue.mjs";
import { fixture, origin, random } from "./fixture.mjs";

test("due delivery is claimed durably and repeated ticks or harmless saves cannot redeliver", async () => {
  const api = await fixture(), due = api.job(), later = api.job({ at: api.now() + 1000, expiresAt: api.now() + 1000 + api.config.expiryMs });
  await api.update([due, later]); const calls = [];
  const queue = new DeliveryQueue(api.store, api.config, { now: api.now, send: async (_sub, payload, options) => {
    const data = JSON.parse(payload); calls.push(data);
    assert(api.store.snapshot().installations[api.enrollment.id].jobs[data.id].claim);
    assert.equal(options.contentEncoding, "aes128gcm"); assert(options.TTL <= 3600); assert.equal(options.topic.length, 32);
    assert(!payload.includes(api.enrollment.capability)); assert(!payload.includes(origin));
  } });
  await Promise.all([queue.tick(), queue.tick()]); assert.equal(calls.length, 1);
  await api.update([due, later], 1); await queue.tick(); assert.equal(calls.length, 1);
  api.advance(1000); await queue.tick(); assert.equal(calls.length, 2); await queue.close();
});

test("transient failures retry with bounded attempts and expiry; expired subscriptions are canceled", async () => {
  const api = await fixture(), job = api.job(); await api.update([job]); let calls = 0;
  const queue = new DeliveryQueue(api.store, api.config, { now: api.now, send: async () => {
    calls++; throw Object.assign(new Error("provider body must not persist"), { statusCode: calls < 2 ? 429 : 410, headers: { "retry-after": "10" } });
  } });
  await queue.tick(); await queue.tick(); assert.equal(calls, 1);
  api.advance(9999); await queue.tick(); assert.equal(calls, 1);
  api.advance(1); await queue.tick(); assert.equal(calls, 2);
  const item = api.store.snapshot().installations[api.enrollment.id];
  assert.equal(item.subscription, null); assert.deepEqual(item.jobs, {}); assert.equal(item.revision, 2);
  assert(!JSON.stringify(item).includes("provider body")); await queue.close();
});

test("inflight cancellation wins over acknowledgement and crash claims recover only after their lease", async () => {
  const api = await fixture(); await api.update([api.job()]); let finish, calls = 0;
  const queue = new DeliveryQueue(api.store, api.config, { now: api.now, send: () => { calls++; return new Promise(resolve => { finish = resolve; }); } });
  const pending = queue.tick(); while (!finish) await new Promise(resolve => setImmediate(resolve));
  await api.update([], 1); finish(); await pending;
  assert.deepEqual(api.store.snapshot().installations[api.enrollment.id].jobs, {});
  const next = api.job(); await api.update([next], 2);
  await api.store.transact(state => { state.installations[api.enrollment.id].jobs[next.id].claim = { id: random(16), until: api.now() + 60_000 }; });
  await queue.tick(); assert.equal(calls, 1);
  api.advance(60_000); const recovered = queue.tick(); finish = null;
  while (!finish) await new Promise(resolve => setImmediate(resolve)); finish(); await recovered;
  assert.equal(calls, 2); await queue.close();
});

test("expired reminders and installation leases are never sent, and retry count is finite", async () => {
  const api = await fixture({ maxAttempts: 2 }); await api.update([api.job()]); let calls = 0;
  const queue = new DeliveryQueue(api.store, api.config, { now: api.now, send: async () => { calls++; throw new Error("network"); } });
  await queue.tick(); api.advance(5000); await queue.tick(); api.advance(60_000); await queue.tick(); assert.equal(calls, 2);
  const next = api.job(); await api.update([next], 1); api.advance(api.config.expiryMs); await queue.tick(); assert.equal(calls, 2);
  const third = api.job(); await api.update([third], 2); await api.store.transact(state => { state.installations[api.enrollment.id].expiresAt = api.now(); });
  await queue.tick(); assert.equal(calls, 2); await queue.close();
});
