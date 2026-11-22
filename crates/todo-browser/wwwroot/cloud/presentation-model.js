import { SETTING_FIELDS, LIST_FIELDS, TASK_FIELDS, GROUP_FIELDS, fieldKey, parseField, same, validDevice, mergeRecords } from "./presentation-format.js";

const pick = (object, fields) => fields.filter(field => object[field] !== undefined).map(field => [field, object[field]]);
const identity = task => JSON.stringify(task);

export function presentationIdentities(state, graph) {
  const lists = new Map(state.lists.filter(list => graph.lists[list.id]?.remoteId).map(list => [list.id, graph.lists[list.id].remoteId]));
  const tasks = new Map();
  for (const task of state.tasks) {
    const record = graph.tasks[task.id];
    if (task.external?.provider === "Planner") tasks.set(task.id, identity(["planner", task.external.id]));
    else if (record?.remoteId && record.listId) tasks.set(task.id, identity(["todo", record.listId, record.remoteId]));
  }
  return { lists, tasks };
}

export function assignGroupIdentities(state, saved = {}, uuid = () => crypto.randomUUID()) {
  const groups = { ...saved };
  if (Object.values(groups).some(value => !validDevice(value)) || new Set(Object.values(groups)).size !== Object.keys(groups).length) {
    throw new Error("The local device settings group mappings are invalid.");
  }
  for (const group of state.groups) groups[group.id] ||= uuid();
  return groups;
}

export function projectPresentation(state, graph, groups) {
  const result = {}, { lists, tasks } = presentationIdentities(state, graph);
  for (const [field, value] of pick(state.settings, SETTING_FIELDS)) result[fieldKey("settings", "app", field)] = value;
  for (const group of state.groups) {
    const id = groups[group.id];
    result[fieldKey("group", id, "exists")] = true;
    for (const [field, value] of pick(group, GROUP_FIELDS)) result[fieldKey("group", id, field)] = value;
  }
  for (const list of state.lists) {
    const id = lists.get(list.id);
    if (!id) continue;
    for (const [field, value] of pick(list, LIST_FIELDS)) result[fieldKey("list", id, field)] = value;
    result[fieldKey("list", id, "group")] = list.id === 1 ? null : groups[list.group_id] || null;
  }
  for (const task of state.tasks) {
    const id = tasks.get(task.id);
    if (!id) continue;
    for (const [field, value] of pick(task, TASK_FIELDS)) result[fieldKey("task", id, field)] = value;
    if (task.external?.provider === "Planner") result[fieldKey("task", id, "important")] = !!task.important;
  }
  result[fieldKey("sidebar", "root", "order")] = (state.sidebar_order || []).flatMap(id =>
    groups[id] && state.groups.some(group => group.id === id) ? [["group", groups[id]]]
      : id !== 1 && lists.has(id) ? [["list", lists.get(id)]] : []);
  return result;
}

// Each field is a Lamport last-writer register. Independent changes merge;
// concurrent changes to one field converge by (counter, stable device UUID).
export function recordLocalChanges(local, baseline, remote, device) {
  const updates = [];
  for (const [key, value] of Object.entries(local)) {
    if (Object.hasOwn(baseline, key) ? !same(value, baseline[key]) : !Object.hasOwn(remote, key)) updates.push([key, false, value]);
  }
  for (const key of Object.keys(baseline)) if (!Object.hasOwn(local, key)) updates.push([key, true]);
  if (!updates.length) return { records: remote, changed: false };
  const counter = Object.values(remote).reduce((maximum, record) => Math.max(maximum, record.clock[0]), 0) + 1;
  if (!Number.isSafeInteger(counter) || counter >= Number.MAX_SAFE_INTEGER) throw new Error("Device settings have exhausted their revision range.");
  const changes = Object.fromEntries(updates.map(([key, deleted, value]) => [key, {
    clock: [counter, device], deleted, ...(!deleted ? { value } : {}),
  }]));
  return { records: mergeRecords(remote, changes), changed: true };
}

export function applyPresentation(state, graph, savedGroups, records) {
  const draft = structuredClone(state), groups = { ...savedGroups };
  const groupToLocal = new Map(Object.entries(groups).map(([local, id]) => [id, Number(local)]));
  const active = new Set();
  for (const [key, record] of Object.entries(records)) {
    const [kind, id, field] = parseField(key);
    if (kind !== "group" || field !== "exists") continue;
    if (record.deleted || record.value !== true) continue;
    active.add(id);
    if (draft.groups.some(group => group.id === groupToLocal.get(id))) continue;
    const name = records[fieldKey("group", id, "name")];
    if (!name || name.deleted) throw new Error("OneDrive contains an incomplete list group.");
    const local = draft.next_id++;
    if (!Number.isSafeInteger(local) || local < 2) throw new Error("Cannot allocate a device settings group identity.");
    draft.groups.push({ id: local, name: name.value, collapsed: false, position: local });
    groups[local] = id; groupToLocal.set(id, local);
  }
  draft.groups = draft.groups.filter(group => !groups[group.id] || active.has(groups[group.id]));
  const liveGroups = new Set(draft.groups.map(group => group.id));
  for (const list of draft.lists) if (!liveGroups.has(list.group_id)) list.group_id = null;
  const identities = presentationIdentities(draft, graph);
  const lists = new Map([...identities.lists].map(([local, id]) => [id, draft.lists.find(list => list.id === local)]));
  const tasks = new Map([...identities.tasks].map(([local, id]) => [id, draft.tasks.find(task => task.id === local)]));
  const groupObjects = new Map(draft.groups.map(group => [groups[group.id], group]));
  for (const [key, record] of Object.entries(records)) {
    if (record.deleted) continue;
    const [kind, id, field] = parseField(key), value = structuredClone(record.value);
    if (kind === "settings") draft.settings[field] = value;
    else if (kind === "group" && field !== "exists" && groupObjects.has(id)) groupObjects.get(id)[field] = value;
    else if (kind === "list" && lists.has(id)) {
      const list = lists.get(id);
      if (field === "group") list.group_id = list.id === 1 ? null : active.has(value) ? groupToLocal.get(value) ?? null : null;
      else list[field] = value;
    } else if (kind === "task" && tasks.has(id)) {
      const task = tasks.get(id);
      if (field !== "important" || task.external?.provider === "Planner") task[field] = value;
    } else if (kind === "sidebar") {
      const order = value.flatMap(([type, key]) => {
        const local = type === "group" ? groupToLocal.get(key) : lists.get(key)?.id;
        return local && (type !== "group" || liveGroups.has(local)) ? [local] : [];
      });
      // Keep unsynchronized local-only list entries in their own relative order.
      draft.sidebar_order = [...new Set([...order, ...(draft.sidebar_order || []).filter(local => !identities.lists.has(local) && !groups[local])])];
    }
  }
  return { draft, groups };
}
