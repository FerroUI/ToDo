import test from "node:test";
import assert from "node:assert/strict";
import { registerHooks } from "node:module";
import { fixture, task, remoteTask } from "./tests/graph-fixture.mjs";

registerHooks({
  resolve(specifier, context, next) {
    const result = next(specifier, context);
    if (result.url.endsWith("/cloud/client.js")) return { url: "fixture:graph", shortCircuit: true };
    if (result.url.endsWith("/wwwroot/attachments.js")) return { url: "fixture:files", shortCircuit: true };
    return result;
  },
  load(url, context, next) {
    if (url === "fixture:files") return { format: "module", shortCircuit: true, source: `
      export const readAttachment = async key => globalThis.graphFixture.files.get(key) || null;
      export const storeAttachment = async (key,file) => globalThis.graphFixture.files.set(key,file);
      export const downloadBlob = () => {};
    ` };
    if (url !== "fixture:graph") return next(url, context);
    return { format: "module", shortCircuit: true, source: `
      export const segment = encodeURIComponent;
      export const listPath = id => 'me/todo/lists/' + segment(id);
      export const taskPath = (list, id) => listPath(list) + '/tasks/' + segment(id);
      export const graph = (...args) => globalThis.graphFixture.graph(...args);
      export const collection = (...args) => globalThis.graphFixture.collection(...args);
      export const readCloud = () => globalThis.graphFixture.readCloud();
      export const cloudRevision = () => globalThis.graphFixture.writes.length;
      export const readList = async id => (await globalThis.graphFixture.readCloud()).find(list=>list.id===id);
      export const hydrateTask = (list,id) => globalThis.graphFixture.graph(taskPath(list,id));
      export const graphBytes = (...args) => globalThis.graphFixture.graphBytes(...args);
      export const graphUpload = (...args) => globalThis.graphFixture.graphUpload(...args);
      export const graphUrl = path => new URL(path,'https://graph.microsoft.com/v1.0/');
    ` };
  },
});
const { synchronize } = await import("../crates/todo-browser/wwwroot/cloud/sync.js");
const sync = api => synchronize(api.runtime, "account-one", "2026-11-21");

test("creates tasks and checklist items once, then updates the mapped task", async () => {
  const local = task(); local.steps = [{ id: 3, title: "First step", completed: false }];
  const api = fixture([local]);
  await sync(api);
  assert.equal(api.cloud[0].tasks.length, 1);
  assert.equal(api.cloud[0].tasks[0].checklistItems.length, 1);
  await sync(api);
  assert.equal(api.cloud[0].tasks.length, 1);
  assert.equal(api.cloud[0].tasks[0].checklistItems.length, 1);
  api.runtime.mutate(state => state.tasks[0].title = "Changed title");
  await sync(api);
  assert.equal(api.cloud[0].tasks[0].title, "Changed title");
});

test("imports remote tasks and merges different fields while preserving My Day", async () => {
  const api = fixture(); api.cloud[0].tasks.push(remoteTask());
  await sync(api);
  api.runtime.mutate(state => { state.tasks[0].important = true; state.tasks[0].my_day = "2026-11-21"; });
  api.cloud[0].tasks[0].title = "A remote rename";
  await sync(api);
  assert.equal(api.runtime.state.tasks[0].title, "A remote rename");
  assert.equal(api.runtime.state.tasks[0].my_day, "2026-11-21");
  assert.equal(api.cloud[0].tasks[0].importance, "high");
});

test("cloud synchronization preserves the local-only sidebar order", async () => {
  const api = fixture([task()]);
  const order = [5, 4];
  api.runtime.mutate(state => {
    state.lists.push({ ...state.lists[0], id: 4, name: "Projects" });
    state.groups.push({ id: 5, name: "Work", position: 0, collapsed: false });
    state.sidebar_order = [...order]; state.next_id = 6;
  });
  await sync(api);
  api.cloud[0].tasks[0].title = "Remote update";
  await sync(api);
  assert.deepEqual(api.runtime.state.sidebar_order, order);
});

