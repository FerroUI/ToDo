import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import { mkdtempSync, mkdirSync, readFileSync, writeFileSync, readdirSync, rmSync, utimesSync } from "node:fs";
import { syncBuiltinESMExports } from "node:module";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createHash } from "node:crypto";
import { installStorage, keyName } from "../crates/todo-desktop/native/storage.mjs";
import { storeAttachment, readAttachment, deleteAttachment, purgeAttachment, cleanTrash } from "../crates/todo-desktop/native/files.mjs";
import { RETENTION_MS } from "../crates/todo-desktop/native/file-records.mjs";
import { synchronizationFileKeys } from "../crates/todo-desktop/native/file-retention.mjs";

function fixture(t) {
  const root = mkdtempSync(join(tmpdir(), "ferroui-native-files-"));
  installStorage(root);
  const directory = join(root, "files"); mkdirSync(directory);
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const metadataPath = key => join(directory, `${keyName(key)}.json`);
  const record = key => JSON.parse(readFileSync(metadataPath(key)));
  const writeRecord = (key, value) => writeFileSync(metadataPath(key), JSON.stringify(value));
  const payload = key => { const value = record(key); return join(directory, `${keyName(key)}${value.version === 2 ? `.${value.sha256}` : ""}.bin`); };
  return { root, directory, metadataPath, record, writeRecord, payload };
}

const document = (...keys) => ({ tasks: [{ attachments: keys.map(storage_key => ({ storage_key })) }] });
const digest = bytes => createHash("sha256").update(bytes).digest("hex");

test("native attachment commits immutable generations and preserves binary metadata", async t => {
  const files = fixture(t), key = "graph:account:list:task:file:modified";
  const bytes = Buffer.from([0, 1, 127, 128, 255]);
  await storeAttachment(key, new File([bytes], "Zażółć 🗂️.bin", { type: "application/octet-stream" }));
  assert.equal(files.record(key).version, 2);
  assert.equal(files.record(key).sha256, digest(bytes));
  const old = files.payload(key), first = await readAttachment(key);
  assert.equal(first.name, "Zażółć 🗂️.bin");
  assert.equal(first.type, "application/octet-stream");
  assert.deepEqual(Buffer.from(await first.arrayBuffer()), bytes);
  await storeAttachment(key, new File(["replacement"], "new.txt", { type: "text/plain" }));
  assert.notEqual(files.payload(key), old);
  assert.equal(fs.existsSync(old), false);
  assert.equal(await (await readAttachment(key)).text(), "replacement");
  await storeAttachment(key, new File(["replacement"], "renamed.txt"));
  assert.equal((await readAttachment(key)).name, "renamed.txt");
  assert.equal(readdirSync(files.directory).length, 2);
});

test("native legacy attachments remain readable and migrate only on replacement", async t => {
  const files = fixture(t), key = "legacy";
  files.writeRecord(key, { key, name: "old.txt", type: "text/plain", size: 6 });
  const old = files.payload(key); writeFileSync(old, "legacy");
  assert.equal(await (await readAttachment(key)).text(), "legacy");
  assert.equal(files.record(key).version, undefined);
  await storeAttachment(key, new File(["updated"], "old.txt"));
  assert.equal(await (await readAttachment(key)).text(), "updated");
  assert.equal(files.record(key).version, 2);
  assert.equal(fs.existsSync(old), false);
});

test("failed metadata publication leaves old attachment bytes and metadata intact", async t => {
  const files = fixture(t), key = "replace";
  await storeAttachment(key, new File(["original"], "keep.txt"));
  const originalMetadata = readFileSync(files.metadataPath(key)), originalPayload = files.payload(key);
  const rename = fs.renameSync;
  fs.renameSync = (source, destination) => {
    if (destination === files.metadataPath(key) || destination === files.metadataPath("new")) throw Object.assign(new Error("Metadata commit refused"), { code: "EACCES" });
    return rename(source, destination);
  };
  syncBuiltinESMExports();
  try {
    await assert.rejects(storeAttachment(key, new File(["changed!"], "changed.txt")), /Metadata commit refused/);
    await assert.rejects(storeAttachment("new", new File(["new bytes"], "new.txt")), /Metadata commit refused/);
  } finally { fs.renameSync = rename; syncBuiltinESMExports(); }
  assert.deepEqual(readFileSync(files.metadataPath(key)), originalMetadata);
  assert.equal(readFileSync(originalPayload, "utf8"), "original");
  assert.equal(await (await readAttachment(key)).text(), "original");
  assert.equal(await readAttachment("new"), null);
  assert.equal(readdirSync(files.directory).length, 2, "failed generations and temporary files are reclaimed");
});

test("native reads reject same-length byte corruption without reviving a deleted record", async t => {
  const files = fixture(t), key = "corrupt";
  await storeAttachment(key, new File(["correct"], "file.txt"));
  await deleteAttachment(key);
  const deleted = files.record(key).deletedAt;
  writeFileSync(files.payload(key), "changed");
  await assert.rejects(readAttachment(key), /integrity/);
  assert.equal(files.record(key).deletedAt, deleted);
});

