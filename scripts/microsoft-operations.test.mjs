import test from "node:test";
import assert from "node:assert/strict";
import { serializeMicrosoft } from "../crates/todo-browser/wwwroot/cloud/operations.js";

const sync = { kind: "syncMicrosoft" };
const disconnect = { kind: "disconnectMicrosoft" };
const deferred = () => Promise.withResolvers();

test("manual sync and refresh share one in-flight operation and its completion", async () => {
  const gate = deferred();
  let runs = 0;
  const request = serializeMicrosoft(async () => { runs++; await gate.promise; return "synced"; });
  const first = request(sync, "2026-11-22");
  const refresh = request(sync, "2026-11-22");
  assert.equal(first, refresh);
  await Promise.resolve();
  assert.equal(runs, 1);
  gate.resolve();
  assert.equal(await first, "synced");
  const later = request(sync, "2026-11-23");
  assert.notEqual(later, first, "completion releases the single-flight slot before notifying callers");
  assert.deepEqual(await Promise.all([refresh, later]), ["synced", "synced"]);
  assert.equal(runs, 2, "a completed sync does not swallow a later refresh");
});

test("disconnect waits for the active sync and cannot be overwritten by its completion", async () => {
  const gate = deferred();
  const events = [];
  let connected = true;
  const request = serializeMicrosoft(async action => {
    events.push(action.kind);
    if (action.kind === "syncMicrosoft") {
      await gate.promise;
      connected = true;
    } else connected = false;
  });
  const first = request(sync);
  const signOut = request(disconnect);
  await Promise.resolve();
  assert.deepEqual(events, ["syncMicrosoft"]);
  gate.resolve();
  await Promise.all([first, signOut]);
  assert.deepEqual(events, ["syncMicrosoft", "disconnectMicrosoft"]);
  assert.equal(connected, false);
});

test("account changes separate synchronization batches and failures do not block the queue", async () => {
  const gate = deferred();
  const events = [];
  let account = "original";
  let runs = 0;
  const request = serializeMicrosoft(async action => {
    if (action.kind === "syncMicrosoft") {
      events.push(`sync:${account}`);
      if (++runs === 1) { await gate.promise; throw new Error("network interrupted"); }
      return account;
    }
    events.push(`connect:${action.client_id}`);
    account = action.client_id;
  });
  const first = request(sync);
  const failed = assert.rejects(first, /network interrupted/);
  const connect = request({ kind: "connectMicrosoft", client_id: "replacement" });
  const later = request(sync);
  assert.notEqual(first, later);
  assert.equal(request(sync), later, "refresh joins the sync after the account transition");
  gate.resolve();
  await Promise.all([failed, connect]);
  assert.equal(await later, "replacement");
  assert.deepEqual(events, ["sync:original", "connect:replacement", "sync:replacement"]);
});