test("same-field conflicts stop before any remote mutation", async () => {
  const api = fixture([task()]); await sync(api);
  api.runtime.mutate(state => state.tasks[0].title = "Local edit");
  api.cloud[0].tasks[0].title = "Remote edit";
  const writes = api.writes.length;
  await assert.rejects(sync(api), /Sync conflict/);
  assert.equal(api.writes.length, writes);
  assert.equal(api.runtime.state.tasks[0].title, "Local edit");
  assert.equal(api.cloud[0].tasks[0].title, "Remote edit");
});

test("deletions propagate in either direction without importing a deleted task again", async () => {
  const api = fixture([task()]); await sync(api);
  api.runtime.mutate(state => state.tasks = []); await sync(api);
  assert.equal(api.cloud[0].tasks.length, 0);
  assert.equal(api.runtime.state.tasks.length, 0);
  api.cloud[0].tasks.push(remoteTask()); await sync(api);
  api.cloud[0].tasks = []; await sync(api);
  assert.equal(api.runtime.state.tasks.length, 0);
});

test("creates lists, imports remote lists, and does not duplicate either on a second sync", async () => {
  const api = fixture();
  api.runtime.mutate(state => { state.lists.push({ ...state.lists[0], id: 2, name: "Local list" }); state.next_id = 3; });
  api.cloud.push({ id: "remote-list", displayName: "Remote list", wellknownListName: "none", isOwner: true, tasks: [] });
  await sync(api); await sync(api);
  assert.deepEqual(api.runtime.state.lists.map(list => list.name).sort(), ["Local list", "Remote list", "Tasks"]);
  assert.equal(api.cloud.length, 3);
});

test("retains concurrent local edits and resumes from a checkpoint", async () => {
  const api = fixture([task()]); await sync(api);
  api.runtime.mutate(state => state.tasks[0].title = "First edit");
  let once = false;
  api.afterWrite = () => { if (!once) { once = true; api.runtime.mutate(state => state.tasks[0].title = "Newer edit"); } };
  await assert.rejects(sync(api), /changed on this device/);
  assert.equal(api.runtime.state.tasks[0].title, "Newer edit");
  api.afterWrite = undefined;
  await sync(api);
  assert.equal(api.cloud[0].tasks[0].title, "Newer edit");
  assert.equal(api.cloud[0].tasks.length, 1);
});

test("an uncertain task POST is recovered through its linked resource without duplicates", async () => {
  const local = task(); local.steps = [{ id: 3, title: "Keep this step", completed: false }];
  const api = fixture([local]);
  api.afterWrite = ({ method, path }) => { if (method === "POST" && path.endsWith("/tasks")) throw new Error("Network disconnected after the server wrote"); };
  await assert.rejects(sync(api), /Network disconnected/);
  api.afterWrite = undefined;
  await sync(api);
  assert.equal(api.cloud[0].tasks.length, 1);
  assert.equal(api.runtime.state.tasks[0].steps[0].title, "Keep this step");
  assert.equal(api.cloud[0].tasks[0].checklistItems.length, 1);
});

test("does not upload a prior account's local document into another account", async () => {
  const api = fixture([task()]); await sync(api);
  const writes = api.writes.length;
  await assert.rejects(synchronize(api.runtime, "another-account", "2026-11-21"), /linked to another/);
  assert.equal(api.writes.length, writes);
});

test("step updates and removals propagate without replacing local task metadata", async () => {
  const local = task(); local.steps = [{ id: 3, title: "Step", completed: false }];
  const api = fixture([local]); await sync(api);
  api.runtime.mutate(state => state.tasks[0].steps[0].completed = true);
  await sync(api);
  assert.equal(api.cloud[0].tasks[0].checklistItems[0].isChecked, true);
  api.cloud[0].tasks[0].checklistItems[0].displayName = "Remote step title";
  await sync(api);
  assert.equal(api.runtime.state.tasks[0].steps[0].title, "Remote step title");
  api.runtime.mutate(state => state.tasks[0].steps = []); await sync(api);
  assert.equal(api.cloud[0].tasks[0].checklistItems.length, 0);
});

