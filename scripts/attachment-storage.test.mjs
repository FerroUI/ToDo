import test from "node:test";
import assert from "node:assert/strict";
import { createAttachmentStore } from "../crates/todo-browser/wwwroot/attachment-store.js";

const turn = () => new Promise(resolve => setImmediate(resolve));

// Drive the browser's independent request and transaction events explicitly:
// request success does not imply commit, and abort need not be preceded by error.
function databaseFixture() {
  const transactions = [];
  const db = {
    closes: 0, objectStoreNames: { contains: () => true },
    close() { this.closes++; },
    transaction(names, mode) {
      const requests = [], writes = [];
      const tx = { names, mode, requests, writes, error: null,
        objectStore(name) { return {
          get(key) { const request = { name, key }; requests.push(request); return request; },
          put(value, key) { writes.push({ operation: "put", name, key, value }); return {}; },
          delete(key) { writes.push({ operation: "delete", name, key }); return {}; },
        }; },
        abort() { tx.onabort?.(); },
        complete() { tx.oncomplete?.(); },
        result(value) { const request = requests.at(-1); request.result = value; request.onsuccess?.(); },
      };
      transactions.push(tx);
      return tx;
    },
  };
  return { db, transactions };
}

function fixture() {
  const opens = [];
  const storage = createAttachmentStore({
    openDatabase() { const request = {}; opens.push(request); return request; },
    // Foreground transactions must also work when task storage denies cleanup.
    retainedKeys() { throw new Error("Task storage denied"); },
    now: () => 1234,
  });
  return { storage, opens, async open(db) { opens.at(-1).result = db; opens.at(-1).onsuccess(); await turn(); } };
}

test("attachment writes wait for transaction commit and reject abort after a successful request", async () => {
  const api = fixture(), first = databaseFixture();
  let settled = false;
  const put = api.storage.put("file-a", new Blob(["bytes"])).finally(() => { settled = true; });
  await api.open(first.db);
  const tx = first.transactions.at(-1);
  assert.equal(tx.mode, "readwrite");
  assert.equal(settled, false, "issuing a write is not a successful save");
  const rejected = assert.rejects(put, /interrupted/);
  tx.abort();
  await rejected;
  const retry = api.storage.put("file-a", new Blob(["bytes"]));
  await turn();
  first.transactions.at(-1).complete();
  await retry;
  assert.equal(api.opens.length, 1, "a transaction abort does not discard a healthy connection");
});

test("deleting an attachment settles on abort even without an error event", async () => {
  const api = fixture(), first = databaseFixture();
  const deletion = api.storage.remove("file-a");
  await api.open(first.db);
  const tx = first.transactions.at(-1), file = new Blob(["keep for undo"]);
  tx.result(file);
  assert.deepEqual(tx.writes.map(({ operation, name, key }) => ({ operation, name, key })), [
    { operation: "put", name: "trash", key: "file-a" },
    { operation: "delete", name: "attachments", key: "file-a" },
  ]);
  assert.equal(tx.writes[0].value.deletedAt, 1234);
  const rejected = assert.rejects(deletion, /interrupted/);
  tx.abort();
  await rejected;
});

test("recovering an undo attachment returns bytes only after the recovery transaction commits", async () => {
  const api = fixture(), first = databaseFixture();
  let settled = false;
  const reading = api.storage.read("file-a").finally(() => { settled = true; });
  await api.open(first.db);
  const lookup = first.transactions.at(-1);
  lookup.result(undefined); lookup.complete();
  await turn();
  const recovery = first.transactions.at(-1), file = new Blob(["undo bytes"]);
  assert.equal(recovery.mode, "readwrite");
  recovery.result({ file, deletedAt: 1200 });
  await turn();
  assert.equal(settled, false);
  recovery.complete();
  assert.equal(await reading, file);
});

test("an aborted undo recovery reports its failure instead of hanging or returning uncommitted bytes", async () => {
  const api = fixture(), first = databaseFixture();
  const reading = api.storage.read("file-a");
  await api.open(first.db);
  const lookup = first.transactions.at(-1);
  lookup.result(undefined); lookup.complete();
  await turn();
  const recovery = first.transactions.at(-1);
  recovery.result({ file: new Blob(["bytes"]), deletedAt: 1200 });
  recovery.error = new DOMException("Disk quota exceeded", "QuotaExceededError");
  const rejected = assert.rejects(reading, /Disk quota exceeded/);
  recovery.abort();
  await rejected;
});

test("a schema change releases the old tab's connection and subsequent work opens a replacement", async () => {
  const api = fixture(), first = databaseFixture(), second = databaseFixture();
  const write = api.storage.put("file-a", new Blob());
  await api.open(first.db); first.transactions.at(-1).complete(); await write;
  first.db.onversionchange();
  assert.equal(first.db.closes, 1);
  const next = api.storage.put("file-b", new Blob());
  assert.equal(api.opens.length, 2);
  await api.open(second.db);
  first.db.onclose();
  second.transactions.at(-1).complete(); await next;
  const third = api.storage.put("file-c", new Blob());
  await turn(); second.transactions.at(-1).complete(); await third;
  assert.equal(api.opens.length, 2, "a stale close must not discard the replacement connection");
});

test("an unexpectedly closed database is reopened for the next operation", async () => {
  const api = fixture(), first = databaseFixture(), second = databaseFixture();
  const write = api.storage.put("file-a", new Blob());
  await api.open(first.db); first.transactions.at(-1).complete(); await write;
  first.db.onclose();
  const next = api.storage.put("file-b", new Blob());
  assert.equal(api.opens.length, 2);
  await api.open(second.db); second.transactions.at(-1).complete(); await next;
});

test("a blocked open rejects promptly, closes a late success, and leaves a retried connection usable", async () => {
  const api = fixture(), stale = databaseFixture(), current = databaseFixture();
  const first = api.storage.put("file-a", new Blob()), request = api.opens[0];
  const rejected = assert.rejects(first, /Close another To Do tab.*try again/);
  request.onblocked(); await rejected;
  const retry = api.storage.put("file-a", new Blob());
  await api.open(current.db);
  request.result = stale.db; request.onsuccess();
  assert.equal(stale.db.closes, 1);
  assert.equal(stale.transactions.length, 0);
  current.transactions.at(-1).complete(); await retry;
  assert.equal(api.opens.length, 2);
});

test("a discarded blocked request aborts a later upgrade rather than holding the database open", async () => {
  const api = fixture();
  const opening = api.storage.read("file-a"), request = api.opens[0];
  const rejected = assert.rejects(opening, /Close another To Do tab/);
  request.onblocked(); await rejected;
  let aborted = 0;
  request.transaction = { abort() { aborted++; } };
  request.onupgradeneeded();
  assert.equal(aborted, 1);
});
