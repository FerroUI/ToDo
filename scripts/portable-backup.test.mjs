import test from "node:test";
import assert from "node:assert/strict";
import { exportPortable, restorePortable } from "../crates/todo-browser/wwwroot/backup/archive.js";
import { readBackupFile } from "../crates/todo-browser/wwwroot/backup/host.js";
import { MAX_ARCHIVE_BYTES, MAX_FILE_BYTES } from "../crates/todo-browser/wwwroot/backup/codec.js";
import { task } from "./tests/graph-fixture.mjs";

const attachment = (id, key, name = "plan.txt", size = 4) => ({ id, storage_key: key, name, mime_type: "text/plain", size });
function runtime(tasks = []) {
  let document = { schema_version: 1, tasks, lists: [{ id: 1, name: "Tasks" }], groups: [], next_id: 30,
    settings: { auto_sync: true, planner_enabled: true, account_name: "My account" } };
  const calls = [];
  return {
    calls, exportData: () => JSON.stringify(document), get state() { return document; },
    validateImportData(json) {
      calls.push("validate"); const value = JSON.parse(json);
      if (value.schema_version !== 1 || value.invalid || !Array.isArray(value.tasks) || !Array.isArray(value.lists)) throw new Error("Rust rejected the task document");
      return JSON.stringify(value);
    },
    importData(json) { calls.push("import"); document = JSON.parse(json); },
    mutate(action) { action(document); },
  };
}

async function portable({ duplicate = false, external = false } = {}) {
  const first = task(2); first.attachments = [attachment(3, "graph:private-account:list:task:file:modified")];
  if (external) {
    first.external = { provider: "Planner", id: "planner-task", url: "https://planner.cloud.microsoft/task/planner-task" };
    first.flagged_email = true; first.assigned_to = "Me";
  }
  const tasks = [first];
  if (duplicate) { const second = task(4); second.attachments = [attachment(5, first.attachments[0].storage_key)]; tasks.push(second); }
  const source = runtime(tasks);
  source.state.lists[0].cloud = { is_owner: false, is_shared: true, well_known_name: "none" };
  const blob = await exportPortable(source, { resolveFile: async () => new File(["plan"], "plan.txt", { type: "text/plain" }) });
  return { source, blob, archive: JSON.parse(await blob.text()) };
}

test("portable export includes file bytes and SHA-256 while excluding storage/account keys", async () => {
  const { source, blob, archive } = await portable({ duplicate: true });
  assert.equal(blob.type, "application/json");
  assert.equal(archive.kind, "ferroui.todo.backup");
  assert.equal(archive.version, 1);
  assert.equal(archive.files.length, 1, "two task references share one archived file");
  assert.equal(archive.files[0].base64, "cGxhbg==");
  assert.match(archive.files[0].sha256, /^[0-9a-f]{64}$/);
  assert.equal(archive.document.tasks[0].attachments[0].storage_key, "backup:1");
  assert.equal(archive.document.tasks[1].attachments[0].storage_key, "backup:1");
  assert.doesNotMatch(await blob.text(), /private-account|graph:/);
  assert.match(source.state.tasks[0].attachments[0].storage_key, /^graph:/, "export never rewrites the source document");
});

test("restore remaps shared files, commits once, and restores service tasks as independent local copies", async () => {
  const { archive } = await portable({ duplicate: true, external: true });
  const target = runtime([task(20, "Old task")]);
  const stored = new Map();
  const result = await restorePortable(target, JSON.stringify(archive), { storeFile: async (key, file) => stored.set(key, file) });
  assert.deepEqual(result, { tasks: 2, files: 1, legacy: false });
  assert.equal(target.calls.filter(call => call === "import").length, 1);
  const key = target.state.tasks[0].attachments[0].storage_key;
  assert.notEqual(key, "backup:1"); assert.notEqual(key, "graph:private-account:list:task:file:modified");
  assert.equal(target.state.tasks[1].attachments[0].storage_key, key);
  assert.equal(await stored.get(key).text(), "plan");
  assert.equal(stored.get(key).name, "plan.txt");
  assert.equal(target.state.settings.auto_sync, false);
  assert.equal(target.state.settings.planner_enabled, false);
  assert.equal(target.state.lists[0].cloud, undefined);
  assert.equal(target.state.tasks[0].external, undefined);
  assert.equal(target.state.tasks[0].assigned_to, null);
  assert.equal(target.state.tasks[0].flagged_email, false);
  assert.equal(target.state.tasks[0].linked_resources[0].web_url, archive.document.tasks[0].external.url);
  assert.equal(archive.document.tasks[0].external.provider, "Planner", "archive retains the original service metadata");
});

test("all document/file validation finishes before any file store or document replacement", async () => {
  const { archive } = await portable();
  const badValues = [
    { ...archive, version: 99 },
    { ...archive, clientSecret: "never-import-host-credentials" },
    { ...archive, document: { ...archive.document, invalid: true } },
    { ...archive, files: [] },
    { ...archive, files: [archive.files[0], archive.files[0]] },
    { ...archive, files: [{ ...archive.files[0], key: "unused" }] },
    { ...archive, files: [{ ...archive.files[0], name: "different.txt" }] },
    { ...archive, files: [{ ...archive.files[0], size: -1 }] },
    { ...archive, files: [{ ...archive.files[0], sha256: "0".repeat(64) }] },
    { ...archive, files: [{ ...archive.files[0], base64: "cGxhbg=" }] },
    { ...archive, files: [{ ...archive.files[0], base64: "cGxhbh==" }] },
    { ...archive, files: [{ ...archive.files[0], token: "not-a-file-field" }] },
  ];
  for (const value of badValues) {
    const target = runtime([task()]), before = target.exportData(); let writes = 0;
    await assert.rejects(restorePortable(target, JSON.stringify(value), { storeFile: async () => writes++ }));
    assert.equal(writes, 0); assert.equal(target.exportData(), before); assert.equal(target.calls.includes("import"), false);
  }
});

