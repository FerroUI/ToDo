import test from "node:test";
import assert from "node:assert/strict";
import { exportMicrosoftArchive, restoreMicrosoftArchive } from "../crates/todo-browser/wwwroot/cloud/archive.js";
import { remoteTask } from "./tests/graph-fixture.mjs";

function sourceLists() {
  const task = remoteTask("task-one", "Original task");
  Object.assign(task, {
    status: "inProgress", importance: "low", startDateTime: { dateTime: "2026-11-20T09:00:00", timeZone: "UTC" },
    body: { contentType: "html", content: "<p>Preserve <strong>formatting</strong></p>" },
    recurrence: { pattern: { type: "relativeMonthly", interval: 2, daysOfWeek: ["monday"], index: "second" },
      range: { type: "numbered", startDate: "2026-11-01", numberOfOccurrences: 9 } },
    categories: ["Work"], lastModifiedDateTime: "2026-11-22T15:10:00Z",
    checklistItems: [{ id: "step-one", displayName: "Review source", isChecked: true }],
    linkedResources: [{ id: "link-one", applicationName: "Outlook", displayName: "Open message", webUrl: "https://outlook.live.com/mail/message/source", externalId: "message-one" }],
    hasAttachments: true, attachments: [{ id: "file-one", name: "original.bin", contentType: "application/octet-stream", size: 4,
      lastModifiedDateTime: "2026-11-22T15:10:00Z", originalProperty: "retained" }],
    unknownFutureProperty: { retain: [1, 2, 3] },
  });
  return [{ id: "default", displayName: "Tasks", wellknownListName: "defaultList", isOwner: true, isShared: false,
    unknownListProperty: "retained", tasks: [task] }];
}

function runtime() {
  let document = { schema_version: 1, tasks: [], lists: [{ id: 1, name: "Tasks" }], groups: [], next_id: 2,
    settings: { theme: "Dark", auto_sync: true, planner_enabled: true } };
  const writes = [];
  return {
    writes, get state() { return document; }, exportData: () => JSON.stringify(document),
    validateImportData(json) {
      const value = JSON.parse(json);
      assert.equal(value.schema_version, 1);
      const ids = value.lists.map(list => list.id);
      for (const task of value.tasks) {
        assert(ids.includes(task.list_id)); ids.push(task.id);
        ids.push(...task.steps.map(step => step.id), ...task.attachments.map(file => file.id));
      }
      assert.equal(new Set(ids).size, ids.length);
      assert(value.next_id > Math.max(...ids));
      return JSON.stringify(value);
    },
    importData(json) { writes.push(json); document = JSON.parse(json); },
  };
}

globalThis.DOMParser = class {
  parseFromString(html) { return { body: { textContent: html.replace(/<[^>]*>/g, "") }, querySelectorAll: () => [] }; }
};

async function archive(lists = sourceLists()) {
  return JSON.parse(await (await exportMicrosoftArchive("private-account-identity", {
    readCloud: async () => lists,
    readFile: async () => new File([new Uint8Array([0, 255, 128, 10])], "original.bin", { type: "application/octet-stream" }),
  })).text());
}

test("Microsoft export reads a fresh account snapshot, preserves raw Graph fields, and includes binary contents without account binding", async () => {
  const lists = sourceLists(), calls = [];
  const blob = await exportMicrosoftArchive("private-account-identity", {
    readCloud: async (accountId, options) => { calls.push(["read", accountId, options]); return lists; },
    readFile: async (key, accountId) => { calls.push(["file", key, accountId]); return new Blob([new Uint8Array([0, 255, 128, 10])]); },
  });
  const text = await blob.text(), value = JSON.parse(text);
  assert.equal(value.kind, "ferroui.microsoft-todo"); assert.equal(value.version, 1);
  assert.deepEqual(value.lists, lists);
  assert.deepEqual(calls[0], ["read", "private-account-identity", { fresh: true }]);
  assert.match(calls[1][1], /^graph:private-account-identity:/);
  assert.doesNotMatch(text, /private-account-identity|graph:private/);
  assert.deepEqual(value.files[0].source, { listId: "default", taskId: "task-one", attachmentId: "file-one" });
  assert.equal(value.files[0].base64, "AP+ACg==");
  assert.equal(value.files[0].key, "backup:1");
});