test("recovers a checklist POST whose response was lost", async () => {
  const local = task(); local.steps = [{ id: 3, title: "Keep once", completed: false }];
  const api = fixture([local]);
  api.afterWrite = ({ method, path }) => { if (method === "POST" && path.endsWith("/checklistItems")) throw new Error("Response lost"); };
  await assert.rejects(sync(api), /Response lost/);
  api.afterWrite = undefined; await sync(api);
  assert.equal(api.cloud[0].tasks[0].checklistItems.length, 1);
  assert.equal(api.runtime.state.tasks[0].steps.length, 1);
});

test("moves a task with its checklist only after the destination exists", async () => {
  const local = task(); local.steps = [{ id: 3, title: "Move with task", completed: false }];
  const api = fixture([local]); await sync(api);
  api.runtime.mutate(state => {
    state.lists.push({ ...state.lists[0], id: 4, name: "Destination" }); state.next_id = 5;
    state.tasks[0].list_id = 4;
  });
  await sync(api);
  assert.equal(api.cloud[0].tasks.length, 0);
  assert.equal(api.cloud[1].tasks.length, 1);
  assert.equal(api.cloud[1].tasks[0].checklistItems[0].displayName, "Move with task");
});

test("a cloud deletion does not discard a concurrent local-only My Day edit", async () => {
  const api = fixture([task()]); await sync(api);
  api.runtime.mutate(state => state.tasks[0].my_day = "2026-11-21");
  api.cloud[0].tasks = [];
  await assert.rejects(sync(api), /changed here and was deleted/);
  assert.equal(api.runtime.state.tasks[0].my_day, "2026-11-21");
});

test("a task move recovers a lost create response before deleting the source", async () => {
  const local = task(); local.steps = [{ id: 3, title: "Keep step", completed: false }];
  const api = fixture([local]); await sync(api);
  api.runtime.mutate(state => {
    state.lists.push({ ...state.lists[0], id: 4, name: "Destination" }); state.next_id = 5;
    state.tasks[0].list_id = 4;
  });
  api.afterWrite = ({ method, path }) => { if (method === "POST" && path.endsWith("/tasks")) throw new Error("Move response lost"); };
  await assert.rejects(sync(api), /Move response lost/);
  assert.equal(api.cloud[0].tasks.length, 1);
  api.afterWrite = undefined; await sync(api);
  assert.equal(api.cloud[0].tasks.length, 0);
  assert.equal(api.cloud[1].tasks.length, 1);
  assert.equal(api.cloud[1].tasks[0].checklistItems[0].displayName, "Keep step");
});

test("sync preserves a reminder's delivered state until its schedule changes", async () => {
  const local = task(); local.reminder = { date: "2026-11-21", minutes_after_midnight: 540, notified: true };
  const api = fixture([local]); await sync(api); await sync(api);
  assert.equal(api.runtime.state.tasks[0].reminder.notified, true);
  api.cloud[0].tasks[0].reminderDateTime.dateTime = "2026-11-22T09:00:00";
  await sync(api);
  assert.equal(api.runtime.state.tasks[0].reminder.notified, false);
});

test("a list gaining a new remote task during sync is never deleted", async () => {
  const api = fixture([task()]);
  api.cloud.push({ id: "remote-list", displayName: "Remote list", wellknownListName: "none", isOwner: true, tasks: [] });
  await sync(api);
  api.runtime.mutate(state => { state.lists = state.lists.filter(list => list.id === 1); state.tasks[0].title = "Trigger remote write"; });
  api.afterWrite = () => { api.cloud[1].tasks.push(remoteTask("arrived-during-sync")); };
  await assert.rejects(sync(api), /changed in Microsoft while syncing/);
  assert.equal(api.cloud[1].tasks[0].id, "arrived-during-sync");
  assert.equal(api.writes.filter(write => write.method === "DELETE" && write.path.endsWith("remote-list")).length, 0);
});

