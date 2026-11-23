import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import vm from "node:vm";
import { fixture, scope, task } from "./tests/push-fixture.mjs";

const source = await readFile(new URL("../crates/todo-browser/wwwroot/push-worker.js", import.meta.url), "utf8");
const context = { URL, Response, Uint8Array, atob, TextDecoder, AbortSignal, fetch };
vm.runInNewContext(source, context);
const workerApi = context.TodoPushWorker;

async function prepared(t) {
  const api = fixture(); t.after(() => api.client.close()); await api.enable();
  const state = await api.store.read(), job = Object.values(state.plan)[0], shown = [];
  const worker = { Notification: { permission: "granted" }, registration: { ...api.registration,
    showNotification: async (title, options) => { shown.push({ title, options }); } } };
  const payload = { version: 1, installation: state.id, workspace: state.workspace, id: job.id, at: job.at, expiresAt: job.expiresAt, title: "Untrusted server text", url: "https://evil.test/" };
  return { ...api, state, job, worker, payload, shown };
}

test("worker displays current local title with a safe app target and durable occurrence dedup", async t => {
  const api = await prepared(t);
  assert.equal(await workerApi.receive(api.worker, api.store, api.payload, api.now), true);
  assert.equal(await workerApi.receive(api.worker, api.store, api.payload, api.now), false);
  assert.equal(api.shown.length, 1); assert.equal(api.shown[0].title, "Remember");
  assert.equal(api.shown[0].options.data.url, scope); assert.equal(api.shown[0].options.renotify, false);
  assert.equal(api.shown[0].options.tag, `todo-push-${api.job.occurrence}`);
  let foreground = 0;
  await api.client.foreground({ task_id: 3, date: task().reminder.date, minutes: 720 }, async () => { foreground++; });
  assert.equal(foreground, 0);
});

test("worker rejects canceled, expired, wrong-account and malformed payloads", async t => {
  const api = await prepared(t);
  for (const payload of [{ ...api.payload, installation: "x".repeat(22) }, { ...api.payload, workspace: "x".repeat(43) },
    { ...api.payload, id: "bad" }, { ...api.payload, version: 2 }, { ...api.payload, at: api.now() + 60_000 }]) {
    assert.equal(await workerApi.receive(api.worker, api.store, payload, api.now), false);
  }
  api.setDocument({ tasks: [task({ completed: true })] }); await api.client.localPlan();
  assert.equal(await workerApi.receive(api.worker, api.store, api.payload, api.now), false);
  api.setDocument({ tasks: [task()] }); await api.client.localPlan(); api.advance(3600_000);
  assert.equal(await workerApi.receive(api.worker, api.store, api.payload, api.now), false); assert.equal(api.shown.length, 0);
});

test("a notification display failure releases its claim, while a concurrent completion wins before display", async t => {
  const api = await prepared(t); let failed = true;
  api.worker.registration.showNotification = async () => { if (failed) { failed = false; throw new Error("OS denied"); } };
  await assert.rejects(workerApi.receive(api.worker, api.store, api.payload, api.now), /OS denied/);
  assert.equal(api.store.receipts.size, 0);
  const originalClaim = api.store.claim.bind(api.store);
  api.store.claim = async input => {
    const result = await originalClaim(input); await api.store.update(state => ({ ...state, plan: {} })); return result;
  };
  assert.equal(await workerApi.receive(api.worker, api.store, api.payload, api.now), false);
  assert.equal(api.store.receipts.size, 0);
});

test("subscription replacement uploads the current schedule with CAS and revoked permission disables it", async t => {
  const api = await prepared(t), calls = [];
  const subscription = await api.registration.pushManager.getSubscription();
  let revision = 3;
  const fetcher = async (url, options) => {
    calls.push({ url: String(url), ...options });
    if (options.method === "PUT") { assert.equal(options.headers["if-match"], '"3"'); revision++; }
    return new Response(options.method === "DELETE" ? null : JSON.stringify({ revision, expiresAt: api.now() + 86_400_000 }), { status: options.method === "DELETE" ? 204 : 200 });
  };
  await workerApi.subscriptionChanged(api.worker, api.store, { newSubscription: subscription }, { fetcher, now: api.now });
  const uploaded = JSON.parse(calls.find(call => call.method === "PUT").body);
  assert.equal(uploaded.workspace, api.state.workspace); assert.equal(uploaded.jobs[0].id, api.job.id); assert(uploaded.subscription);
  api.worker.Notification.permission = "denied";
  await workerApi.subscriptionChanged(api.worker, api.store, { newSubscription: subscription }, { fetcher, now: api.now });
  assert.equal((await api.store.read()).enabled, false); assert.deepEqual((await api.store.read()).plan, {}); assert.equal(calls.at(-1).method, "DELETE");
});
