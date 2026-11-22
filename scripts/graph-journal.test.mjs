import test from "node:test";
import assert from "node:assert/strict";
import { commit, recoverCommit, forgetDocumentMappings } from "../crates/todo-browser/wwwroot/cloud/metadata.js";
import { Storage } from "./tests/graph-fixture.mjs";

const DOCUMENT = "ferroui.todo.v1", JOURNAL = "ferroui.todo.graph.v1.commit", META = "ferroui.todo.graph.v1.account";

test("restoring independent copies forgets mappings and correlation identity without deleting task data", () => {
  const storage = new Storage(); globalThis.localStorage = storage;
  for (const key of [META, JOURNAL, "ferroui.todo.graph.v1.document-account", "ferroui.todo.graph.v1.device"]) storage.setItem(key, "old");
  storage.setItem(DOCUMENT, "restored tasks"); storage.setItem("unrelated", "keep");
  forgetDocumentMappings();
  assert.equal(storage.getItem(DOCUMENT), "restored tasks"); assert.equal(storage.getItem("unrelated"), "keep");
  assert.equal(storage.length, 2);
});

test("an interrupted metadata write recovers against Rust's canonical prospective document", () => {
  const storage = new Storage(); globalThis.localStorage = storage;
  const before = JSON.stringify({ lists: [{ id: 1 }], sidebar_order: [], tasks: [] });
  const draft = { lists: [{ id: 1 }, { id: 2 }], sidebar_order: [99], tasks: [] };
  const prepared = JSON.stringify({ lists: [{ id: 1, sort_reversed: false }, { id: 2, sort_reversed: false }], sidebar_order: [2], tasks: [] });
  storage.setItem(DOCUMENT, before);
  const write = storage.setItem.bind(storage);
  storage.setItem = (key, value) => { if (key === META) throw new Error("Metadata storage failed"); write(key, value); };
  let preparedCalls = 0;
  const runtime = {
    prepareCloudData(json) { assert.deepEqual(JSON.parse(json), draft); preparedCalls++; return prepared; },
    replaceFromCloud(json, expected) {
      assert.equal(expected, before); assert.equal(json, prepared);
      write(DOCUMENT, json);
    },
  };
  assert.throws(() => commit(runtime, before, draft, META, { version: 1 }), /Metadata storage failed/);
  assert.equal(preparedCalls, 1);
  assert.equal(JSON.parse(storage.getItem(JOURNAL)).after, prepared);
  assert.equal(storage.getItem(DOCUMENT), prepared);
  storage.setItem = write;
  recoverCommit();
  assert.equal(storage.getItem(META), '{"version":1}');
  assert.equal(storage.getItem(DOCUMENT), prepared);
  assert.equal(storage.getItem(JOURNAL), null);
});

test("failed canonical preparation creates neither a journal nor a replacement", () => {
  const storage = new Storage(); globalThis.localStorage = storage;
  storage.setItem(DOCUMENT, "original");
  const runtime = {
    prepareCloudData() { throw new Error("Invalid prospective document"); },
    replaceFromCloud() { assert.fail("An invalid document cannot be committed"); },
  };
  assert.throws(() => commit(runtime, "original", {}, META, {}), /Invalid prospective/);
  assert.equal(storage.getItem(JOURNAL), null);
  assert.equal(storage.getItem(DOCUMENT), "original");
});

test("canonical journaling still rejects another tab's intervening document", () => {
  const storage = new Storage(); globalThis.localStorage = storage;
  const before = '{"tasks":[]}', after = '{"tasks":[{"id":2}]}', newer = '{"tasks":[{"id":3}]}';
  storage.setItem(DOCUMENT, before);
  const runtime = {
    prepareCloudData: () => after,
    replaceFromCloud() { storage.setItem(DOCUMENT, newer); throw new Error("Another tab changed tasks"); },
  };
  assert.throws(() => commit(runtime, before, JSON.parse(after), META, {}), /another tab changed tasks/);
  assert.equal(storage.getItem(DOCUMENT), newer);
  assert.equal(storage.getItem(META), null);
  assert(storage.getItem(JOURNAL));
});
