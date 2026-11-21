import test from "node:test";
import assert from "node:assert/strict";
import { createDocumentStorage } from "../crates/todo-browser/wwwroot/document-storage.js";

class Storage {
  data = new Map();
  failWrite = false;
  getItem(key) { return this.data.get(key) ?? null; }
  setItem(key, value) {
    if (this.failWrite) throw new Error("Quota exceeded");
    this.data.set(key, String(value));
  }
  removeItem(key) { this.data.delete(key); }
}
const KEY = "ferroui.todo.v1";
const document = title => JSON.stringify({ tasks: [{ id: 2, title }] });
const tab = (storage, key = KEY) => createDocumentStorage(() => storage, key);

test("a stale tab cannot overwrite another tab's saved tasks", () => {
  const storage = new Storage();
  storage.setItem(KEY, document("Initial"));
  const first = tab(storage), second = tab(storage);
  assert.equal(first.load(), second.load());
  first.save(document("First tab changed this"));
  const unsaved = document("Second tab's local edit");
  assert.throws(() => second.save(unsaved), /another tab.*Export a backup.*reload/i);
  assert.equal(storage.getItem(KEY), document("First tab changed this"));
  assert.equal(unsaved, document("Second tab's local edit"));
  // Retrying without reloading does not silently adopt the other tab's baseline.
  assert.throws(() => second.save(unsaved), /another tab/);
});

test("successful saves advance the baseline and an explicit reload permits new edits", () => {
  const storage = new Storage();
  const first = tab(storage), second = tab(storage);
  assert.equal(first.load(), "");
  assert.equal(second.load(), "");
  first.save(document("One"));
  first.save(document("Two"));
  assert.throws(() => second.save(document("Stale")), /another tab/);
  assert.equal(second.load(), document("Two"));
  second.save(document("Three"));
  second.save(document("Four"));
  assert.equal(storage.getItem(KEY), document("Four"));
  assert.throws(() => first.save(document("Older")), /another tab/);
});

test("a failed write keeps the observed baseline and can safely retry", () => {
  const storage = new Storage();
  storage.setItem(KEY, document("Before"));
  const current = tab(storage);
  current.load();
  storage.failWrite = true;
  assert.throws(() => current.save(document("After")), /Quota exceeded/);
  storage.failWrite = false;
  assert.equal(storage.getItem(KEY), document("Before"));
  current.save(document("After"));
  current.save(document("Final"));
  assert.equal(storage.getItem(KEY), document("Final"));
});

test("recovering a corrupted document does not change the expected active value", () => {
  const storage = new Storage();
  storage.setItem(KEY, "{unreadable");
  const current = tab(storage);
  const corrupted = current.load();
  storage.setItem(`${KEY}.recovery.123`, corrupted);
  current.save(document("Recovered"));
  assert.equal(storage.getItem(`${KEY}.recovery.123`), "{unreadable");
  assert.equal(storage.getItem(KEY), document("Recovered"));
});

test("demo and real task documents have independent baselines", () => {
  const storage = new Storage();
  const normal = tab(storage), demo = tab(storage, "ferroui.todo.demo.v1");
  normal.load(); demo.load();
  normal.save(document("Real task"));
  demo.save(document("Demo task"));
  normal.save(document("Real edit"));
  demo.save(document("Demo edit"));
  assert.equal(storage.getItem(KEY), document("Real edit"));
  assert.equal(storage.getItem("ferroui.todo.demo.v1"), document("Demo edit"));
});

test("cleared storage and a missing-to-empty change are detected", () => {
  const storage = new Storage();
  const current = tab(storage);
  current.load();
  storage.setItem(KEY, "");
  assert.throws(() => current.save(document("Stale")), /another tab/);
  current.load(); current.save(document("Saved"));
  storage.removeItem(KEY);
  assert.throws(() => current.save(document("Stale again")), /another tab/);
  assert.equal(storage.getItem(KEY), null);
});

test("saving without a successful initial read cannot overwrite an existing document", () => {
  const storage = new Storage();
  storage.setItem(KEY, document("Keep me"));
  const current = tab(storage);
  assert.throws(() => current.save(document("Uninitialized")), /not read successfully.*Export.*reload/);
  assert.equal(storage.getItem(KEY), document("Keep me"));
});
