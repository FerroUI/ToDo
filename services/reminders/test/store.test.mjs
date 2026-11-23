import test from "node:test";
import assert from "node:assert/strict";
import * as fs from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, dirname } from "node:path";
import { DurableStore } from "../src/store.mjs";
import { fixture } from "./fixture.mjs";

async function directory(t) {
  const path = await fs.mkdtemp(join(tmpdir(), "todo-reminders-"));
  t.after(() => fs.rm(path, { recursive: true, force: true })); return join(path, "reminders.json");
}

test("durable state reopens with queue claims intact, private permissions, and one-writer lock", async t => {
  const path = await directory(t), api = await fixture(), job = api.job(); await api.update([job]);
  const store = await DurableStore.open(path);
  await assert.rejects(DurableStore.open(path), /locked/);
  await store.transact(state => Object.assign(state, api.store.snapshot()));
  assert.equal((await fs.stat(path)).mode & 0o777, 0o600);
  await store.close(); const reopened = await DurableStore.open(path);
  assert.deepEqual(reopened.snapshot(), api.store.snapshot()); await reopened.close();
});

test("a directory fsync failure after rename poisons queued mutations and delivery snapshots", async t => {
  const path = await directory(t), api = await fixture(); let fail = false;
  const store = await DurableStore.open(path, { io: { rename: fs.rename, rm: fs.rm, async open(name, ...args) {
    if (name === dirname(path) && fail) return { async sync() { throw new Error("Injected directory fsync failure"); }, async close() {} };
    return fs.open(name, ...args);
  } } });
  await store.transact(state => Object.assign(state, api.store.snapshot()));
  fail = true;
  const first = store.transact(state => { state.installations[api.enrollment.id].revision = 7; });
  const second = store.transact(state => { state.installations[api.enrollment.id].revision = 8; });
  await assert.rejects(first, /Injected/); await assert.rejects(second, /durability is uncertain/);
  assert.throws(() => store.snapshot(), /durability is uncertain/);
  assert.equal(JSON.parse(await fs.readFile(path, "utf8")).installations[api.enrollment.id].revision, 7);
  await store.close(); const reopened = await DurableStore.open(path);
  assert.equal(reopened.snapshot().installations[api.enrollment.id].revision, 7); await reopened.close();
});

test("malformed persisted subscriptions, jobs and claims fail closed before the scheduler starts", async t => {
  const path = await directory(t), api = await fixture(), job = api.job(); await api.update([job]);
  for (const change of [item => { item.subscription.endpoint = "http://localhost/private"; },
    item => { item.jobs[job.id].at = "tomorrow"; }, item => { item.jobs[job.id].claim = { id: "bad", until: 123 }; },
    item => { item.jobs[job.id].workspace = "other"; }, item => { item.revision = -1; }]) {
    const state = api.store.snapshot(); change(state.installations[api.enrollment.id]);
    await fs.writeFile(path, JSON.stringify(state)); await assert.rejects(DurableStore.open(path), /Invalid reminder store/);
  }
});
