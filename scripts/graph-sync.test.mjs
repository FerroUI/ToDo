import test from "node:test";
import assert from "node:assert/strict";
import { registerHooks } from "node:module";
import { fixture, task, remoteTask } from "./tests/graph-fixture.mjs";

registerHooks({
  resolve(specifier, context, next) {
    const result = next(specifier, context);
    return result.url.endsWith("/cloud/client.js") ? { url: "fixture:graph", shortCircuit: true } : result;
  },
  load(url, context, next) {
    if (url !== "fixture:graph") return next(url, context);
    return { format: "module", shortCircuit: true, source: `
      export const segment = encodeURIComponent;
      export const listPath = id => 'me/todo/lists/' + segment(id);
      export const taskPath = (list, id) => listPath(list) + '/tasks/' + segment(id);
      export const graph = (...args) => globalThis.graphFixture.graph(...args);
      export const collection = (...args) => globalThis.graphFixture.collection(...args);
      export const readCloud = () => globalThis.graphFixture.readCloud();
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

test("a move never drops Microsoft file attachments", async () => {
  const api = fixture(); api.cloud[0].tasks.push({ ...remoteTask(), hasAttachments: true });
  await sync(api);
  api.runtime.mutate(state => {
    state.lists.push({ ...state.lists[0], id: state.next_id++, name: "Destination" });
    state.tasks[0].list_id = state.lists[1].id;
  });
  const writes = api.writes.length;
  await assert.rejects(sync(api), /Move it in Microsoft To Do/);
  assert.equal(api.writes.length, writes);
  assert.equal(api.cloud[0].tasks.length, 1);
});

test("editing another field preserves a recurrence outside the local date-anchor model", async () => {
  const api = fixture();
  const recurrence = { pattern: { type: "absoluteMonthly", interval: 1, dayOfMonth: 15 }, range: { type: "noEnd", startDate: "2026-11-21" } };
  api.cloud[0].tasks.push({ ...remoteTask(), recurrence }); await sync(api);
  assert.equal(api.runtime.state.tasks[0].recurrence, null);
  api.runtime.mutate(state => state.tasks[0].important = true); await sync(api);
  assert.deepEqual(api.cloud[0].tasks[0].recurrence, recurrence);
});
