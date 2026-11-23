import test from "node:test";
import assert from "node:assert/strict";
import { fixture, origin, random, subscription } from "./fixture.mjs";

test("enrollment is retryable and only hashed capabilities persist; origin and identity isolate installations", async () => {
  const api = await fixture();
  assert.equal((await api.schedules.enroll(api.enrollment, origin)).revision, 0);
  const persisted = JSON.stringify(api.store.snapshot());
  assert(!persisted.includes(api.enrollment.capability)); assert(!persisted.includes(api.config.enrollmentKey));
  assert.throws(() => api.schedules.get(api.enrollment.id, api.enrollment.capability, "https://evil.test"), /credentials/);
  assert.throws(() => api.schedules.get(api.enrollment.id, random(), origin), /credentials/);
  await assert.rejects(api.schedules.enroll({ ...api.enrollment, capability: random() }, origin), /already in use/);
  await assert.rejects(api.schedules.enroll({ ...api.enrollment, id: random(16), capability: random() }, origin), /already enrolled/);
});

test("full schedule replacement is atomic, CAS rejects stale writers, and cancellation removes jobs", async () => {
  const api = await fixture(), job = api.job();
  assert.equal((await api.update([job])).revision, 1);
  await assert.rejects(api.update([api.job()]), error => error.status === 409 && error.details.revision === 1);
  assert.equal(Object.keys(api.store.snapshot().installations[api.enrollment.id].jobs)[0], job.id);
  await assert.rejects(api.update([job, { ...api.job(), title: "" }], 1), /invalid/);
  assert.equal(api.schedules.get(api.enrollment.id, api.enrollment.capability, origin).revision, 1);
  assert.equal((await api.update([], 1)).scheduled, 0);
  assert.deepEqual(api.store.snapshot().installations[api.enrollment.id].jobs, {});
});

test("quotas, time boundaries and content IDs fail closed without changing a valid schedule", async () => {
  const api = await fixture({ maxJobs: 2 }), job = api.job(); await api.update([job]);
  for (const jobs of [[api.job(), api.job(), api.job()], [api.job({ at: api.now() + api.config.horizonMs + 1 })],
    [api.job({ expiresAt: api.now() + api.config.expiryMs + 1 })], [api.job({ title: "a\nsecret" })], [job, job]]) {
    await assert.rejects(api.update(jobs, 1), error => error.status === 400);
  }
  await assert.rejects(api.update([{ ...job, at: job.at + 1 }], 1), /reused/);
  assert.equal(api.schedules.get(api.enrollment.id, api.enrollment.capability, origin).revision, 1);
  const next = subscription(); await api.update([], 1, { subscription: next });
  assert.equal(api.store.snapshot().installations[api.enrollment.id].subscription.endpoint, next.endpoint);
  await api.schedules.remove(api.enrollment.id, api.enrollment.capability, origin);
  assert.equal(Object.keys(api.store.snapshot().installations).length, 0);
});

test("renaming pending and delivered reminders retains occurrence delivery state", async () => {
  const api = await fixture(), job = api.job(); await api.update([job]);
  await api.update([{ ...job, title: "New title" }], 1);
  assert.equal(api.store.snapshot().installations[api.enrollment.id].jobs[job.id].title, "New title");
  await api.store.transact(state => { state.installations[api.enrollment.id].jobs[job.id].deliveredAt = api.now(); });
  await api.update([{ ...job, title: "Another title" }], 2);
  assert.equal(api.schedules.get(api.enrollment.id, api.enrollment.capability, origin).scheduled, 0);
});

test("known browser subscription expiration bounds the lease and requires renewal after expiry", async () => {
  const api = await fixture(), next = { ...subscription(), expirationTime: api.now() + 1000 };
  assert.equal((await api.update([], 0, { subscription: next })).expiresAt, next.expirationTime);
  api.advance(1000); await assert.rejects(api.update([], 1), error => error.status === 410);
  const renewed = await api.update([], 1, { subscription: subscription() }); assert(renewed.expiresAt > api.now());
  await assert.rejects(api.schedules.enroll({ ...api.enrollment, id: random(16), subscription: next }, origin), /already expired/);
});
