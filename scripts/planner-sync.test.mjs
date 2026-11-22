import test from "node:test";
import assert from "node:assert/strict";
import { registerHooks } from "node:module";
import { fixture } from "./tests/graph-fixture.mjs";

registerHooks({
  resolve(specifier, context, next) {
    const result = next(specifier, context);
    return result.url.endsWith("/cloud/client.js") ? { url: "fixture:planner", shortCircuit: true } : result;
  },
  load(url, context, next) {
    if (url !== "fixture:planner") return next(url, context);
    return { format: "module", shortCircuit: true, source: `
      export const graph = (...args) => globalThis.plannerFixture.graph(...args);
      export const collection = (...args) => globalThis.plannerFixture.collection(...args);
    ` };
  },
});
const { synchronizePlanner } = await import("../crates/todo-browser/wwwroot/cloud/planner.js");
const sync = (api, options) => synchronizePlanner(api.runtime, "account-one", "2026-11-22", options);

function plannerFixture() {
  const { runtime } = fixture();
  const tasks = [{ id: "planner-one", planId: "plan-one", title: "Assigned task", percentComplete: 50,
    dueDateTime: "2026-11-23T17:30:00Z", createdDateTime: "2026-11-20T10:00:00Z", "@odata.etag": 'W/"task-1"' }];
  const details = new Map([[tasks[0].id, { description: "Team notes", checklist: {
    "step-one": { title: "Review", isChecked: false, orderHint: "a" },
  }, references: { "https%3A//example%2Etest/document": { alias: "Specification" }, "javascript%3Aalert(1)": { alias: "Invalid" } }, "@odata.etag": 'W/"details-1"' }]]);
  let revision = 2;
  const api = { runtime, tasks, details, writes: [], reads: 0, afterWrite: null, collectionError: null,
    async collection(path) {
      assert.equal(path, "me/planner/tasks"); api.reads++;
      if (api.collectionError) throw api.collectionError;
      return structuredClone(tasks);
    },
    async graph(path, method = "GET", body, etag) {
      const parts = path.split("/").map(decodeURIComponent), id = parts[2];
      assert.equal(parts[0], "planner"); assert.equal(parts[1], "tasks");
      const item = parts[3] === "details" ? details.get(id) : tasks.find(task => task.id === id);
      assert(item, `fixture object exists for ${path}`);
      if (method === "GET") { api.reads++; return structuredClone(item); }
      assert.equal(method, "PATCH", "Planner projections never create or delete service tasks");
      if (etag !== item["@odata.etag"]) throw Object.assign(new Error("Precondition failed"), { status: 412 });
      api.writes.push({ path, body: structuredClone(body), etag });
      if (body.checklist) for (const [id, step] of Object.entries(body.checklist)) {
        if (step === null) delete item.checklist[id];
        else item.checklist[id] = { ...item.checklist[id], ...step };
      }
      Object.assign(item, Object.fromEntries(Object.entries(body).filter(([key]) => key !== "checklist")));
      item["@odata.etag"] = `W/"revision-${revision++}"`;
      if (api.afterWrite) await api.afterWrite({ path, body });
      return null;
    },
  };
  globalThis.plannerFixture = api;
  return api;
}

test("imports assigned Planner tasks, checklist and safe sources without copying them into To Do", async () => {
  const api = plannerFixture(); await sync(api);
  const task = api.runtime.state.tasks[0];
  assert.equal(task.external.provider, "Planner"); assert.equal(task.external.id, "planner-one");
  assert.equal(task.assigned_to, "me"); assert.equal(task.list_id, 1); assert.equal(task.due_date, "2026-11-23");
  assert.equal(task.steps[0].title, "Review"); assert.equal(task.linked_resources.length, 1);
  assert.equal(task.linked_resources[0].web_url, "https://example.test/document");
  assert.equal(api.writes.length, 0);
  const ids = [task.id, task.steps[0].id]; await sync(api);
  assert.deepEqual([api.runtime.state.tasks[0].id, api.runtime.state.tasks[0].steps[0].id], ids);
});