test("moving a task preserves cloud-only writable properties and linked resources", async () => {
  const api = fixture();
  api.cloud[0].tasks.push({ ...remoteTask(), status: "inProgress", startDateTime: { dateTime: "2026-11-20T00:00:00", timeZone: "UTC" },
    linkedResources: [{ id: "external-link", webUrl: "https://example.test/original", applicationName: "Another app", displayName: "Original", externalId: "original-id" }] });
  await sync(api);
  api.runtime.mutate(state => {
    state.lists.push({ ...state.lists[0], id: state.next_id++, name: "Destination" });
    state.tasks[0].list_id = state.lists[1].id;
  });
  await sync(api);
  assert.equal(api.cloud[1].tasks[0].status, "inProgress");
  assert.equal(api.cloud[1].tasks[0].startDateTime.dateTime, "2026-11-20T00:00:00");
  assert.equal(api.cloud[1].tasks[0].linkedResources[0].externalId, "original-id");
});

test("a move copies and verifies Microsoft files before deleting the source", async () => {
  const api = fixture(); api.cloud[0].tasks.push({ ...remoteTask(), hasAttachments: true, attachments: [
    { id: "cloud-file", name: "keep.txt", contentType: "text/plain", size: 3, contentBytes: "YWJj" },
  ] });
  await sync(api);
  api.runtime.mutate(state => {
    state.lists.push({ ...state.lists[0], id: state.next_id++, name: "Destination" });
    state.tasks[0].list_id = state.lists[1].id;
  });
  await sync(api);
  assert.equal(api.cloud[0].tasks.length, 0);
  assert.equal(api.cloud[1].tasks[0].attachments[0].contentBytes, "YWJj");
  assert.match(api.runtime.state.tasks[0].attachments[0].storage_key, new RegExp(`^graph:account-one:${api.cloud[1].id}:${api.cloud[1].tasks[0].id}:`));
  const upload = api.writes.findIndex(write => write.method === "POST" && write.path.endsWith("/attachments"));
  const remove = api.writes.findIndex(write => write.method === "DELETE" && write.path.endsWith("/remote-task"));
  assert(upload >= 0 && remove > upload);
});

test("editing another field preserves a recurrence outside the local date-anchor model", async () => {
  const api = fixture();
  const recurrence = { pattern: { type: "absoluteMonthly", interval: 1, dayOfMonth: 15 }, range: { type: "noEnd", startDate: "2026-11-21" } };
  api.cloud[0].tasks.push({ ...remoteTask(), recurrence }); await sync(api);
  assert.equal(api.runtime.state.tasks[0].recurrence, null);
  api.runtime.mutate(state => state.tasks[0].important = true); await sync(api);
  assert.deepEqual(api.cloud[0].tasks[0].recurrence, recurrence);
});

test("Planner tasks stay in the local document without uploading to To Do", async () => {
  const external = task(2, "Assigned Planner task"); external.external = { provider: "Planner", id: "plan-task", url: "https://planner.cloud.microsoft/" };
  const api = fixture([external]); await sync(api);
  assert.equal(api.cloud[0].tasks.length, 0);
  assert.deepEqual(api.runtime.state.tasks[0], external);
});

