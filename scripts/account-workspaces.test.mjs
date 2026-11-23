import test from "node:test";
import assert from "node:assert/strict";
import { activateWorkspace, assertWorkspace, checkpointWorkspace, commitWorkspaceImport, detachWorkspace, recoverWorkspaceSwitch, workspaceAccount, workspaceFileKeys } from "../crates/todo-browser/wwwroot/cloud/workspaces.js";
import { createDocumentStorage } from "../crates/todo-browser/wwwroot/document-storage.js";

const DOCUMENT = "ferroui.todo.v1", BINDING = "ferroui.todo.graph.v1.document-account";
const ACTIVE = "ferroui.todo.workspaces.v1.active", JOURNAL = "ferroui.todo.workspaces.v1.switch";
const snapshot = account => `ferroui.todo.workspaces.v1.document.${JSON.stringify(account)}`;
const doc = title => JSON.stringify({ tasks: title ? [{ id: 2, title, attachments: [{ storage_key: `file:${title}` }] }] : [], settings: {} });
class Storage {
  data = new Map(); fail;
  get length() { return this.data.size; }
  key(index) { return [...this.data.keys()][index] ?? null; }
  getItem(key) { return this.data.get(key) ?? null; }
  setItem(key, value) { if (this.fail?.(key, value)) throw new Error("Disk full"); this.data.set(key, value); }
  removeItem(key) { if (this.fail?.(key)) throw new Error("Disk full"); this.data.delete(key); }
}
function fixture(title = "Local", storage = new Storage()) {
  if (storage.getItem(DOCUMENT) === null) storage.setItem(DOCUMENT, doc(title));
  const persistence = createDocumentStorage(() => storage, DOCUMENT);
  let current = persistence.load();
  const runtime = {
    flushEdits() {}, exportData: () => current, emptyAccountData: () => doc(""),
    validateImportData: json => { const parsed = JSON.parse(json); assert.ok(Array.isArray(parsed.tasks)); return JSON.stringify(parsed); },
    prepareImportData: json => JSON.stringify(JSON.parse(json)),
    replaceAccountData(json, expected) { assert.equal(current, expected); persistence.save(json); current = json; },
  };
  return { storage, runtime, persistence,
    switch(account, extra = {}) { return activateWorkspace(runtime, account, { storage, afterCommit: json => persistence.acceptWorkspace(json), ...extra }); },
    edit(title) { persistence.save(doc(title)); current = doc(title); },
  };
}

test("switches preserve each account's pending edits, local workspace and sidecars", () => {
  const f = fixture();
  f.switch("Alice");
  assert.equal(f.runtime.exportData(), doc("Local"));
  f.edit("Alice changed");
  f.storage.setItem("ferroui.todo.graph.v1.Alice", "Alice sync history");
  f.storage.setItem("ferroui.todo.planner.v1.Alice", "Alice Planner history");
  f.switch("Bob");
  assert.equal(f.runtime.exportData(), doc(""));
  f.edit("Bob changed"); f.switch("Alice");
  assert.equal(f.runtime.exportData(), doc("Alice changed"));
  assert.equal(f.storage.getItem("ferroui.todo.graph.v1.Alice"), "Alice sync history");
  assert.equal(f.storage.getItem("ferroui.todo.planner.v1.Alice"), "Alice Planner history");
  f.switch(null); assert.equal(f.runtime.exportData(), doc("Local"));
  f.switch("Bob"); assert.equal(f.runtime.exportData(), doc("Bob changed"));
  assertWorkspace("Bob", f.storage);
  assert.throws(() => assertWorkspace("Alice", f.storage), /different account/);
  assert.deepEqual([...workspaceFileKeys(f.storage)].sort(), ["file:Alice changed", "file:Bob changed", "file:Local"].sort());
});

test("migrates an existing binding without copying its tasks to a newly selected account", () => {
  const storage = new Storage(); storage.setItem(BINDING, "Alice");
  const f = fixture("Alice existing", storage);
  f.switch("Bob"); assert.equal(f.runtime.exportData(), doc(""));
  f.switch("Alice"); assert.equal(f.runtime.exportData(), doc("Alice existing"));
});

test("a stale tab cannot write across accounts even when the task bytes are identical", () => {
  const f = fixture(""); f.switch("Alice");
  const stale = fixture("", f.storage);
  f.switch("Bob");
  assert.equal(stale.runtime.exportData(), f.runtime.exportData());
  assert.throws(() => stale.edit("Wrong account"), /another tab/);
  f.edit("Bob edits");
  assert.equal(f.storage.getItem(DOCUMENT), doc("Bob edits"));
});

