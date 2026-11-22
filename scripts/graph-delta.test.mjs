import test from "node:test";
import assert from "node:assert/strict";
import { createCloudReader } from "../crates/todo-browser/wwwroot/cloud/delta.js";

function fixture() {
  const requests = [], routes = new Map();
  const graphUrl = path => {
    const url = new URL(path, "https://graph.microsoft.com/v1.0/");
    if (url.origin !== "https://graph.microsoft.com" || !url.pathname.startsWith("/v1.0/")) throw new Error("Unexpected continuation");
    return url;
  };
  const key = path => graphUrl(path).href.replace("https://graph.microsoft.com/v1.0/", "");
  const api = {
    graphUrl, listPath: id => `me/todo/lists/${id}`, taskPath: (list, id) => `me/todo/lists/${list}/tasks/${id}`,
    async graph(path) {
      path = key(path); requests.push(path);
      const route = routes.get(path);
      if (!route) throw new Error(`Unexpected request ${path}`);
      return structuredClone(typeof route === "function" ? await route() : route);
    },
    async collection(path) { return (await api.graph(path)).value; },
  };
  const task = (id, title) => ({ id, title, checklistItems: [], linkedResources: [], hasAttachments: false });
  const page = (value, cursor) => ({ value, "@odata.deltaLink": cursor });
  routes.set("me/todo/lists/delta", page([{ id: "list", displayName: "Tasks" }], "me/todo/lists/delta?$deltatoken=L1"));
  routes.set("me/todo/lists/list/tasks/delta", page([{ id: "task", title: "Original" }], "me/todo/lists/list/tasks/delta?$deltatoken=T1"));
  routes.set("me/todo/lists/list/tasks/task?$expand=checklistItems,linkedResources", task("task", "Original"));
  routes.set("me/todo/lists/delta?$deltatoken=L1", page([], "me/todo/lists/delta?$deltatoken=L1"));
  routes.set("me/todo/lists/list/tasks/delta?$deltatoken=T1", page([], "me/todo/lists/list/tasks/delta?$deltatoken=T1"));
  return { reader: createCloudReader(api), api, requests, routes, task, page };
}

test("unchanged delta polling requests only list/task cursors and isolates returned snapshots", async () => {
  const f = fixture(); const initial = await f.reader.read("one");
  initial[0].tasks[0].title = "Mutated by caller";
  f.requests.length = 0;
  const next = await f.reader.read("one");
  assert.equal(next[0].tasks[0].title, "Original");
  assert.deepEqual(f.requests, ["me/todo/lists/delta?$deltatoken=L1", "me/todo/lists/list/tasks/delta?$deltatoken=T1"]);
});

test("delta additions, changes, and tombstones hydrate only remaining changed tasks", async () => {
  const f = fixture(); await f.reader.read("one");
  f.routes.set("me/todo/lists/list/tasks/delta?$deltatoken=T1", f.page([{ id: "task", "@removed": { reason: "deleted" } }, { id: "new" }], "me/todo/lists/list/tasks/delta?$deltatoken=T2"));
  f.routes.set("me/todo/lists/list/tasks/new?$expand=checklistItems,linkedResources", f.task("new", "New task"));
  const next = await f.reader.read("one");
  assert.deepEqual(next[0].tasks.map(t => [t.id, t.title]), [["new", "New task"]]);
});

test("expired tokens restart the affected collection without retaining deleted cache entries", async () => {
  const f = fixture(); await f.reader.read("one");
  f.routes.set("me/todo/lists/list/tasks/delta?$deltatoken=T1", () => { throw Object.assign(new Error("Expired"), { status: 410 }); });
  f.routes.set("me/todo/lists/list/tasks/delta", f.page([], "me/todo/lists/list/tasks/delta?$deltatoken=T2"));
  assert.deepEqual((await f.reader.read("one"))[0].tasks, []);
});

test("failed delta pages do not advance a saved cursor or expose a partial cache", async () => {
  const f = fixture(); await f.reader.read("one");
  f.routes.set("me/todo/lists/list/tasks/delta?$deltatoken=T1", { value: [{ id: "task", "@removed": {} }], "@odata.nextLink": "me/todo/lists/list/tasks/delta?$skiptoken=page2" });
  f.routes.set("me/todo/lists/list/tasks/delta?$skiptoken=page2", () => { throw new Error("Network failed"); });
  await assert.rejects(f.reader.read("one"), /Network failed/);
  f.routes.set("me/todo/lists/list/tasks/delta?$deltatoken=T1", f.page([], "me/todo/lists/list/tasks/delta?$deltatoken=T1"));
  assert.equal((await f.reader.read("one"))[0].tasks[0].title, "Original");
});

test("own task writes are reread directly even when the delta feed has not caught up", async () => {
  const f = fixture(); await f.reader.read("one");
  f.reader.invalidate("me/todo/lists/list/tasks/new", { id: "new" });
  f.routes.set("me/todo/lists/list/tasks/new?$expand=checklistItems,linkedResources", f.task("new", "Written now"));
  assert.equal((await f.reader.read("one"))[0].tasks.find(t => t.id === "new").title, "Written now");
});

test("a new account or explicit fresh archive starts from initial delta endpoints", async () => {
  const f = fixture(); await f.reader.read("one"); f.requests.length = 0;
  await f.reader.read("two"); assert.equal(f.requests[0], "me/todo/lists/delta");
  f.requests.length = 0;
  await f.reader.read("two", { fresh: true }); assert.equal(f.requests[0], "me/todo/lists/delta");
});

test("repeated and foreign delta continuation URLs fail before requesting them", async () => {
  for (const next of ["me/todo/lists/delta", "https://attacker.test/steal"]) {
    const f = fixture(); f.routes.set("me/todo/lists/delta", { value: [], "@odata.nextLink": next });
    await assert.rejects(f.reader.read("one"), /continuation/i);
    assert.equal(f.requests.length, 1);
  }
});