test("Microsoft archive restore makes fresh local entities, preserves the full raw upload seed, and restores files once", async () => {
  const value = await archive(), target = runtime(), files = new Map();
  const result = await restoreMicrosoftArchive(target, JSON.stringify(value), { storeFile: async (key, file) => files.set(key, file) });
  assert.deepEqual(result, { tasks: 1, files: 1, legacy: false }); assert.equal(target.writes.length, 1);
  const task = target.state.tasks[0];
  assert.equal(task.title, "Original task"); assert.equal(task.notes, "Preserve formatting");
  assert.equal(task.important, false); assert.equal(task.completed, false); assert.equal(task.recurrence, null);
  assert.deepEqual(task.microsoft_data, value.lists[0].tasks[0], "HTML, unsupported recurrence, status, start time and extensions survive in the raw seed");
  assert.equal(task.steps[0].completed, true); assert.equal(task.linked_resources[0].application_name, "Outlook");
  assert.equal(task.created_on, "2026-11-21"); assert.equal(task.updated_on, "2026-11-22");
  assert.equal(target.state.settings.theme, "Dark"); assert.equal(target.state.settings.auto_sync, false); assert.equal(target.state.settings.planner_enabled, false);
  assert.equal(target.state.lists[0].cloud, undefined); assert.equal(task.external, undefined); assert.equal(task.flagged_email, false);
  const stored = files.get(task.attachments[0].storage_key);
  assert.deepEqual([...new Uint8Array(await stored.arrayBuffer())], [0, 255, 128, 10]);
});

test("flagged email and shared lists restore as ordinary local lists with source links retained", async () => {
  const lists = sourceLists();
  lists[0].wellknownListName = "flaggedEmails"; lists[0].displayName = "Flagged email"; lists[0].isOwner = false; lists[0].isShared = true;
  const value = await archive(lists), target = runtime();
  await restoreMicrosoftArchive(target, JSON.stringify(value), { storeFile: async () => {} });
  assert.deepEqual(target.state.lists.map(list => list.name), ["Tasks", "Flagged email"]);
  assert.equal(target.state.tasks[0].list_id, target.state.lists[1].id);
  assert.equal(target.state.tasks[0].flagged_email, false);
  assert.match(target.state.tasks[0].linked_resources[0].web_url, /^https:\/\/outlook/);
});

test("duplicate source IDs, partial collections and orphan file mappings fail before writing local data", async () => {
  const value = await archive();
  const edits = [
    copy => copy.version = 99,
    copy => copy.accountId = "must-not-be-a-wrapper-field",
    copy => copy.lists.push(structuredClone(copy.lists[0])),
    copy => copy.lists[0].tasks.push(structuredClone(copy.lists[0].tasks[0])),
    copy => copy.lists[0].tasks[0].checklistItems.push(structuredClone(copy.lists[0].tasks[0].checklistItems[0])),
    copy => delete copy.lists[0].tasks[0].attachments,
    copy => copy.files[0].source.taskId = "orphan",
    copy => copy.files[0].size = 2,
    copy => copy.files[0].sha256 = "0".repeat(64),
    copy => copy.files.push(structuredClone(copy.files[0])),
  ];
  for (const edit of edits) {
    const copy = structuredClone(value); edit(copy);
    const target = runtime(), before = target.exportData(); let files = 0;
    await assert.rejects(restoreMicrosoftArchive(target, JSON.stringify(copy), { storeFile: async () => files++ }));
    assert.equal(target.exportData(), before); assert.equal(files, 0); assert.equal(target.writes.length, 0);
  }
});

test("an incomplete Microsoft file aborts export and oversized account content is rejected before download", async () => {
  await assert.rejects(exportMicrosoftArchive("account", { readCloud: async () => sourceLists(), readFile: async () => new Blob(["short"]) }), /missing or incomplete/);
  const lists = sourceLists(), task = lists[0].tasks[0];
  task.attachments = Array.from({ length: 6 }, (_, index) => ({ id: `file-${index}`, name: `file-${index}`, size: 25 * 1024 * 1024 }));
  let reads = 0;
  await assert.rejects(exportMicrosoftArchive("account", { readCloud: async () => lists, readFile: async () => reads++ }), /128 MB/);
  assert.equal(reads, 0);
});

test("an account with no lists restores an empty default Tasks list", async () => {
  const value = await archive([]), target = runtime();
  assert.deepEqual(await restoreMicrosoftArchive(target, JSON.stringify(value)), { tasks: 0, files: 0, legacy: false });
  assert.deepEqual(target.state.lists.map(list => [list.id, list.name]), [[1, "Tasks"]]);
});