test("edits task and details with separate ETags while retaining partial progress and due-time precision", async () => {
  const api = plannerFixture(); await sync(api);
  api.runtime.mutate(state => { const task = state.tasks[0]; task.title = "Local rename"; task.notes = "Local note"; task.steps[0].completed = true; task.my_day = "2026-11-22"; task.important = true; });
  await sync(api);
  assert.equal(api.tasks[0].percentComplete, 50); assert.equal(api.tasks[0].dueDateTime, "2026-11-23T17:30:00Z");
  assert.equal(api.tasks[0].title, "Local rename"); assert.equal(api.details.get("planner-one").description, "Local note");
  assert.equal(api.details.get("planner-one").checklist["step-one"].isChecked, true);
  assert.deepEqual(api.writes.map(write => write.etag), ['W/"task-1"', 'W/"details-1"']);
  assert.equal(api.runtime.state.tasks[0].my_day, "2026-11-22"); assert.equal(api.runtime.state.tasks[0].important, true);
});

test("merges independent remote fields and preserves local My Day", async () => {
  const api = plannerFixture(); await sync(api);
  api.runtime.mutate(state => { state.tasks[0].notes = "Local notes"; state.tasks[0].my_day = "2026-11-22"; });
  api.tasks[0].title = "Team renamed"; api.details.get("planner-one").checklist["step-one"].title = "New review";
  await sync(api);
  assert.equal(api.runtime.state.tasks[0].title, "Team renamed"); assert.equal(api.runtime.state.tasks[0].steps[0].title, "New review");
  assert.equal(api.runtime.state.tasks[0].notes, "Local notes"); assert.equal(api.runtime.state.tasks[0].my_day, "2026-11-22");
});

test("conflicting task or checklist edits stop before any remote mutation", async () => {
  for (const field of ["title", "step"]) {
    const api = plannerFixture(); await sync(api);
    api.runtime.mutate(state => { state.tasks[0].notes = "Independent local change";
      if (field === "title") state.tasks[0].title = "Local title"; else state.tasks[0].steps[0].title = "Local step"; });
    if (field === "title") api.tasks[0].title = "Remote title"; else api.details.get("planner-one").checklist["step-one"].title = "Remote step";
    await assert.rejects(sync(api), /Sync conflict/); assert.equal(api.writes.length, 0);
    assert.equal(api.runtime.state.tasks[0].notes, "Independent local change");
  }
});

test("checklist additions recover the same client GUID after an uncertain PATCH response", async () => {
  const api = plannerFixture(); await sync(api);
  api.runtime.mutate(state => state.tasks[0].steps.push({ id: state.next_id++, title: "New step", completed: false }));
  api.afterWrite = () => { throw new Error("Response lost after write"); };
  await assert.rejects(sync(api), /Response lost/);
  const ids = Object.keys(api.details.get("planner-one").checklist);
  api.afterWrite = null; await sync(api);
  assert.deepEqual(Object.keys(api.details.get("planner-one").checklist), ids);
  assert.equal(api.runtime.state.tasks[0].steps.length, 2);
  await sync(api); assert.equal(api.writes.length, 1);
});

test("late local edits survive an applied remote write and synchronize on the next round", async () => {
  const api = plannerFixture(); await sync(api);
  api.runtime.mutate(state => state.tasks[0].title = "First edit");
  api.afterWrite = () => api.runtime.mutate(state => state.tasks[0].title = "Newer edit");
  await assert.rejects(sync(api), /changed on this device/);
  assert.equal(api.runtime.state.tasks[0].title, "Newer edit");
  api.afterWrite = null;
  await sync(api); assert.equal(api.tasks[0].title, "Newer edit");
});

test("checkpointing an acknowledged title does not discard notes whose upload was interrupted", async () => {
  const api = plannerFixture(); await sync(api);
  api.runtime.mutate(state => { state.tasks[0].title = "New title"; state.tasks[0].notes = "Pending notes"; });
  api.afterWrite = ({ path }) => { if (!path.endsWith("/details")) api.runtime.mutate(state => state.tasks[0].notes = "Newer notes"); };
  await assert.rejects(sync(api), /changed on this device/);
  assert.equal(api.details.get("planner-one").description, "Team notes");
  api.afterWrite = null; await sync(api);
  assert.equal(api.details.get("planner-one").description, "Newer notes");
});

