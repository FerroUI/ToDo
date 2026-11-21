export class Storage {
  data = new Map();
  getItem(key) { return this.data.get(key) ?? null; }
  setItem(key, value) { this.data.set(key, String(value)); }
  removeItem(key) { this.data.delete(key); }
}

export function task(id = 2, title = "A task", listId = 1) {
  return { id, list_id: listId, title, notes: "", important: false, completed: false, due_date: null,
    reminder: null, recurrence: null, my_day: null, my_day_excluded: null, recurrence_generated: false,
    steps: [], attachments: [], tags: [], assigned_to: null, flagged_email: false,
    created_on: "2026-11-21", updated_on: "2026-11-21", completed_on: null, position: id };
}

export function remoteTask(id = "remote-task", title = "Cloud task") {
  return { id, title, body: { contentType: "text", content: "" }, importance: "normal", status: "notStarted",
    dueDateTime: null, isReminderOn: false, reminderDateTime: null, recurrence: null, categories: [],
    checklistItems: [], linkedResources: [], "@odata.etag": 'W/"1"', createdDateTime: "2026-11-21T10:00:00Z" };
}

export function fixture(tasks = []) {
  globalThis.localStorage = new Storage();
  globalThis.location = { origin: "https://example.test", pathname: "/ToDo/" };
  globalThis.DOMParser = class { parseFromString(html) { return { body: { textContent: html.replaceAll("<br>", "\n") }, querySelectorAll: () => [] }; } };
  let state = { schema_version: 1, tasks, lists: [{ id: 1, name: "Tasks", color: "#3E69E4", background: null, group_id: null, position: 1, sort: "Manual", show_completed: true }], groups: [], settings: {}, next_id: Math.max(2, ...tasks.map(task => task.id + 1), ...tasks.flatMap(task => task.steps.map(step => step.id + 1))) };
  const persist = () => localStorage.setItem("ferroui.todo.v1", JSON.stringify(state));
  const runtime = {
    exportData: () => JSON.stringify(state),
    replaceFromCloud(json, expected) {
      if (JSON.stringify(state) !== expected) throw new Error("Local compare-and-swap failed");
      state = JSON.parse(json); persist();
    },
    get state() { return state; },
    mutate(action) { action(state); persist(); },
  };
  persist();
  let next = 1;
  const cloud = [{ id: "default", displayName: "Tasks", wellknownListName: "defaultList", isOwner: true, tasks: [] }];
  const writes = [];
  let afterWrite;
  const api = {
    cloud, writes, runtime,
    set afterWrite(value) { afterWrite = value; },
    readCloud: async () => structuredClone(cloud),
    collection: async path => structuredClone(await api.graph(path)).value,
    async graph(path, method = "GET", body, etag) {
      const pieces = path.split("?")[0].split("/").map(decodeURIComponent);
      const list = cloud.find(item => item.id === pieces[3]);
      const item = list?.tasks.find(task => task.id === pieces[5]);
      const steps = item?.checklistItems;
      let result;
      if (method !== "GET") writes.push({ path, method, body: structuredClone(body) });
      if (method === "GET" && pieces.length === 6) result = item;
      else if (method === "GET" && pieces.length === 7) result = { value: steps };
      else if (method === "POST" && pieces.length === 3) {
        result = { id: `list-${next++}`, displayName: body.displayName, wellknownListName: "none", isOwner: true, tasks: [] }; cloud.push(result);
      } else if (method === "PATCH" && pieces.length === 4) Object.assign(list, body), result = list;
      else if (method === "DELETE" && pieces.length === 4) cloud.splice(cloud.indexOf(list), 1), result = null;
      else if (method === "POST" && pieces.length === 5) {
        result = { ...remoteTask(`task-${next++}`, body.title), ...body }; list.tasks.push(result);
      } else if (method === "PATCH" && pieces.length === 6) Object.assign(item, body), result = item;
      else if (method === "DELETE" && pieces.length === 6) list.tasks = list.tasks.filter(task => task.id !== pieces[5]), result = null;
      else if (method === "POST" && pieces.length === 7) { result = { id: `step-${next++}`, ...body }; steps.push(result); }
      else if (method === "PATCH" && pieces.length === 8) { result = steps.find(step => step.id === pieces[7]); Object.assign(result, body); }
      else if (method === "DELETE" && pieces.length === 8) item.checklistItems = steps.filter(step => step.id !== pieces[7]), result = null;
      else throw new Error(`Unexpected fixture Graph call: ${method} ${path}`);
      if (method !== "GET" && afterWrite) await afterWrite({ path, method, body });
      return structuredClone(result);
    },
  };
  globalThis.graphFixture = api;
  return api;
}