test("storage and document validation failures keep the original account and document", () => {
  const f = fixture("Alice"); f.switch("Alice");
  f.storage.setItem(snapshot("Bob"), "{corrupt");
  assert.throws(() => f.switch("Bob"), SyntaxError);
  assert.equal(workspaceAccount(f.storage), "Alice");
  assert.equal(f.storage.getItem(JOURNAL), null);
  f.storage.removeItem(snapshot("Bob"));
  f.storage.fail = key => key === DOCUMENT;
  assert.throws(() => f.switch("Bob"), /Disk full/);
  assert.equal(workspaceAccount(f.storage), "Alice");
  assert.equal(f.runtime.exportData(), doc("Alice"));
  assert.equal(f.storage.getItem(DOCUMENT), doc("Alice"));
  assert.equal(f.storage.getItem(JOURNAL), null);
});

test("a committed document with interrupted sidecars completes the same account after restart", () => {
  const f = fixture("Alice"); f.switch("Alice");
  f.storage.fail = key => key === BINDING;
  assert.throws(() => f.switch("Bob"), /Disk full/);
  assert.equal(f.runtime.exportData(), doc(""));
  assert.notEqual(f.storage.getItem(JOURNAL), null);
  assert.throws(() => assertWorkspace("Bob", f.storage), /different account/);
  f.storage.fail = undefined;
  recoverWorkspaceSwitch(f.storage);
  assert.equal(workspaceAccount(f.storage), "Bob");
  assert.equal(f.storage.getItem(JOURNAL), null);
  const restarted = fixture("", f.storage);
  restarted.edit("Bob recovered"); restarted.switch("Alice");
  assert.equal(restarted.runtime.exportData(), doc("Alice"));
});

test("ambiguous crash recovery preserves its journal and refuses to relabel unrelated data", () => {
  const f = fixture("Unrelated");
  f.storage.setItem(JOURNAL, JSON.stringify({ version: 1, from: "Alice", to: "Bob", before: doc("Before"), after: doc("After") }));
  assert.throws(() => recoverWorkspaceSwitch(f.storage), /interrupted account switch/);
  assert.notEqual(f.storage.getItem(JOURNAL), null);
  assert.equal(f.runtime.exportData(), doc("Unrelated"));
});

test("local restores retain the preceding account snapshot and its mapping namespace", () => {
  const f = fixture("Alice"); f.switch("Alice");
  f.storage.setItem("ferroui.todo.graph.v1.Alice", "keep history");
  checkpointWorkspace(f.runtime, f.storage);
  f.edit("Imported local copy");
  detachWorkspace(f.runtime, { storage: f.storage, afterCommit: json => f.persistence.acceptWorkspace(json) });
  assert.equal(workspaceAccount(f.storage), null);
  f.switch("Alice");
  assert.equal(f.runtime.exportData(), doc("Alice"));
  assert.equal(f.storage.getItem("ferroui.todo.graph.v1.Alice"), "keep history");
  f.switch(null); assert.equal(f.runtime.exportData(), doc("Imported local copy"));
});

test("new account adoption is explicit once a local workspace has been established", () => {
  const f = fixture("Local"); f.switch(null);
  f.switch("Alice"); assert.equal(f.runtime.exportData(), doc(""));
  f.switch(null); f.switch("Bob", { adoptLocal: true });
  assert.equal(f.runtime.exportData(), doc("Local"));
  f.switch(null); f.edit("New local edits");
  f.switch("Bob", { adoptLocal: true });
  assert.equal(f.runtime.exportData(), doc("Local"));
});

test("a restore journal binds committed imported data to local before any account can resume", () => {
  const f = fixture("Alice"); f.switch("Alice");
  let revoked = false;
  f.storage.fail = key => key === ACTIVE;
  assert.throws(() => commitWorkspaceImport(f.runtime, doc("Imported"), json => {
    assert.equal(revoked, true, "session resume is revoked before the task write");
    f.runtime.replaceAccountData(json, f.runtime.exportData());
  }, { storage: f.storage, beforeCommit: () => { revoked = true; } }), /Disk full/);
  assert.equal(f.storage.getItem(DOCUMENT), doc("Imported"));
  assert.notEqual(f.storage.getItem(JOURNAL), null);
  f.storage.fail = undefined;
  recoverWorkspaceSwitch(f.storage);
  assert.equal(workspaceAccount(f.storage), null);
  assert.equal(f.storage.getItem(snapshot("Alice")), doc("Alice"));
  assert.equal(f.storage.getItem(snapshot(null)), doc("Imported"));
});
