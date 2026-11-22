// Account-scoped session caching keeps duplicate large bodies out of localStorage.
export function createCloudReader(api) {
  const caches = new Map();
  const dirtyLists = new Set(), dirtyTasks = new Map(), scanLists = new Set();
  const empty = () => ({ cursor: null, lists: new Map(), tasks: new Map() });
  let activeAccount;

  async function delta(initial, previous) {
    try { return await pages(previous?.cursor || initial, previous?.items || new Map()); }
    catch (error) {
      if (!previous?.cursor || !(error.status === 410 || ["syncStateNotFound", "SyncStateNotFound", "InvalidDeltaToken", "resyncRequired"].includes(error.code))) throw error;
      return pages(initial, new Map());
    }
  }

  async function pages(path, original) {
    const items = new Map(original), changed = new Set(), visited = new Set();
    let cursor;
    while (path) {
      const canonical = api.graphUrl(path).href;
      if (visited.has(canonical) || visited.size >= 10_000) throw new Error("Microsoft returned an invalid delta continuation sequence.");
      visited.add(canonical);
      const page = await api.graph(path);
      if (!Array.isArray(page?.value)) throw new Error("Microsoft returned an invalid delta response.");
      for (const item of page.value) {
        if (typeof item.id !== "string" || !item.id) throw new Error("Microsoft returned a delta item without an identifier.");
        changed.add(item.id);
        if (item["@removed"]) items.delete(item.id);
        else items.set(item.id, { ...items.get(item.id), ...item });
      }
      path = page["@odata.nextLink"];
      cursor = page["@odata.deltaLink"] || cursor;
    }
    if (typeof cursor !== "string") throw new Error("Microsoft did not complete its delta sequence.");
    api.graphUrl(cursor);
    return { items, cursor, changed };
  }

  async function task(listId, taskId, existing) {
    const path = api.taskPath(listId, taskId);
    const value = existing || await api.graph(`${path}?$expand=checklistItems,linkedResources`);
    if (!Array.isArray(value.checklistItems) || value["checklistItems@odata.nextLink"]) value.checklistItems = await api.collection(`${path}/checklistItems`);
    if (!Array.isArray(value.linkedResources) || value["linkedResources@odata.nextLink"]) value.linkedResources = await api.collection(`${path}/linkedResources`);
    value.attachments = value.hasAttachments ? await api.collection(`${path}/attachments`) : [];
    return value;
  }

  async function list(listId) {
    const value = await api.graph(api.listPath(listId));
    value.tasks = await api.collection(`${api.listPath(listId)}/tasks?$expand=checklistItems,linkedResources`);
    for (let index = 0; index < value.tasks.length; index++) value.tasks[index] = await task(listId, value.tasks[index].id, value.tasks[index]);
    return value;
  }

  async function read(accountId, { fresh = false } = {}) {
    if (activeAccount !== accountId) {
      dirtyLists.clear(); dirtyTasks.clear(); scanLists.clear(); activeAccount = accountId;
    }
    const old = fresh ? empty() : caches.get(accountId) || empty();
    const lists = await delta("me/todo/lists/delta", old.cursor ? { cursor: old.cursor, items: old.lists } : null);
    for (const id of dirtyLists) {
      try { lists.items.set(id, await api.graph(api.listPath(id))); }
      catch (error) { if (error.status === 404) lists.items.delete(id); else throw error; }
    }
    const next = { cursor: lists.cursor, lists: lists.items, tasks: new Map() };
    for (const [id] of next.lists) {
      const initial = `${api.listPath(id)}/tasks/delta`;
      const previous = scanLists.has(id) ? null : old.tasks.get(id);
      const tasks = await delta(initial, previous);
      for (const changed of dirtyTasks.get(id) || []) tasks.changed.add(changed);
      for (const taskId of tasks.changed) {
        if (!tasks.items.has(taskId) && !dirtyTasks.get(id)?.has(taskId)) continue;
        try { tasks.items.set(taskId, await task(id, taskId)); }
        catch (error) { if (error.status === 404) tasks.items.delete(taskId); else throw error; }
      }
      next.tasks.set(id, { cursor: tasks.cursor, items: tasks.items });
    }
    // A failed page never advances a token or publishes a partial cache.
    caches.set(accountId, next);
    dirtyLists.clear(); dirtyTasks.clear(); scanLists.clear();
    return structuredClone([...next.lists].map(([id, list]) => ({ ...list, tasks: [...next.tasks.get(id).items.values()] })));
  }

  function invalidate(path, response) {
    let parts = api.graphUrl(path).pathname.split("/").slice(2).map(decodeURIComponent);
    if (parts[0] === "users" && parts[1]) parts = ["me", ...parts.slice(2)];
    if (parts[0] !== "me" || parts[1] !== "todo" || parts[2] !== "lists") return;
    const listId = parts[3] || response?.id;
    if (!listId) { caches.clear(); return; }
    if (parts.length <= 4) dirtyLists.add(listId);
    if (parts[4] === "tasks") {
      const taskId = parts[5] || response?.id;
      if (!taskId) scanLists.add(listId);
      else {
        if (!dirtyTasks.has(listId)) dirtyTasks.set(listId, new Set());
        dirtyTasks.get(listId).add(taskId);
      }
    }
  }

  return { read, task, list, invalidate, clear() { caches.clear(); dirtyLists.clear(); dirtyTasks.clear(); scanLists.clear(); activeAccount = undefined; } };
}