test("source links and shared-list capabilities are imported without exposing correlation links", async () => {
  const api = fixture(); api.cloud[0].tasks.push({ ...remoteTask(), linkedResources: [
    { id: "email", applicationName: "Outlook", displayName: "Open in Outlook", webUrl: "https://outlook.live.com/mail/item", externalId: "source" },
    { id: "invalid", applicationName: "Other", webUrl: "javascript:alert(1)" },
    { id: "own", applicationName: "FerroUI To Do", webUrl: "https://example.test/ToDo/" },
  ] });
  api.cloud.push({ id: "shared", displayName: "Shared", wellknownListName: "none", isShared: true, isOwner: false, tasks: [] });
  await sync(api);
  assert.deepEqual(api.runtime.state.tasks[0].linked_resources.map(link => [link.id, link.web_url]), [["email", "https://outlook.live.com/mail/item"], ["invalid", ""]]);
  assert.deepEqual(api.runtime.state.lists.find(list => list.name === "Shared").cloud, { is_owner: false, is_shared: true, well_known_name: "none" });
  api.runtime.mutate(state => state.lists.find(list => list.name === "Shared").name = "Renamed");
  await assert.rejects(sync(api), /cannot be renamed/);
});

test("cloud attachment metadata imports once with an account-bound download reference", async () => {
  const api = fixture(); api.cloud[0].tasks.push({ ...remoteTask(), hasAttachments: true, attachments: [
    { id: "file/1", name: "notes.txt", contentType: "text/plain", size: 3, contentBytes: "YWJj" },
  ] });
  await sync(api); const first = structuredClone(api.runtime.state.tasks[0].attachments);
  assert.equal(first.length, 1); assert.match(first[0].storage_key, /^graph:account-one:default:remote-task:file%2F1:/);
  await sync(api); assert.deepEqual(api.runtime.state.tasks[0].attachments, first);
  api.runtime.mutate(state => state.tasks[0].attachments = []);
  await sync(api); assert.deepEqual(api.cloud[0].tasks[0].attachments, []);
});

test("local attachments upload once and uncertain POST responses recover by content digest", async () => {
  const local = task(); local.attachments.push({ id: 3, name: "notes.txt", mime_type: "text/plain", size: 3, storage_key: "local-file" });
  const api = fixture([local]); api.runtime.mutate(state => state.next_id = 4);
  api.files.set("local-file", new File(["abc"], "notes.txt", { type: "text/plain" }));
  api.afterWrite = ({ method, path }) => { if (method === "POST" && path.endsWith("/attachments")) throw new Error("Upload response lost"); };
  await assert.rejects(sync(api), /Upload response lost/);
  assert.equal(api.cloud[0].tasks[0].attachments.length, 1);
  api.afterWrite = undefined; await sync(api); await sync(api);
  assert.equal(api.cloud[0].tasks[0].attachments.length, 1);
  assert.equal(api.runtime.state.tasks[0].attachments[0].storage_key, "local-file");
});

test("attachment removal conflicts stop before remote writes and cloud additions protect task deletion", async () => {
  const api = fixture(); api.cloud[0].tasks.push({ ...remoteTask(), hasAttachments: true, attachments: [{ id: "file", name: "one.txt", size: 3, contentType: "text/plain" }] });
  await sync(api);
  api.runtime.mutate(state => state.tasks[0].attachments = []);
  api.cloud[0].tasks[0].attachments[0].name = "changed.txt";
  const writes = api.writes.length;
  await assert.rejects(sync(api), /deleted here and changed/); assert.equal(api.writes.length, writes);
  api.runtime.mutate(state => state.tasks = []);
  await assert.rejects(sync(api), /deleted here and changed/); assert.equal(api.writes.length, writes);
});

test("unchanged synchronization preserves the mounted document", async () => {
  const api = fixture([task()]); await sync(api);
  const replace = api.runtime.replaceFromCloud; let replacements = 0;
  api.runtime.replaceFromCloud = (...args) => { replacements++; return replace(...args); };
  assert.equal(await sync(api), false);
  assert.equal(replacements, 0);
});