test("an acknowledged new checklist item keeps its GUID when edited before local commit", async () => {
  const api = plannerFixture(); await sync(api);
  api.runtime.mutate(state => state.tasks[0].steps.push({ id: state.next_id++, title: "Draft step", completed: false }));
  const localId = api.runtime.state.tasks[0].steps[1].id;
  api.afterWrite = () => api.runtime.mutate(state => state.tasks[0].steps[1].title = "Edited again");
  await assert.rejects(sync(api), /changed on this device/);
  const ids = Object.keys(api.details.get("planner-one").checklist);
  api.afterWrite = null; await sync(api);
  assert.deepEqual(Object.keys(api.details.get("planner-one").checklist), ids);
  assert.equal(api.runtime.state.tasks[0].steps.find(step => step.id === localId).title, "Edited again");
});

test("unassigned tasks disappear only when their editable local fields are unchanged", async () => {
  const api = plannerFixture(); await sync(api); api.tasks.length = 0; await sync(api);
  assert.equal(api.runtime.state.tasks.length, 0); assert.equal(api.writes.length, 0);
  const other = plannerFixture(); await sync(other);
  other.runtime.mutate(state => state.tasks[0].notes = "Unsynced work"); other.tasks.length = 0;
  await assert.rejects(sync(other), /no longer assigned/); assert.equal(other.runtime.state.tasks[0].notes, "Unsynced work");
});

test("local projection removal never deletes the Planner service task", async () => {
  const api = plannerFixture(); await sync(api); api.runtime.mutate(state => state.tasks = []);
  await sync(api); assert.equal(api.tasks.length, 1); assert.equal(api.runtime.state.tasks.length, 1); assert.equal(api.writes.length, 0);
});

test("checklist deletes merge and conflict with simultaneous remote edits", async () => {
  const api = plannerFixture(); await sync(api); api.runtime.mutate(state => state.tasks[0].steps = []);
  await sync(api); assert.deepEqual(api.details.get("planner-one").checklist, {});
  const other = plannerFixture(); await sync(other); other.runtime.mutate(state => state.tasks[0].steps = []);
  other.details.get("planner-one").checklist["step-one"].title = "Changed remotely";
  await assert.rejects(sync(other), /removed step changed online/); assert.equal(other.writes.length, 0);
});

test("automatic Planner polling is bounded, while local edits and manual refresh bypass the delay", async () => {
  const api = plannerFixture(); await sync(api); const reads = api.reads;
  assert.deepEqual(await sync(api, { force: false }), { skipped: true }); assert.equal(api.reads, reads);
  api.runtime.mutate(state => state.tasks[0].title = "Immediate edit");
  assert.equal((await sync(api, { force: false })).skipped, false); assert.equal(api.tasks[0].title, "Immediate edit");
  const next = api.reads; await sync(api); assert(api.reads > next);
});

test("missing ETags, account mismatch and unavailable Planner preserve local tasks", async () => {
  const api = plannerFixture(); await sync(api); api.runtime.mutate(state => state.tasks[0].title = "Local edit");
  delete api.tasks[0]["@odata.etag"];
  await assert.rejects(sync(api), /No unconditional update/); assert.equal(api.writes.length, 0);
  await assert.rejects(synchronizePlanner(api.runtime, "other-account", "2026-11-22"), /linked to another/);
  api.collectionError = Object.assign(new Error("Forbidden"), { status: 403 });
  await assert.rejects(sync(api), error => error.code === "planner_unavailable");
  assert.equal(api.runtime.state.tasks[0].title, "Local edit");
});

test("completing, reopening and changing dates update only supported task fields", async () => {
  const api = plannerFixture(); await sync(api);
  api.runtime.mutate(state => { state.tasks[0].completed = true; state.tasks[0].due_date = "2026-12-01"; });
  await sync(api); assert.equal(api.tasks[0].percentComplete, 100); assert.equal(api.tasks[0].dueDateTime, "2026-12-01T00:00:00.000Z");
  api.runtime.mutate(state => { state.tasks[0].completed = false; state.tasks[0].due_date = null; });
  await sync(api); assert.equal(api.tasks[0].percentComplete, 0); assert.equal(api.tasks[0].dueDateTime, null);
  assert.deepEqual(Object.keys(api.writes[0].body).sort(), ["dueDateTime", "percentComplete"]);
});
