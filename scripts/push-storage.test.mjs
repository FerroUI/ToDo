import test from "node:test";
import assert from "node:assert/strict";
import "../crates/todo-browser/wwwroot/push-storage.js";

const turn = () => new Promise(resolve => setImmediate(resolve));
function database() {
  const transactions = [], db = { closes: 0, close() { this.closes++; }, transaction(names, mode) {
    const reads = [], writes = [];
    const tx = { names, mode, reads, writes, error: null,
      objectStore(name) { return {
        get(key) { const request = { name, key }; reads.push(request); return request; },
        openCursor() { const request = { name }; reads.push(request); return request; },
        put(value, key) { writes.push({ name, key, value }); },
        delete(key) { writes.push({ name, key, delete: true }); }, clear() { writes.push({ name, clear: true }); },
      }; },
      result(value) { reads.at(-1).onsuccess({ target: { result: value } }); },
      complete() { tx.oncomplete(); }, abort() { tx.onabort(); },
    }; transactions.push(tx); return tx;
  } };
  return { db, transactions };
}
function fixture() {
  const opens = [], store = new globalThis.TodoPushStore("https://example.test/ToDo/", { open() { const request = {}; opens.push(request); return request; } });
  return { store, opens, async ready(db, index = opens.length - 1) { opens[index].result = db; opens[index].onsuccess(); await turn(); } };
}

test("push storage rejects a blocked attempt and closes its late connection without invalidating a retry", async () => {
  const api = fixture(), stale = database(), current = database();
  const first = api.store.read(), rejected = assert.rejects(first, /Close older/);
  api.opens[0].onblocked(); await rejected;
  const retry = api.store.read(); await api.ready(current.db);
  await api.ready(stale.db, 0); assert.equal(stale.db.closes, 1); assert.equal(stale.transactions.length, 0);
  current.transactions[0].result({ enabled: false }); current.transactions[0].complete(); assert.deepEqual(await retry, { enabled: false });
  const next = api.store.read(); await turn(); current.transactions[1].result(null); current.transactions[1].complete(); await next;
  assert.equal(api.opens.length, 2);
});

test("abandoned upgrades abort, synchronous open failures retry, and old close events cannot discard a new connection", async () => {
  const api = fixture(), first = api.store.read(), rejected = assert.rejects(first, /Close older/);
  api.opens[0].onblocked(); await rejected;
  let aborted = false; api.opens[0].transaction = { abort() { aborted = true; } }; api.opens[0].onupgradeneeded(); assert(aborted);
  let calls = 0;
  const store = new globalThis.TodoPushStore("scope", { open() { calls++; throw new Error("Denied"); } });
  await assert.rejects(store.read(), /Denied/); await assert.rejects(store.read(), /Denied/); assert.equal(calls, 2);
  const old = database(), fresh = database(), read = api.store.read(); await api.ready(old.db);
  old.transactions[0].result(null); old.transactions[0].complete(); await read; old.db.onversionchange();
  const replacement = api.store.read(); await api.ready(fresh.db); old.db.onclose();
  fresh.transactions[0].result(null); fresh.transactions[0].complete(); await replacement;
  assert.equal(api.opens.length, 3);
});

test("notification claims atomically gate active state and receipts, and settle only after transaction commit", async () => {
  const api = fixture(), first = database(), input = { installation: "installation", workspace: "workspace", id: "job", occurrence: "occurrence",
    taskId: "3", date: "2026-11-23", minutes: 720, now: 1000, until: 5000 };
  let settled = false;
  const claiming = api.store.claim(input).finally(() => { settled = true; }); await api.ready(first.db);
  const tx = first.transactions[0]; assert.deepEqual(tx.names, ["state", "receipts"]); assert.equal(tx.mode, "readwrite");
  tx.result({ enabled: true, id: input.installation, workspace: input.workspace, expiresAt: 10_000,
    plan: { job: { at: 1000, expiresAt: 5000, occurrence: input.occurrence } } });
  tx.result(undefined); await turn(); assert.equal(settled, false); assert.equal(tx.writes.length, 1);
  tx.complete(); const claimed = await claiming; assert.equal(claimed.claim, tx.writes[0].value.claim.id);
  const retry = api.store.claim(input); await turn(); const duplicate = first.transactions[1];
  duplicate.result({ enabled: true, id: input.installation, workspace: input.workspace, expiresAt: 10_000,
    plan: { job: { at: 1000, expiresAt: 5000, occurrence: input.occurrence } } });
  duplicate.result({ deliveredAt: 900 }); duplicate.complete(); assert.equal(await retry, null); assert.equal(duplicate.writes.length, 0);
});

test("foreground receipt claims work after a server lease expires; push claims remain gated", async () => {
  const api = fixture(), db = database(), state = { enabled: true, id: "i", workspace: "w", expiresAt: 100, plan: {} };
  const input = { installation: "i", workspace: "w", id: "j", occurrence: "o", now: 1000, until: 2000 };
  const push = api.store.claim(input); await api.ready(db.db); db.transactions[0].result(state); db.transactions[0].complete(); assert.equal(await push, null);
  const foreground = api.store.claim({ ...input, requirePlan: false }); await turn(); const tx = db.transactions[1];
  tx.result(state); tx.result(undefined); tx.complete(); assert((await foreground).claim);
  const failed = api.store.update(value => ({ ...value, enabled: false })); await turn(); const failure = db.transactions[2]; failure.result(state);
  failure.error = new Error("Quota exceeded"); const rejected = assert.rejects(failed, /Quota exceeded/); failure.abort(); await rejected;
});

test("delivered receipts survive long closed-page intervals and are retired only after a changed occurrence grace period", async () => {
  const api = fixture(), db = database(), day = 86_400_000;
  const active = { workspace: "w", reminders: { "3": { date: "2026-11-23", minutes_after_midnight: 720 } } };
  const receipt = { workspace: "w", taskId: "3", date: "2026-11-23", minutes: 720, deliveredAt: 1, until: day };
  const run = async (value, time, expected) => {
    const pending = api.store.prune(time, expected); if (!api.opens[0].result) await api.ready(db.db); else await turn();
    const tx = db.transactions.at(-1), changes = [];
    tx.result({ value, delete() { changes.push("delete"); }, update(next) { changes.push(next); }, continue() {} });
    tx.result(null); tx.complete(); await pending; return changes;
  };
  assert.deepEqual(await run(receipt, 30 * day, active), []);
  const [obsolete] = await run(receipt, 30 * day, { workspace: "w", reminders: {} }); assert.equal(obsolete.obsoleteAt, 30 * day);
  assert.deepEqual(await run(obsolete, 38 * day, { workspace: "w", reminders: {} }), ["delete"]);
});