test("a file upload interrupted during a move retains the source and resumes without duplicates", async () => {
  const api = fixture(); api.cloud[0].tasks.push({ ...remoteTask(), hasAttachments: true, attachments: [
    { id: "file", name: "keep.txt", contentType: "text/plain", size: 3, contentBytes: "YWJj" },
  ] });
  await sync(api);
  api.runtime.mutate(state => { state.lists.push({ ...state.lists[0], id: state.next_id++, name: "Destination", cloud: null }); state.tasks[0].list_id = state.lists[1].id; });
  api.afterWrite = ({ method, path }) => { if (method === "POST" && path.endsWith("/attachments")) throw new Error("Copy response lost"); };
  await assert.rejects(sync(api), /Copy response lost/);
  assert.equal(api.cloud[0].tasks.length, 1); assert.equal(api.cloud[1].tasks.length, 1);
  api.afterWrite = undefined; await sync(api);
  assert.equal(api.cloud[0].tasks.length, 0); assert.equal(api.cloud[1].tasks.length, 1);
  assert.equal(api.cloud[1].tasks[0].attachments.length, 1);
  assert.equal(api.cloud[1].tasks[0].attachments[0].contentBytes, "YWJj");
});

test("a source file changing during a move preserves both tasks instead of deleting new content", async () => {
  const api = fixture(); api.cloud[0].tasks.push({ ...remoteTask(), hasAttachments: true, attachments: [
    { id: "file", name: "keep.txt", contentType: "text/plain", size: 3, contentBytes: "YWJj" },
  ] });
  await sync(api);
  api.runtime.mutate(state => { state.lists.push({ ...state.lists[0], id: state.next_id++, name: "Destination", cloud: null }); state.tasks[0].list_id = state.lists[1].id; });
  api.afterWrite = ({ method, path }) => { if (method === "POST" && path.endsWith("/attachments")) api.cloud[0].tasks[0].attachments[0].contentBytes = "eHl6"; };
  await assert.rejects(sync(api), /original task changed.*Both copies/);
  assert.equal(api.cloud[0].tasks[0].attachments[0].contentBytes, "eHl6");
  assert.equal(api.cloud[1].tasks[0].attachments[0].contentBytes, "YWJj");
  assert(!api.writes.some(write => write.method === "DELETE" && write.path.endsWith("/remote-task")));
});

test("a Microsoft archive seed preserves unedited raw fields when creating an independent copy", async () => {
  const local = task();
  const recurrence = { pattern: { type: "relativeMonthly", daysOfWeek: ["monday"], index: "first", interval: 1 }, range: { type: "endDate", startDate: "2026-11-21", endDate: "2027-11-21" } };
  local.microsoft_data = { ...remoteTask("original", local.title), status: "inProgress", startDateTime: { dateTime: "2026-11-20T00:00:00", timeZone: "UTC" }, recurrence };
  const api = fixture([local]); await sync(api);
  assert.equal(api.cloud[0].tasks[0].status, "inProgress");
  assert.deepEqual(api.cloud[0].tasks[0].recurrence, recurrence);
  assert.deepEqual(api.cloud[0].tasks[0].startDateTime, local.microsoft_data.startDateTime);
  assert.notEqual(api.cloud[0].tasks[0].id, "original");
});

test("an interrupted later upload cannot turn a draft-only remote file import into a deletion", async () => {
  const api = fixture([task()]); await sync(api);
  api.cloud[0].tasks[0].hasAttachments = true;
  api.cloud[0].tasks[0].attachments.push({ id: "new-file", name: "keep.txt", contentType: "text/plain", size: 3, contentBytes: "YWJj" });
  api.runtime.mutate(state => state.tasks.push(task(state.next_id++, "Another task")));
  api.afterWrite = ({ method, path }) => { if (method === "POST" && path.endsWith("/tasks")) throw new Error("Later task response lost"); };
  await assert.rejects(sync(api), /Later task response lost/);
  assert.equal(api.runtime.state.tasks[0].attachments.length, 0);
  api.afterWrite = undefined; await sync(api);
  assert.equal(api.cloud[0].tasks[0].attachments[0].id, "new-file");
  assert.equal(api.runtime.state.tasks[0].attachments.length, 1);
  assert(!api.writes.some(write => write.method === "DELETE" && write.path.endsWith("/new-file")));
});