test("native cleanup reclaims every expired file, recovers undo references and starts orphan retention", async t => {
  const files = fixture(t), now = Date.now(), expired = now - RETENTION_MS - 1000;
  for (const key of ["expired-a", "expired-b", "restored", "recent", "unreferenced"]) {
    await storeAttachment(key, new File([key], `${key}.txt`));
    if (key !== "unreferenced") files.writeRecord(key, { ...files.record(key), deletedAt: key === "recent" ? now : expired });
  }
  cleanTrash(document("restored"));
  assert.equal(await readAttachment("expired-a"), null);
  assert.equal(await readAttachment("expired-b"), null);
  assert.equal(files.record("restored").deletedAt, undefined, "undo survives restart without first opening the attachment");
  assert.equal(files.record("recent").deletedAt, now);
  assert.ok(files.record("unreferenced").deletedAt >= now);
  const firstOrphan = files.record("unreferenced").deletedAt;
  cleanTrash(document("restored"));
  assert.equal(files.record("unreferenced").deletedAt, firstOrphan, "subsequent launches do not restart the grace period");
  assert.equal(readdirSync(files.directory).length, 6);
});

test("native cleanup reclaims old interrupted generations while preserving active and fresh files", async t => {
  const files = fixture(t), key = "active", old = new Date(Date.now() - RETENTION_MS - 1000);
  await storeAttachment(key, new File(["current"], "current.txt"));
  utimesSync(files.payload(key), old, old);
  const obsolete = join(files.directory, `${keyName(key)}.${digest("obsolete")}.bin`);
  const orphan = join(files.directory, `${keyName("missing-record")}.${digest("orphan")}.bin`);
  const fresh = join(files.directory, `${keyName("fresh")}.${digest("fresh")}.bin`);
  const temporary = `${obsolete}.12345678-1234-1234-1234-123456789abc.tmp`;
  for (const path of [obsolete, orphan, fresh, temporary]) writeFileSync(path, "file");
  for (const path of [obsolete, orphan, temporary]) utimesSync(path, old, old);
  cleanTrash(document(key));
  assert.equal(fs.existsSync(obsolete), false); assert.equal(fs.existsSync(orphan), false);
  assert.equal(fs.existsSync(temporary), false);
  assert.equal(fs.existsSync(fresh), true);
  assert.equal(await (await readAttachment(key)).text(), "current");
});

test("native purge removes all generations for exactly its storage key", async t => {
  const files = fixture(t);
  for (const key of ["remove", "keep"]) await storeAttachment(key, new File([key], "file.txt"));
  writeFileSync(join(files.directory, `${keyName("remove")}.bin`), "legacy");
  writeFileSync(join(files.directory, `${keyName("remove")}.${digest("staged")}.bin`), "staged");
  await purgeAttachment("remove");
  await purgeAttachment("remove");
  assert.equal(await readAttachment("remove"), null);
  assert.equal(await (await readAttachment("keep")).text(), "keep");
  assert.equal(readdirSync(files.directory).length, 2);
});

test("native metadata cannot direct cleanup to another attachment or an arbitrary file", async t => {
  const files = fixture(t), outside = join(files.root, "keep.txt");
  writeFileSync(outside, "keep");
  await storeAttachment("keep", new File(["important"], "keep.txt"));
  const malicious = { ...files.record("keep"), key: "different", sha256: "../../keep.txt", deletedAt: 1 };
  files.writeRecord("keep", malicious);
  assert.throws(() => cleanTrash(document()), /metadata is invalid/);
  await assert.rejects(readAttachment("keep"), /metadata is invalid/);
  assert.equal(readFileSync(outside, "utf8"), "keep");
  assert.equal(readdirSync(files.directory).length, 2);
});

test("native store rejects unreadable oversized metadata before publishing any bytes", async t => {
  const files = fixture(t);
  await assert.rejects(storeAttachment("large-name", new File(["small"], "x".repeat(65_536))), /metadata exceeds/);
  assert.deepEqual(readdirSync(files.directory), []);
});

test("native cleanup retains files referenced only by durable Microsoft mappings and interrupted moves", async t => {
  const files = fixture(t), prefix = "ferroui.todo.graph.v1.";
  const move = { sourceListId: "list", sourceTaskId: "task", source: { attachments: [{ id: "file" }] } };
  localStorage.setItem(`${prefix}device`, "device");
  localStorage.setItem(`${prefix}document-account`, "account");
  localStorage.setItem(`${prefix}account`, JSON.stringify({ version: 1, lists: {},
    tasks: { 2: { attachments: { 3: { storageKey: "mapped", baseLocal: { storage_key: "original" } } } } },
    pendingTasks: { 4: { move } }, pendingDeletes: [{ ...move, sourceTaskId: "older-task" }] }));
  const calls = [], cloudKey = (...args) => { calls.push(args); return `source:${args[2]}`; };
  const retained = synchronizationFileKeys(localStorage, cloudKey);
  assert.deepEqual([...retained].sort(), ["mapped", "original", "source:older-task", "source:task"]);
  assert.equal(calls.length, 2);
  assert.ok(calls.every(call => call[0] === "account" && call[1] === "list" && call[3].id === "file"));
  for (const key of retained) {
    await storeAttachment(key, new File([key], "file.txt"));
    files.writeRecord(key, { ...files.record(key), deletedAt: Date.now() - RETENTION_MS - 1000 });
  }
  cleanTrash(document(), retained);
  for (const key of retained) {
    assert.equal(files.record(key).deletedAt, undefined);
    assert.equal(await (await readAttachment(key)).text(), key);
  }
});

test("native cleanup refuses to infer orphanhood from unreadable or unrecovered sync metadata", t => {
  fixture(t);
  const key = "ferroui.todo.graph.v1.account";
  localStorage.setItem(key, "broken");
  assert.throws(() => synchronizationFileKeys(localStorage, () => ""), /cleanup was paused/);
  localStorage.removeItem(key);
  localStorage.setItem("ferroui.todo.graph.v1.commit", "{}");
  assert.throws(() => synchronizationFileKeys(localStorage, () => ""), /pending.*commit/);
});