test("missing content, changed file sizes, and aggregate limits fail before producing a misleading export", async () => {
  const source = runtime([task()]); source.state.tasks[0].attachments = [attachment(3, "missing")];
  await assert.rejects(exportPortable(source, { resolveFile: async () => null }), /unavailable.*task-only/);
  await assert.rejects(exportPortable(source, { resolveFile: async () => new Blob(["wrong size"]) }), /changed size/);
  source.state.tasks[0].attachments = Array.from({ length: 6 }, (_, index) => attachment(index + 3, `large:${index}`, `large-${index}`, MAX_FILE_BYTES));
  let reads = 0;
  await assert.rejects(exportPortable(source, { resolveFile: async () => reads++ }), /128 MB/);
  assert.equal(reads, 0);
});

test("export and restore preserve local edits made during asynchronous work", async () => {
  const { source, archive } = await portable();
  await assert.rejects(exportPortable(source, { resolveFile: async () => { source.mutate(state => state.tasks[0].title = "Fresh edit"); return new Blob(["plan"]); } }), /Tasks changed/);
  assert.equal(source.state.tasks[0].title, "Fresh edit");
  const target = runtime([task()]), written = [], removed = [];
  await assert.rejects(restorePortable(target, JSON.stringify(archive), {
    storeFile: async key => { written.push(key); target.mutate(state => state.tasks[0].title = "Keep my newer edit"); },
    deleteFile: async key => removed.push(key),
  }), /Tasks changed/);
  assert.deepEqual(removed, written); assert.equal(target.state.tasks[0].title, "Keep my newer edit");
  assert.equal(target.calls.includes("import"), false);
});

test("a failed file write or final import rolls back only new file keys and preserves the original error", async () => {
  const { archive } = await portable();
  for (const phase of ["store", "import"]) {
    const target = runtime([task()]), before = target.exportData(), written = [], removed = [];
    const failure = new Error(`${phase} interrupted`);
    if (phase === "import") target.importData = () => { throw failure; };
    await assert.rejects(restorePortable(target, JSON.stringify(archive), {
      storeFile: async key => { written.push(key); if (phase === "store") throw failure; },
      deleteFile: async key => { removed.push(key); throw new Error("cleanup also interrupted"); },
    }), error => error === failure);
    assert.deepEqual(removed, written); assert.equal(target.exportData(), before);
    assert.notEqual(written[0], archive.files[0].key);
  }
});

test("legacy task-only JSON remains supported without pretending it contains file blobs", async () => {
  const first = task(); first.attachments = [attachment(3, "existing-local-file")];
  first.external = { provider: "Planner", id: "source", url: "https://planner.cloud.microsoft/task/source" };
  const source = runtime([first]), target = runtime();
  let writes = 0;
  const result = await restorePortable(target, source.exportData(), { storeFile: async () => writes++ });
  assert.deepEqual(result, { tasks: 1, files: 0, legacy: true });
  assert.equal(writes, 0); assert.equal(target.state.tasks[0].attachments[0].storage_key, "existing-local-file");
  assert.equal(target.state.tasks[0].external, undefined); assert.equal(target.state.settings.auto_sync, false);
});

test("file-size limit is checked before reading or parsing a selected backup", async () => {
  const oversized = new Blob(["{}"]); let reads = 0;
  Object.defineProperty(oversized, "size", { value: MAX_ARCHIVE_BYTES + 1 });
  oversized.text = async () => { reads++; return "{}"; };
  await assert.rejects(readBackupFile(oversized), /256 MB/);
  assert.equal(reads, 0);
});

test("binary and empty files round-trip; a damaged final entry prevents every storage write", async () => {
  const first = task();
  first.attachments = [attachment(3, "binary", "binary.bin", 5), attachment(4, "empty", "empty.txt", 0)];
  const contents = new Map([["binary", new Blob([new Uint8Array([0, 255, 128, 65, 90])])], ["empty", new Blob([])]]);
  const archive = JSON.parse(await (await exportPortable(runtime([first]), { resolveFile: async file => contents.get(file.storage_key) })).text());
  const restored = [];
  await restorePortable(runtime(), JSON.stringify(archive), { storeFile: async (_key, file) => restored.push([...new Uint8Array(await file.arrayBuffer())]) });
  assert.deepEqual(restored, [[0, 255, 128, 65, 90], []]);
  archive.files[1].sha256 = "0".repeat(64);
  let writes = 0;
  await assert.rejects(restorePortable(runtime(), JSON.stringify(archive), { storeFile: async () => writes++ }), /checksum/);
  assert.equal(writes, 0);
});

test("legacy reference-only backups are not limited by the size of absent file contents", async () => {
  const first = task();
  first.attachments = Array.from({ length: 6 }, (_, index) => attachment(index + 3, `existing:${index}`, `large-${index}`, MAX_FILE_BYTES));
  const target = runtime();
  const result = await restorePortable(target, runtime([first]).exportData());
  assert.equal(result.legacy, true); assert.equal(result.files, 0); assert.equal(target.state.tasks[0].attachments.length, 6);
});