test("a multi-file move resumes after an earlier file already changed its cache key", async () => {
  const api = fixture(); api.cloud[0].tasks.push({ ...remoteTask(), hasAttachments: true, attachments: [
    { id: "one", name: "one.txt", contentType: "text/plain", size: 3, contentBytes: "YWJj" },
    { id: "two", name: "two.txt", contentType: "text/plain", size: 3, contentBytes: "eHl6" },
  ] });
  await sync(api);
  api.runtime.mutate(state => { state.lists.push({ ...state.lists[0], id: state.next_id++, name: "Destination", cloud: null }); state.tasks[0].list_id = state.lists[1].id; });
  let uploads = 0;
  api.afterWrite = ({ method, path }) => { if (method === "POST" && path.endsWith("/attachments") && ++uploads === 2) throw new Error("Second copy response lost"); };
  await assert.rejects(sync(api), /Second copy response lost/);
  assert.equal(api.cloud[0].tasks.length, 1);
  assert(api.runtime.state.tasks[0].attachments.every(file => file.storage_key.includes(":default:remote-task:")));
  api.afterWrite = undefined; await sync(api); await sync(api);
  assert.equal(api.cloud[0].tasks.length, 0);
  assert.deepEqual(api.cloud[1].tasks[0].attachments.map(file => file.contentBytes), ["YWJj", "eHl6"]);
  assert.equal(api.writes.filter(write => write.method === "POST" && write.path.endsWith("/attachments")).length, 2);
  const prefix = `graph:account-one:${api.cloud[1].id}:${api.cloud[1].tasks[0].id}:`;
  assert(api.runtime.state.tasks[0].attachments.every(file => file.storage_key.startsWith(prefix)));
});

test("move recovery copies a remote-added file and reallocates abandoned draft IDs", async () => {
  const api = fixture(); api.cloud[0].tasks.push({ ...remoteTask(), hasAttachments: true, attachments: [
    { id: "one", name: "one.txt", contentType: "text/plain", size: 3, contentBytes: "YWJj" },
  ] });
  await sync(api);
  api.cloud[0].tasks[0].attachments.push({ id: "new", name: "new.txt", contentType: "text/plain", size: 3, contentBytes: "eHl6" });
  api.runtime.mutate(state => { state.lists.push({ ...state.lists[0], id: state.next_id++, name: "Destination", cloud: null }); state.tasks[0].list_id = state.lists[1].id; });
  api.afterWrite = ({ method, path }) => { if (method === "POST" && path.endsWith("/attachments")) throw new Error("First copy response lost"); };
  await assert.rejects(sync(api), /First copy response lost/);
  assert.equal(api.cloud[0].tasks[0].attachments.length, 2);
  api.runtime.mutate(state => state.tasks.push(task(state.next_id++, "Created between retries")));
  api.afterWrite = undefined; await sync(api);
  assert.equal(api.cloud[0].tasks.length, 1);
  assert.equal(api.cloud[0].tasks[0].title, "Created between retries");
  assert.deepEqual(api.cloud[1].tasks[0].attachments.map(file => file.contentBytes), ["YWJj", "eHl6"]);
  assert.equal(api.runtime.state.tasks[0].attachments.length, 2);
  const entities = [...api.runtime.state.lists, ...api.runtime.state.tasks, ...api.runtime.state.tasks.flatMap(task => [...task.steps, ...task.attachments])];
  assert.equal(new Set(entities.map(item => item.id)).size, entities.length);
  const uploads = api.writes.filter(write => write.method === "POST" && write.path.endsWith("/attachments"));
  assert.equal(uploads.length, 2);
  const removal = api.writes.findIndex(write => write.method === "DELETE" && write.path.endsWith("/remote-task"));
  assert(removal > api.writes.findIndex(write => write === uploads[1]));
});

