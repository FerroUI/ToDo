import test from "node:test";
import assert from "node:assert/strict";
import { fixture, task, key, config } from "./tests/push-fixture.mjs";
import { buildPlan, reminderIdentity, reminderInstant } from "../crates/todo-browser/wwwroot/push-plan.js";
import { serviceUrl, serviceConfig } from "../crates/todo-browser/wwwroot/push-api.js";
import { workerSupportsPush } from "../crates/todo-browser/wwwroot/push-protocol.js";

test("opt-in pins the public VAPID key, keeps credentials private, and sends only active reminder data", async t => {
  const api = fixture({ tasks: [task(), task({ id: 4, completed: true }), task({ id: 5, reminder: { ...task().reminder, notified: true } })] });
  t.after(() => api.client.close()); await api.enable();
  const state = await api.store.read(), remote = api.remote.get(state.id);
  assert.equal(remote.jobs.length, 1); assert.equal(remote.jobs[0].title, "Remember"); assert.equal(api.subscriptions(), 1);
  assert.equal(api.permissionRequests(), 0); assert.equal(state.enabled, true);
  assert(!JSON.stringify(api.states).includes(state.capability)); assert(!JSON.stringify(state).includes("x".repeat(43)));
  assert(api.requests.every(request => request.credentials === "omit" && request.redirect === "error" && request.referrerPolicy === "no-referrer"));
  assert.deepEqual(Object.keys(remote.jobs[0]).sort(), ["at", "expiresAt", "id", "title"]);
  assert.throws(() => api.client.enable({ service_url: "http://evil.test", public_key: key, enrollment_key: "x".repeat(43) }), /HTTPS/);
});

test("account changes, completion, deletion and edits replace the private and remote schedule", async t => {
  const api = fixture(); t.after(() => api.client.close()); await api.enable();
  const initial = await api.store.read(), firstJob = Object.values(initial.plan)[0];
  api.setAccount("different-Microsoft-account"); await api.client.sync();
  const switched = await api.store.read(); assert.notEqual(switched.workspace, initial.workspace);
  assert.notEqual(Object.keys(switched.plan)[0], firstJob.id);
  assert(!JSON.stringify(api.requests).includes("different-Microsoft-account"));
  api.setDocument({ tasks: [task({ title: "New title" })] }); await api.client.sync();
  assert.equal(Object.keys((await api.store.read()).plan)[0], Object.keys(switched.plan)[0], "rename retains occurrence identity");
  api.setDocument({ tasks: [task({ completed: true })] }); await api.client.sync();
  assert.deepEqual(api.remote.get(initial.id).jobs, []);
  api.setDocument({ tasks: [] }); await api.client.sync(); assert.deepEqual((await api.store.read()).plan, {});
});

test("a stale revision rebuilds from a newly active account instead of replaying an older body", async t => {
  const api = fixture(); t.after(() => api.client.close()); await api.enable();
  let once = false;
  api.afterGet(item => { if (!once) { once = true; item.revision++; api.setAccount("new-account"); api.setDocument({ tasks: [task({ id: 9, title: "New account task" })] }); } });
  await api.client.sync(true);
  const state = await api.store.read(), item = api.remote.get(state.id);
  assert.equal(item.jobs.length, 1); assert.equal(item.jobs[0].title, "New account task"); assert.equal(item.workspace, state.workspace);
});

test("offline completion cancels worker delivery immediately, and reconnection removes server jobs", async t => {
  const api = fixture(); t.after(() => api.client.close()); await api.enable(); const id = (await api.store.read()).id;
  api.setOffline(true); api.setDocument({ tasks: [task({ completed: true })] }); await api.client.localPlan();
  assert.deepEqual((await api.store.read()).plan, {}); assert.equal(api.remote.get(id).jobs.length, 1);
  await assert.rejects(api.client.sync(), /Offline/); api.setOffline(false); await api.client.sync();
  assert.deepEqual(api.remote.get(id).jobs, []);
});

test("revoked permission disables local delivery first, unsubscribes, and retries remote cleanup", async t => {
  const api = fixture(); t.after(() => api.client.close()); await api.enable(); const id = (await api.store.read()).id;
  api.setPermission("denied"); api.setOffline(true); await api.client.sync();
  let state = await api.store.read(); assert.equal(state.enabled, false); assert.deepEqual(state.plan, {}); assert.equal(api.unsubscribed(), 1);
  assert.equal(state.pendingDeletes.length, 1); assert(api.remote.has(id));
  api.setOffline(false); await api.client.sync(); state = await api.store.read();
  assert.equal(state.pendingDeletes.length, 0); assert(!api.remote.has(id));
});

test("explicit refresh renews an expired browser subscription without requesting Microsoft data", async t => {
  const api = fixture(); t.after(() => api.client.close()); await api.enable(); const state = await api.store.read();
  api.remote.get(state.id).subscriptionActive = false;
  await api.client.refresh(); assert.equal(api.unsubscribed(), 1); assert.equal(api.subscriptions(), 2);
  assert.equal(api.remote.get(state.id).subscriptionActive, true);
});

test("a missing browser subscription cannot be reported as successfully scheduled by an ordinary save", async t => {
  const api = fixture(); t.after(() => api.client.close()); await api.enable();
  await (await api.registration.pushManager.getSubscription()).unsubscribe();
  await assert.rejects(api.client.sync(), /subscription changed.*Refresh/);
  await api.client.refresh(); assert.equal(api.subscriptions(), 2); assert.match(api.states.at(-1).status, /scheduled/);
});

