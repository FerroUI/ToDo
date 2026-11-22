import test from "node:test";
import assert from "node:assert/strict";
import { attachFiles, checkAttachmentBudget, MAX_TASK_ATTACHMENTS as LIMIT } from "../crates/todo-browser/wwwroot/attachment-upload.js";

function fixture(existing = []) {
  const task = { id: 2, attachments: existing };
  const writes = [], removed = [];
  const runtime = {
    exportData: () => JSON.stringify({ tasks: [task] }),
    attachFile: (_id, name, type, size, storage_key) => task.attachments.push({ name, type, size, storage_key }),
  };
  const storage = { save: async file => { writes.push(file); return `file-${writes.length}`; }, remove: async key => removed.push(key) };
  return { task, runtime, storage, writes, removed };
}
const file = size => ({ name: "file.bin", type: "application/octet-stream", size });

test("a multi-file selection exceeding the task total is rejected before any blob or metadata write", async () => {
  const api = fixture([file(20 * 1024 * 1024)]);
  await assert.rejects(attachFiles(api.runtime, 2, [file(3 * 1024 * 1024), file(3 * 1024 * 1024)], api.storage), /total 25 MB/);
  assert.equal(api.writes.length, 0);
  assert.equal(api.task.attachments.length, 1);
});

test("the exact aggregate limit is accepted and an empty selection changes nothing", async () => {
  const api = fixture([file(LIMIT - 3)]);
  await attachFiles(api.runtime, 2, [file(1), file(2)], api.storage);
  await attachFiles(api.runtime, 2, [], api.storage);
  assert.equal(api.task.attachments.reduce((total, file) => total + file.size, 0), LIMIT);
  assert.equal(api.writes.length, 2);
});

test("overlapping selections account for the earlier commit without partially accepting the second", async () => {
  const api = fixture([file(LIMIT - 3)]);
  const first = attachFiles(api.runtime, 2, [file(2)], api.storage);
  const second = attachFiles(api.runtime, 2, [file(1), file(1)], api.storage);
  await first;
  await assert.rejects(second, /total 25 MB/);
  assert.equal(api.writes.length, 1);
  assert.equal(api.task.attachments.length, 2);
});

test("a rejected metadata commit discards its new blob and the queue can recover", async () => {
  const api = fixture();
  const attach = api.runtime.attachFile;
  api.runtime.attachFile = () => { throw new Error("Task was removed"); };
  await assert.rejects(attachFiles(api.runtime, 2, [file(1)], api.storage), /Task was removed/);
  assert.deepEqual(api.removed, ["file-1"]);
  assert.equal(api.task.attachments.length, 0);
  api.runtime.attachFile = attach;
  await attachFiles(api.runtime, 2, [file(1)], api.storage);
  assert.equal(api.task.attachments.length, 1);
});

test("invalid file sizes and legacy documents already over budget cannot accept another attachment", () => {
  for (const size of [-1, NaN, Infinity, 1.5, Number.MAX_SAFE_INTEGER + 1]) {
    assert.throws(() => checkAttachmentBudget([], [file(size)]), /invalid size/);
  }
  assert.throws(() => checkAttachmentBudget([file(LIMIT), file(1)], [file(0)]), /total 25 MB/);
});