test("an interrupted move preserves remote checklist edits and additions alongside newer local edits", async () => {
  const api = fixture(); api.cloud[0].tasks.push({ ...remoteTask(), checklistItems: [{ id: "existing", displayName: "Original step", isChecked: false }] });
  await sync(api);
  api.cloud[0].tasks[0].checklistItems[0].displayName = "Renamed in Microsoft";
  api.cloud[0].tasks[0].checklistItems.push({ id: "new", displayName: "Added in Microsoft", isChecked: false });
  api.runtime.mutate(state => { state.lists.push({ ...state.lists[0], id: state.next_id++, name: "Destination", cloud: null }); state.tasks[0].list_id = state.lists[1].id; });
  let posts = 0;
  api.afterWrite = ({ method, path }) => { if (method === "POST" && path.endsWith("/checklistItems") && ++posts === 2) throw new Error("Second step response lost"); };
  await assert.rejects(sync(api), /Second step response lost/);
  api.runtime.mutate(state => { state.tasks[0].steps[0].completed = true; state.tasks.push(task(state.next_id++, "New task during interruption")); });
  api.afterWrite = undefined; await sync(api);
  assert.deepEqual(api.cloud[1].tasks[0].checklistItems.map(step => [step.displayName, step.isChecked]), [["Renamed in Microsoft", true], ["Added in Microsoft", false]]);
  assert.deepEqual(api.runtime.state.tasks[0].steps.map(step => [step.title, step.completed]), [["Renamed in Microsoft", true], ["Added in Microsoft", false]]);
  assert.equal(api.writes.filter(write => write.method === "POST" && write.path.endsWith("/checklistItems")).length, 2);
  const entities = [...api.runtime.state.lists, ...api.runtime.state.tasks, ...api.runtime.state.tasks.flatMap(task => [...task.steps, ...task.attachments])];
  assert.equal(new Set(entities.map(item => item.id)).size, entities.length);
});

test("checkpoints retain pending step and file deletions across another task's interruption", async () => {
  const first = task(2), second = task(3, "Second task");
  second.steps = [{ id: 4, title: "Remove step", completed: false }];
  second.attachments = [{ id: 5, name: "remove.txt", mime_type: "text/plain", size: 3, storage_key: "local-file" }];
  const api = fixture([first, second]); api.files.set("local-file", new File(["abc"], "remove.txt", { type: "text/plain" }));
  await sync(api);
  api.runtime.mutate(state => { state.tasks[0].title = "First edit"; state.tasks[1].steps = []; state.tasks[1].attachments = []; });
  api.afterWrite = ({ method }) => { if (method === "PATCH") api.runtime.mutate(state => state.tasks[0].title = "Newer edit"); };
  await assert.rejects(sync(api), /changed on this device/);
  assert.equal(api.cloud[0].tasks[1].checklistItems.length, 1); assert.equal(api.cloud[0].tasks[1].attachments.length, 1);
  api.afterWrite = undefined; await sync(api);
  assert.equal(api.cloud[0].tasks[1].checklistItems.length, 0); assert.equal(api.cloud[0].tasks[1].attachments.length, 0);
});

test("unchanged polling does not serialize a metadata checkpoint for each task and step", async () => {
  let next = 2;
  const tasks = Array.from({ length: 20 }, () => {
    const local = task(next++, `Task ${next}`);
    local.steps = Array.from({ length: 3 }, () => ({ id: next++, title: `Step ${next}`, completed: false }));
    return local;
  });
  const api = fixture(tasks); await sync(api);
  const read = localStorage.getItem.bind(localStorage), metadataKey = "ferroui.todo.graph.v1.account-one";
  let metadataReads = 0;
  localStorage.getItem = key => { if (key === metadataKey) metadataReads++; return read(key); };
  const writes = api.writes.length;
  assert.equal(await sync(api), false);
  assert.equal(api.writes.length, writes);
  // One metadata load and one final equality check. Every checkpoint would
  // clone/serialize the entire sidecar and perform another comparison read.
  assert.equal(metadataReads, 2);
});