test("foreground delivery survives an expired offline lease and deduplicates the same occurrence", async t => {
  const api = fixture(); t.after(() => api.client.close()); await api.enable();
  await api.store.update(state => ({ ...state, expiresAt: api.now() - 1 })); api.setOffline(true);
  const action = { task_id: 3, date: task().reminder.date, minutes: 720, title: "Remember" }, shown = [];
  await api.client.foreground(action, async delivery => { assert(delivery.shouldShow()); shown.push(delivery.tag); });
  await api.client.foreground(action, async delivery => shown.push(delivery.tag));
  assert.equal(shown.length, 1); assert.match(shown[0], /^todo-push-/);
});

test("foreground transitions never leak an old account reminder, including disabled push and blocked imports", async t => {
  for (const enabled of [false, true]) {
    const api = fixture(); t.after(() => api.client.close()); if (enabled) await api.enable();
    const originalRead = api.store.read.bind(api.store); let switchOnce = false;
    api.store.read = async () => { const state = await originalRead(); if (!switchOnce) { switchOnce = true; api.setAccount("another-account"); } return state; };
    let shown = 0;
    await api.client.foreground({ task_id: 3, date: task().reminder.date, minutes: 720 }, async () => { shown++; });
    assert.equal(shown, 0);
    api.setBlocked(true);
    assert.equal(await api.client.foreground({ task_id: 3, date: task().reminder.date, minutes: 720 }, async () => { shown++; }), true);
    assert.equal(shown, 0);
  }
});

test("local import resets only local occurrence identities and unavailable/demo hosts never enroll", async t => {
  const api = fixture(); t.after(() => api.client.close()); await api.enable(); const first = (await api.store.read()).workspace;
  api.client.imported(); await api.store.pending; await api.client.localPlan();
  assert.notEqual((await api.store.read()).workspace, first);
  const disabled = fixture({ supported: false }); t.after(() => disabled.client.close()); await disabled.client.start();
  await assert.rejects(disabled.enable(), /Unsupported/); assert.equal(disabled.requests.length, 0); assert.equal(await disabled.store.read(), null);
});

test("a local-plan write cannot resurrect an imported generation or a concurrently switched account", async t => {
  for (const transition of ["import", "account"]) {
    const api = fixture(); t.after(() => api.client.close()); await api.enable();
    const original = await api.store.read(), update = api.store.update.bind(api.store); let once = false;
    api.store.update = async change => {
      if (!once) {
        once = true;
        if (transition === "import") await update(state => ({ ...state, localEpoch: "new-generation", plan: {}, workspace: null }));
        else { api.setAccount("new-account"); api.setDocument({ tasks: [task({ title: "New workspace task" })] }); }
      }
      return update(change);
    };
    const plan = await api.client.localPlan();
    assert.notEqual(plan.workspace, original.workspace);
    if (transition === "import") assert.equal(plan.localEpoch, "new-generation");
    else assert.equal(Object.values(plan.plan)[0].title, "New workspace task");
    assert.equal((await api.store.read()).workspace, plan.workspace);
  }
});

test("worker capability handshake rejects older or mismatched active deployments", async () => {
  assert.equal(await workerSupportsPush({ postMessage(_message, [port]) { port.postMessage({ type: "TODO_PUSH_READY", version: 1 }); } }), true);
  assert.equal(await workerSupportsPush({ postMessage(_message, [port]) { port.postMessage({ type: "TODO_PUSH_READY", version: 2 }); } }), false);
  assert.equal(await workerSupportsPush({ postMessage() {} }, 10), false);
  assert.equal(await workerSupportsPush({ postMessage() { throw new Error("Stopped worker"); } }), false);
});

test("optional push storage failures preserve guarded foreground delivery and can still report status", async t => {
  for (const operation of ["read", "claim"]) {
    const api = fixture(); t.after(() => api.client.close()); await api.enable();
    api.store[operation] = async () => { throw new Error("Private storage denied"); };
    if (operation === "read") api.store.update = async () => { throw new Error("Private storage denied"); };
    let shown = 0;
    await api.client.foreground({ task_id: 3, date: task().reminder.date, minutes: 720, title: "Remember" }, async delivery => {
      assert(delivery.shouldShow()); shown++; return true;
    });
    assert.equal(shown, 1); assert.match(api.states.at(-1).status, /storage denied/);
    await api.client.failed(new Error("Still denied")); assert.equal(api.states.at(-1).status, "Still denied");
  }
});

test("schedule bounds preserve title identity and reject malformed dates and untrusted configuration", async () => {
  const api = fixture(), workspace = "a".repeat(43), now = api.now();
  const plan = await buildPlan({ tasks: [task(), task({ id: 4 }), task({ id: 5 })] }, workspace, { ...config, maxJobs: 2 }, now);
  assert.equal(Object.keys(plan.plan).length, 2); assert.equal(plan.omitted, 1);
  assert.equal(reminderInstant("2026-02-30", 100), null); assert.equal(reminderInstant("2026-11-23", 1440), null);
  assert.equal((await reminderIdentity(workspace, 3, "2026-11-23", 720)).id, Object.keys(plan.plan)[0]);
  for (const url of ["https://user:password@example.test", "https://example.test/?token=secret", "javascript:alert(1)"]) assert.throws(() => serviceUrl(url));
  assert.throws(() => serviceConfig({ ...config, maxJobs: 1001 }, key), /configuration/);
});
